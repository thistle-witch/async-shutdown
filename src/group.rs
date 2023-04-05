use core::{future::Future, sync::atomic::Ordering, task::Poll};

use crate::{spawn::Spawn, state::State, ShutdownSignal, task::Task};

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
