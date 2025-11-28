use std::ffi::c_void;

use oxidize_macro::{ffi_function, ffi_type};

#[ffi_function]
pub extern "C" fn drop_heap_allocated(ha: HeapAllocatedRaw) {
    drop(ha);
}

#[ffi_type]
pub struct HeapAllocatedRaw {
    ptr: *mut c_void,
    drop_fn: *const c_void,
}

impl HeapAllocatedRaw {
    pub fn new<T>(value: T) -> Self {
        let boxed = Box::new(value);
        unsafe extern "C" fn drop_typed<T>(ptr: *mut c_void) {
            unsafe {
                drop(Box::from_raw(ptr as *mut T));
            }
        }

        Self {
            ptr: Box::into_raw(boxed) as *mut c_void,
            drop_fn: drop_typed::<T> as *mut c_void,
        }
    }

    #[allow(dead_code)]
    pub unsafe fn as_ref<T>(&self) -> &T {
        unsafe { &*(self.ptr as *const T) }
    }

    #[allow(dead_code)]
    pub unsafe fn as_mut<T>(&mut self) -> &mut T {
        unsafe { &mut *(self.ptr as *mut T) }
    }
}

impl Drop for HeapAllocatedRaw {
    fn drop(&mut self) {
        unsafe {
            if !self.ptr.is_null() {
                let dropper: unsafe extern "C" fn(*mut c_void) = std::mem::transmute(self.drop_fn);
                dropper(self.ptr);
                self.ptr = std::ptr::null_mut();
            }
        }
    }
}

pub fn get_utils_registry() -> oxidize_core::registry::Registry {
    let mut registry = oxidize_core::registry::Registry::new();

    registry
        .register_type::<HeapAllocatedRaw>()
        .register_function::<drop_heap_allocated>();

    registry
}
