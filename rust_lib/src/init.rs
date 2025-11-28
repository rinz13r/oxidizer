lazy_static::lazy_static! {
    pub static ref RT: tokio::runtime::Runtime = {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(8)
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime")
    };
}

#[ctor::ctor]
fn init_runtime() {
    lazy_static::initialize(&RT);
}
