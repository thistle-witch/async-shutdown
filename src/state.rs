use core::sync::atomic::AtomicUsize;

use futures_util::task::AtomicWaker;

pub struct State {
    pub(crate) running_tasks: AtomicUsize,
    pub(crate) done_waker: AtomicWaker,
}

impl State {
    pub const fn new() -> Self {
        State {
            running_tasks: AtomicUsize::new(0),
            done_waker: AtomicWaker::new(),
        }
    }
}
