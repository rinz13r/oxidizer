use std::future::Future;
use std::pin::Pin;

use oxidizer::Runtime;

const MAX_WORKER_THREADS: usize = 8;

pub struct TokioRuntime(tokio::runtime::Runtime);

impl std::ops::Deref for TokioRuntime {
    type Target = tokio::runtime::Runtime;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Runtime for TokioRuntime {
    fn spawn(&self, fut: Pin<Box<dyn Future<Output = ()> + Send + 'static>>) {
        self.0.spawn(fut);
    }
}

pub struct SmolRuntime(smol::Executor<'static>);

impl std::ops::Deref for SmolRuntime {
    type Target = smol::Executor<'static>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Runtime for SmolRuntime {
    fn spawn(&self, fut: Pin<Box<dyn Future<Output = ()> + Send + 'static>>) {
        self.0.spawn(fut).detach();
    }
}

lazy_static::lazy_static! {
    pub static ref RT: TokioRuntime = {
        TokioRuntime(tokio::runtime::Builder::new_multi_thread()
            .worker_threads(MAX_WORKER_THREADS)
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime"))
    };

    pub static ref RT2: SmolRuntime = SmolRuntime(smol::Executor::new());
}

#[ctor::ctor]
fn init_runtime() {
    lazy_static::initialize(&RT);
    lazy_static::initialize(&RT2);

    for i in 0..MAX_WORKER_THREADS {
        let ex = &*RT2;
        std::thread::Builder::new()
            .name(format!("smol-worker-{i}"))
            .spawn(move || smol::block_on(ex.run(std::future::pending::<()>())))
            .expect("Failed to spawn smol worker thread");
    }
}
