use core::future::Future;

pub trait Spawn {
    fn spawn(&self, future: impl Future<Output = ()> + Send + 'static);
}

#[cfg(feature = "tokio")]
impl Spawn for &tokio::runtime::Runtime {
    fn spawn(&self, future: impl Future<Output = ()> + Send + 'static)
    {
        tokio::runtime::Runtime::spawn(self, future);
    }
}