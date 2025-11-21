use oxidize_macro::{ffi_function, ffi_type};

lazy_static::lazy_static! {
    static ref RT: tokio::runtime::Runtime = {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(8)
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime")
    };
}

#[ctor::ctor]
fn init_runtime() {
    // Force initialization of the runtime at library load time
    lazy_static::initialize(&RT);
}

#[ffi_function]
fn add(x: u64, y: u64) -> FFITy {
    FFITy { x, y }
}

#[ffi_function(RT)]
async fn check_async_1(_param: i32) -> f64 {
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    42.0
}

#[ffi_function]
async fn check_async_2(_param: i32) -> u64 {
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    42
}

#[ffi_type]
pub struct FFITy {
    pub x: u64,
    pub y: u64,
}

pub fn get_ffi_types_registry() -> oxidize_core::registry::Registry {
    let mut registry = oxidize_core::registry::Registry::new();

    registry
        .register_type::<FFITy>()
        .register_function::<add>()
        .register_function::<check_async_1>()
        .register_function::<check_async_2>();

    registry
}
