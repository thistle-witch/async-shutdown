use core::{future::Future, ops::Deref, sync::atomic::Ordering};

use pin_project_lite::pin_project;

use crate::State;

pin_project! {
    pub struct Task<C: Deref<Target = State>, F> {
        state: C,
        #[pin]
        inner: F,
    }

    impl<C: Deref<Target = State>, F> PinnedDrop for Task<C, F> {
        fn drop(this: Pin<&mut Self>) {
            let running_tasks = this.state.running_tasks.fetch_sub(1, Ordering::SeqCst);

            if running_tasks == 1 {
                this.state.done_waker.wake();
            }
        }
    }
}

impl<C: Deref<Target = State>, F> Task<C, F> {
    pub fn new(state: C, future: F) -> Self {
        let running_tasks = state.running_tasks.fetch_add(1, Ordering::SeqCst);

        if running_tasks == usize::MAX {
            panic!();
        }

        Task {
            state,
            inner: future,
        }
    }
}

impl<C: Deref<Target = State>, F: Future> Future for Task<C, F> {
    type Output = F::Output;

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        let this = self.project();
        this.inner.poll(cx)
    }
}
