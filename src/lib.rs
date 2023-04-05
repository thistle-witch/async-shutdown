#![no_std]

use core::{future::Future, sync::atomic::Ordering, task::Poll};

use spawn::Spawn;
use state::State;

mod spawn;
mod state;

/// Future which completes once the associated task group has signaled a shutdown.
pub struct ShutdownSignal;

/// A group of potentially related tasks. Tasks spawned by this struct can be waited on or signaled to shut down.
pub struct TaskGroup<S: Spawn> {
    spawner: S,
    state: &'static State,
}

impl<S: Spawn> TaskGroup<S> {
    /// Create a new task group using the provided spawner
    pub fn with_spawner(spawner: S, state: &'static State) -> Self {
        TaskGroup { spawner, state }
    }

    /// Signal a shutdown to all tasks in this group and wait for shutdown to finish.
    pub async fn shutdown() {
        todo!()
    }

    /// Wait for all tasks in this group to finish without explicitly sending a shutdown signal.
    pub fn done(&self) -> DoneFuture {
        DoneFuture { state: self.state }
    }

    /// Spawn a task as part of this task group
    pub fn spawn(&self, future: impl Future<Output = ()> + Send + 'static) {
        let task = Task::new(self.state);
        self.spawner.spawn(async {
            future.await;
            core::mem::drop(task);
        });
    }

    pub fn spawn_with_shutdown<F>(&self, f: impl FnOnce(ShutdownSignal) -> F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let future = f(ShutdownSignal);
        self.spawn(future);
    }
}

pub struct DoneFuture {
    state: &'static State,
}

impl Future for DoneFuture {
    type Output = ();

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        self.state.done_waker.register(cx.waker());

        if self.state.running_tasks.load(Ordering::SeqCst) == 0 {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

pub struct Task {
    state: &'static State,
}

impl Task {
    pub fn new(state: &'static State) -> Self {
        let running_tasks = state.running_tasks.fetch_add(1, Ordering::SeqCst);

        if running_tasks == usize::MAX {
            panic!();
        }

        Task { state }
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        let running_tasks = self.state.running_tasks.fetch_sub(1, Ordering::SeqCst);

        if running_tasks == 1 {
            self.state.done_waker.wake();
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn spawns_tasks() {
        static STATE: State = State::new();

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let group = TaskGroup::with_spawner(&runtime, &STATE);
        let (tx, rx) = tokio::sync::oneshot::channel();

        runtime.block_on(async move {
            group.spawn(async move {
                if let Err(_) = tx.send(()) {
                    panic!("the receiver dropped");
                }
            });

            tokio::select! {
                result = rx => match result {
                    Ok(_) => {}
                    Err(_) => panic!("the sender did not spawn"),
                },
                _ = tokio::time::sleep(core::time::Duration::from_secs(10)) => panic!()
            }
        });
    }

    #[test]
    fn done_waits() {
        static STATE: State = State::new();

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let group = TaskGroup::with_spawner(&runtime, &STATE);

        runtime.block_on(async move {
            group.spawn(async move {
                loop {
                    tokio::time::sleep(core::time::Duration::from_millis(100)).await;
                }
            });

            tokio::select! {
                _ = group.done() => panic!(),
                _ = tokio::time::sleep(core::time::Duration::from_secs(2)) => {}
            }
        });
    }

    #[test]
    fn done_exits() {
        static STATE: State = State::new();

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let group = TaskGroup::with_spawner(&runtime, &STATE);

        runtime.block_on(async move {
            for _ in 0..5 {
                group.spawn(async move {
                    tokio::time::sleep(core::time::Duration::from_millis(100)).await;
                });
            }

            tokio::select! {
                _ = group.done() => {},
                _ = tokio::time::sleep(core::time::Duration::from_secs(2)) => panic!()
            }
        });
    }
}
