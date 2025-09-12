use std::{sync::mpsc, thread};
use tokio::sync::oneshot;

/// A dedicated Tokio runtime (with N worker threads) for one endpoint.
pub struct EndpointPool<Req, Res> {
    tx: mpsc::Sender<(Req, oneshot::Sender<Res>)>,
}

impl<Req: Send + 'static, Res: Send + 'static> EndpointPool<Req, Res> {
    pub fn start<F, Fut>(name: &'static str, threads: usize, handler: F) -> Self
    where
        F: Fn(Req) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Res> + Send + 'static,
    {
        let (tx, rx) = mpsc::channel::<(Req, oneshot::Sender<Res>)>();
        let handler = std::sync::Arc::new(handler);

        thread::Builder::new()
            .name(name.to_string())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(threads.max(1))
                    // 👇 add `move` so the closure owns `name`
                    .thread_name_fn(move || {
                        static ID: std::sync::atomic::AtomicUsize =
                            std::sync::atomic::AtomicUsize::new(0);
                        let n = ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        format!("{}-pool-{}", name, n)
                    })
                    .enable_all()
                    .build()
                    .expect("build endpoint runtime");

                rt.block_on(async move {
                    while let Ok((req, resp_tx)) = rx.recv() {
                        let h = handler.clone();
                        let jh = tokio::spawn(async move { h(req).await });
                        match jh.await {
                            Ok(res) => { let _ = resp_tx.send(res); }
                            Err(e) => eprintln!("[{name}] task join error: {e}"),
                        }
                    }
                });
            })
            .expect("spawn endpoint pool");

        Self { tx }
    }

    pub async fn run(&self, req: Req) -> Res {
        let (tx, rx) = oneshot::channel();
        self.tx.send((req, tx)).expect("endpoint pool offline");
        rx.await.expect("endpoint pool dropped responder")
    }
}

pub fn threads_from_env(key: &str, default_n: usize) -> usize {
    std::env::var(key).ok().and_then(|s| s.parse().ok()).unwrap_or(default_n)
}
