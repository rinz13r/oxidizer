use oxidizer_macro::{ffi_function, ffi_type};
pub mod heap_allocated;
mod init;

use init::RT;

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

#[ffi_function]
fn heap_alloc_check() -> heap_allocated::HeapAllocated<FFIHeapTy> {
    heap_allocated::HeapAllocated::new(FFIHeapTy { x: 10, y: 20 })
}

#[ffi_type]
pub struct FFITy {
    pub x: u64,
    pub y: u64,
}

#[ffi_type(heap)]
pub struct FFIHeapTy {
    pub x: u64,
    pub y: u64,
}

pub fn get_ffi_types_registry() -> oxidizer_core::registry::Registry {
    let mut registry = heap_allocated::get_utils_registry();

    registry
        .register_type::<FFITy>()
        .register_type::<heap_allocated::HeapAllocated<FFIHeapTy>>()
        .register_function::<add>()
        .register_function::<check_async_1>()
        .register_function::<check_async_2>()
        .register_function::<heap_alloc_check>();

    registry
}
