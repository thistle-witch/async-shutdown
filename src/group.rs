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

use crate::{intrusive::Node, task::Task, State};

/// A group of potentially related tasks. Tasks created by this struct can be waited on or signaled to shut down.
pub struct TaskGroup<S: Deref<Target = State>> {
    state: S,
}

#[cfg(feature = "alloc")]
impl TaskGroup<Arc<State>> {
    /// Create a new task group using the provided spawner
    pub fn new() -> Self {
        TaskGroup {
            state: Arc::new(State::new()),
        }
    }
}

impl TaskGroup<&'static State> {
    /// Create a new task group using the provided spawner and state
    pub fn with_static(state: &'static State) -> Self {
        TaskGroup { state }
    }
}

impl<S: 'static + Deref<Target = State> + Clone + Send> TaskGroup<S> {
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

    /// Create a task as part of this task group. The returned Task should be spawned or awaited.
    pub fn create<F>(&self, future: F) -> Task<S, F>
    where
        F: Future,
    {
        let task = Task::new(self.state.clone(), future);

        task
    }

    pub fn create_with_shutdown<F>(&self, f: impl FnOnce(ShutdownSignal<S>) -> F) -> Task<S, F>
    where
        F: Future,
    {
        let signal = ShutdownSignal {
            state: self.state.clone(),
            node: Node::new(None),
        };
        let future = f(signal);
        self.create(future)
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
