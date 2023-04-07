use core::{ops::Deref, sync::atomic::Ordering};

use crate::State;

pub struct Task<C: Deref<Target = State>> {
    state: C,
}

impl<C: Deref<Target = State>> Task<C> {
    pub fn new(state: C) -> Self {
        let running_tasks = state.running_tasks.fetch_add(1, Ordering::SeqCst);

        if running_tasks == usize::MAX {
            panic!();
        }

        Task { state }
    }
}

impl<C: Deref<Target = State>> Drop for Task<C> {
    fn drop(&mut self) {
        let running_tasks = self.state.running_tasks.fetch_sub(1, Ordering::SeqCst);

        if running_tasks == 1 {
            self.state.done_waker.wake();
        }
    }
}
