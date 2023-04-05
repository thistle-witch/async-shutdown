#![no_std]

use core::sync::atomic::Ordering;

use state::State;

mod group;
mod spawn;
mod state;

pub use group::TaskGroup;

/// Future which completes once the associated task group has signaled a shutdown.
pub struct ShutdownSignal;

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
