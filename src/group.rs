use core::{
    future::Future,
    ops::Deref,
    pin::Pin,
    sync::atomic::Ordering,
    task::{Poll, Waker},
};

#[cfg(feature = "alloc")]
use alloc::sync::Arc;

use pin_project_lite::pin_project;

use crate::{intrusive::Node, spawn::Spawn, task::Task, State};

/// A group of potentially related tasks. Tasks spawned by this struct can be waited on or signaled to shut down.
pub struct TaskGroup<S: Spawn, C: Deref<Target = State>> {
    spawner: S,
    state: C,
}

#[cfg(feature = "alloc")]
impl<S: Spawn> TaskGroup<S, Arc<State>> {
    /// Create a new task group using the provided spawner
    pub fn new(spawner: S) -> Self {
        TaskGroup {
            spawner,
            state: Arc::new(State::new()),
        }
    }
}

impl<S: Spawn> TaskGroup<S, &'static State> {
    /// Create a new task group using the provided spawner and state
    pub fn with_static(spawner: S, state: &'static State) -> Self {
        TaskGroup { spawner, state }
    }
}

impl<S: Spawn, C: 'static + Deref<Target = State> + Clone + Send> TaskGroup<S, C> {
    /// Signal a shutdown to all tasks in this group and wait for shutdown to finish.
    pub async fn shutdown(&self) {
        critical_section::with(|cs| {
            self.state.shutdown_signaled.store(true, Ordering::SeqCst);

            let list = self.state.shutdown_wakers.borrow(cs).borrow_mut();

            let mut node = list.peek_front();
            while let Some(inner_node) = node {
                if let Some(ref waker) = inner_node.data {
                    waker.clone().wake();
                }

                node = inner_node.next();
            }
        });

        self.done().await;
    }

    /// Wait for all tasks in this group to finish without explicitly sending a shutdown signal.
    pub async fn done(&self) {
        DoneFuture {
            state: self.state.clone(),
        }
        .await
    }

    /// Spawn a task as part of this task group
    pub fn spawn(&self, future: impl Future<Output = ()> + Send + 'static) {
        let task = Task::new(self.state.clone());
        self.spawner.spawn(async {
            future.await;
            core::mem::drop(task);
        });
    }

    pub fn spawn_with_shutdown<F>(&self, f: impl FnOnce(ShutdownSignal<C>) -> F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let signal = ShutdownSignal {
            state: self.state.clone(),
            node: Node::new(None),
        };
        let future = f(signal);
        self.spawn(future);
    }
}

struct DoneFuture<C> {
    state: C,
}

impl<C: Deref<Target = State>> Future for DoneFuture<C> {
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

pin_project! {
    /// Future which completes once the associated task group has signaled a shutdown.
    pub struct ShutdownSignal<C: Deref<Target = State>> {
        state: C,
        #[pin]
        node: Node<Option<Waker>>,
    }

    impl<C: Deref<Target = State>> PinnedDrop for ShutdownSignal<C> {
        fn drop(this: Pin<&mut Self>) {
            let this = this.project();

            critical_section::with(|cs| {
                let mut list = this.state.shutdown_wakers.borrow(cs).borrow_mut();
                if this.node.is_init() {
                    unsafe {this.node.remove(&mut list) };
                }
            });
        }
    }
}

impl<C: Deref<Target = State>> Future for ShutdownSignal<C> {
    type Output = ();

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<Self::Output> {
        let mut this = self.project();
        unsafe {
            critical_section::with(|cs| {
                if this.state.shutdown_signaled.load(Ordering::SeqCst) {
                    return Poll::Ready(());
                }
                let node = Pin::as_mut(&mut this.node).get_unchecked_mut();
                node.data = Some(cx.waker().clone());
                if !node.is_init() {
                    this.state
                        .shutdown_wakers
                        .borrow(cs)
                        .borrow_mut()
                        .push_front(this.node);
                }
                return Poll::Pending;
            })
        }
    }
}
