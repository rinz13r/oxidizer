use oxidizer::prelude::*;
use oxidizer::{Registry, get_utils_registry};
mod init;

use init::RT;

// Example slice functions

/// Returns an owned slice of numbers to C#.
/// The C# side receives an OwnedArray<ulong> which must be disposed.
#[ffi_function]
fn get_numbers() -> OwnedSlice<u64> {
    OwnedSlice::from_vec(vec![1, 2, 3, 4, 5])
}

/// Sums numbers from a borrowed slice.
#[ffi_function]
fn sum_numbers(data: FFISlice<u64>) -> u64 {
    unsafe { data.as_slice().iter().sum() }
}

/// Returns a larger array of numbers (for testing).
#[ffi_function]
fn get_large_array(count: u64) -> OwnedSlice<u64> {
    OwnedSlice::from_vec((0..count).collect())
}

/// Provides scoped access to data via callback. Safe!
#[ffi_function]
fn with_data(callback: SliceCallback<u64>) {
    let data: Vec<u64> = vec![1, 2, 3, 4, 5];
    callback.call(&data);
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

#[ffi_function(RT)]
async fn check_async_2(_param: i32) -> u64 {
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    42
}

#[ffi_function]
fn heap_alloc_check_1() -> Owned<FFIHeapTy> {
    Owned::new(FFIHeapTy { x: 10, y: 20 })
}

#[ffi_function]
fn heap_alloc_check_2(_param: Owned<FFIHeapTy>) {}

#[ffi_function]
fn heap_sum(param: Owned<FFIHeapTy>) -> u64 {
    let val = unsafe { param.as_ref() };
    val.x + val.y
}

#[ffi_function(RT)]
async fn heap_alloc_check_async() -> Owned<FFIHeapTy> {
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    Owned::new(FFIHeapTy { x: 10, y: 20 })
}

#[ffi_type]
pub struct FFITy {
    pub x: u64,
    pub y: u64,
}

#[ffi_type(marker)]
pub struct FFIHeapTy {
    pub x: u64,
    pub y: u64,
}

pub fn get_ffi_types_registry() -> Registry {
    let mut registry = get_utils_registry();

    registry
        .register_type::<FFITy>()
        .register_type::<Owned<FFIHeapTy>>()
        .register_function::<add>()
        .register_function::<check_async_1>()
        .register_function::<check_async_2>()
        .register_function::<heap_alloc_check_1>()
        .register_function::<heap_alloc_check_2>()
        .register_function::<heap_sum>()
        .register_function::<heap_alloc_check_async>()
        // Slice functions
        .register_function::<get_numbers>()
        .register_function::<sum_numbers>()
        .register_function::<get_large_array>()
        // Slice callback functions (safe scoped access)
        .register_function::<with_data>();

    registry
}
