#![no_std]

mod group;
mod spawn;
pub(crate) mod task;

use core::sync::atomic::AtomicUsize;

use futures_util::task::AtomicWaker;
pub use group::TaskGroup;

/// Future which completes once the associated task group has signaled a shutdown.
pub struct ShutdownSignal;

pub struct State {
    running_tasks: AtomicUsize,
    done_waker: AtomicWaker,
}

impl State {
    pub const fn new() -> Self {
        State {
            running_tasks: AtomicUsize::new(0),
            done_waker: AtomicWaker::new(),
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
