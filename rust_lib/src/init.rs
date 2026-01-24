const MAX_WORKER_THREADS: usize = 8;

lazy_static::lazy_static! {
    pub static ref RT: tokio::runtime::Runtime = {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(MAX_WORKER_THREADS)
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime")
    };
}

#[ctor::ctor]
fn init_runtime() {
    lazy_static::initialize(&RT);
}
