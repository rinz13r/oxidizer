use std::ffi::c_void;

use oxidizer_core::{
    FieldInfo, FunctionInfo, FunctionParameter, TypeInfo, TypeKind, WireFunction, WireType,
};

// Note: We manually implement WireType/WireFunction here instead of using the macros
// because the macros generate code that references ::oxidizer::, which would create
// a circular dependency (oxidizer depends on oxidizer_utils).

#[repr(C)]
pub struct HeapAllocatedRaw {
    ptr: *mut c_void,
    drop_fn: *const c_void,
}

impl WireType for HeapAllocatedRaw {
    fn get_type_info() -> TypeInfo {
        let fields = vec![
            FieldInfo::new("ptr", <*mut c_void as WireType>::get_type_info()),
            FieldInfo::new("drop_fn", <*const c_void as WireType>::get_type_info()),
        ];
        TypeInfo::new("HeapAllocatedRaw", fields, TypeKind::UserDefined)
    }
}

#[allow(non_camel_case_types)]
pub struct drop_heap_allocated;

impl drop_heap_allocated {
    #[unsafe(export_name = "drop_heap_allocated")]
    pub extern "C" fn call(ha: HeapAllocatedRaw) {
        drop(ha);
    }
}

impl WireFunction for drop_heap_allocated {
    fn get_function_info() -> FunctionInfo {
        FunctionInfo::new(
            "drop_heap_allocated",
            vec![FunctionParameter::new(
                "ha",
                HeapAllocatedRaw::get_type_info(),
            )],
            TypeInfo::new("()", Vec::new(), TypeKind::Void),
            false,
        )
    }
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

pub fn get_utils_registry() -> oxidizer_core::registry::Registry {
    let mut registry = oxidizer_core::registry::Registry::new();

    // Register drop function so C# can call it to dispose heap allocations
    registry.register_function::<drop_heap_allocated>();

    registry
}

/// Type-safe wrapper for heap-allocated values passed across FFI.
///
/// This is the public API for creating heap-allocated objects that can be
/// passed to C#. The C# side receives this as `HeapHandle<T>`.
#[repr(transparent)]
pub struct HeapAllocated<T> {
    inner: HeapAllocatedRaw,
    _marker: std::marker::PhantomData<T>,
}

impl<T> HeapAllocated<T> {
    /// Create a new heap-allocated value.
    ///
    /// The value is boxed and ownership is transferred to the FFI boundary.
    /// The C# side is responsible for disposing the handle.
    pub fn new(value: T) -> Self {
        Self {
            inner: HeapAllocatedRaw::new(value),
            _marker: std::marker::PhantomData,
        }
    }

    /// Get a reference to the underlying value.
    ///
    /// # Safety
    /// The caller must ensure T matches the actual type stored.
    #[allow(dead_code)]
    pub unsafe fn as_ref(&self) -> &T {
        unsafe { self.inner.as_ref() }
    }

    /// Get a mutable reference to the underlying value.
    ///
    /// # Safety
    /// The caller must ensure T matches the actual type stored.
    #[allow(dead_code)]
    pub unsafe fn as_mut(&mut self) -> &mut T {
        unsafe { self.inner.as_mut() }
    }
}

impl<T> WireType for HeapAllocated<T>
where
    T: WireType,
{
    fn get_type_info() -> TypeInfo {
        // Get the inner type's info to build the HeapAllocated type name
        let inner_type_info = T::get_type_info();
        let type_name =
            Box::leak(format!("HeapAllocated<{}>", inner_type_info.name()).into_boxed_str());

        // HeapAllocated<T> has the same layout as HeapAllocatedRaw due to #[repr(transparent)]
        let raw_info = HeapAllocatedRaw::get_type_info();
        TypeInfo::new(
            type_name,
            raw_info.fields().clone(),
            oxidizer_core::TypeKind::HeapAllocated,
        )
    }
}
