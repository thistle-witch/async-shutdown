#![no_std]

use core::future::Future;

/// Future which completes once the associated task group has signaled a shutdown.
pub struct ShutdownSignal;

/// A group of potentially related tasks. Tasks spawned by this struct can be waited on or signaled to shut down.
pub struct TaskGroup;

impl TaskGroup {
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
        let _ = future;
        todo!()
    }

    pub fn spawn_with_shutdown<F>(&self, f: impl FnOnce(ShutdownSignal) -> F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let _ = f;
        todo!()
    }
}
