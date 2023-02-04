use eframe::egui::Context;
use poll_promise::Promise;

pub trait ContextExt {
    fn spawn_async<T>(
        &self,
        future: impl std::future::Future<Output = T> + 'static + Send,
    ) -> Promise<T>
    where
        T: Send + 'static;
}

impl ContextExt for Context {
    fn spawn_async<T>(
        &self,
        future: impl std::future::Future<Output = T> + 'static + Send,
    ) -> Promise<T>
    where
        T: Send + 'static,
    {
        let ctx = self.clone();
        Promise::spawn_async(async move {
            let result = future.await;
            ctx.request_repaint();
            result
        })
    }
}
