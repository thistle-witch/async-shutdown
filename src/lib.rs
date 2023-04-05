#![no_std]

use core::future::Future;

use spawn::Spawn;

mod spawn;

/// Future which completes once the associated task group has signaled a shutdown.
pub struct ShutdownSignal;

/// A group of potentially related tasks. Tasks spawned by this struct can be waited on or signaled to shut down.
pub struct TaskGroup<S: Spawn> {
    spawner: S,   
}

impl<S: Spawn> TaskGroup<S> {
    /// Create a new task group using the provided spawner
    pub fn with_spawner(spawner: S) -> Self {
        TaskGroup { spawner }
    }
    /// Signal a shutdown to all tasks in this group and wait for shutdown to finish.
    pub async fn shutdown() {
        todo!()
    }

    /// Wait for all tasks in this group to finish without explicitly sending a shutdown signal.
    pub async fn done() {
        todo!()
    }

    /// Spawn a task as part of this task group
    pub fn spawn(&self, future: impl Future<Output = ()> + Send + 'static) {
        self.spawner.spawn(future);
    }

    pub fn spawn_with_shutdown<F>(&self, f: impl FnOnce(ShutdownSignal) -> F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let future = f(ShutdownSignal);
        self.spawn(future);
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn spawns_tasks() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let group = TaskGroup::with_spawner(&runtime);
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
}
