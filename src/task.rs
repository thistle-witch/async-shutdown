use core::sync::atomic::Ordering;

use crate::state::State;

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