use oxidizer::prelude::*;
use oxidizer::{get_utils_registry, Registry};
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

#[ffi_function(RT)]
async fn check_async_2(_param: i32) -> u64 {
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    42
}

#[ffi_function]
fn heap_alloc_check() -> HeapAllocated<FFIHeapTy> {
    HeapAllocated::new(FFIHeapTy { x: 10, y: 20 })
}

#[ffi_function(RT)]
async fn heap_alloc_check_async() -> HeapAllocated<FFIHeapTy> {
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    HeapAllocated::new(FFIHeapTy { x: 10, y: 20 })
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

pub fn get_ffi_types_registry() -> Registry {
    let mut registry = get_utils_registry();

    registry
        .register_type::<FFITy>()
        .register_type::<HeapAllocated<FFIHeapTy>>()
        .register_function::<add>()
        .register_function::<check_async_1>()
        .register_function::<check_async_2>()
        .register_function::<heap_alloc_check>()
        .register_function::<heap_alloc_check_async>();

    registry
}
