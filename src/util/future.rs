use futures::Future;
use std::time::Duration;

pub struct Defer(tokio::task::JoinHandle<()>);

pub fn defer<Fut>(duration: Duration, task: Fut) -> Defer
    where Fut: Future<Output = ()> + Send + 'static
{
    Defer(tokio::spawn(async move {
        tokio::time::sleep(duration).await;
        task.await
    }))
}

impl Drop for Defer {
    fn drop(&mut self) {
        self.0.abort()
    }
}
