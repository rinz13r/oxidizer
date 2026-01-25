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
        TypeInfo::new("HeapAllocatedRaw", fields, TypeKind::UserDefined, false)
    }
}

#[allow(non_camel_case_types)]
pub struct drop_heap_allocated;

impl drop_heap_allocated {
    #[unsafe(export_name = "drop_heap_allocated")]
    pub extern "C" fn call(ha: HeapAllocatedRaw) {
        unsafe {
            if !ha.ptr.is_null() {
                let dropper: unsafe extern "C" fn(*mut c_void) = std::mem::transmute(ha.drop_fn);
                dropper(ha.ptr);
                // ha.ptr = std::ptr::null_mut();
            }
        }
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
            TypeInfo::new("()", Vec::new(), TypeKind::Void, false),
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

pub fn get_utils_registry() -> oxidizer_core::registry::Registry {
    let mut registry = oxidizer_core::registry::Registry::new();

    // Register drop function so C# can call it to dispose heap allocations
    registry.register_function::<drop_heap_allocated>();

    // Register drop function for owned slices
    registry.register_function::<drop_owned_slice>();

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
            oxidizer_core::TypeKind::UserDefined,
            true, // is_heap_allocated
        )
    }
}

// =============================================================================
// FFISlice - Borrowed immutable slice
// =============================================================================

/// Borrowed immutable slice for FFI.
///
/// This type represents a borrowed `&[T]` across the FFI boundary.
/// The lifetime is erased at the FFI boundary, so the caller must ensure
/// the underlying data outlives the slice.
#[repr(C)]
pub struct FFISlice<T> {
    ptr: *const T,
    len: usize,
}

impl<T> FFISlice<T> {
    /// Create a new FFISlice from a Rust slice.
    pub fn from_slice(slice: &[T]) -> Self {
        Self {
            ptr: slice.as_ptr(),
            len: slice.len(),
        }
    }

    /// Get the slice as a Rust slice reference.
    ///
    /// # Safety
    /// The caller must ensure:
    /// - The pointer is valid and properly aligned
    /// - The underlying data has not been freed
    /// - No mutable references exist to the same data
    pub unsafe fn as_slice(&self) -> &[T] {
        if self.ptr.is_null() || self.len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
        }
    }

    /// Get the length of the slice.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if the slice is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<T: WireType> WireType for FFISlice<T> {
    fn get_type_info() -> TypeInfo {
        let element_info = T::get_type_info();
        let type_name =
            Box::leak(format!("FFISlice<{}>", element_info.name()).into_boxed_str());

        TypeInfo::new(
            type_name,
            vec![],
            TypeKind::Slice {
                element_kind: Box::new(element_info.kind().clone()),
            },
            false,
        )
    }
}

// =============================================================================
// FFISliceMut - Borrowed mutable slice
// =============================================================================

/// Borrowed mutable slice for FFI.
///
/// This type represents a borrowed `&mut [T]` across the FFI boundary.
/// The lifetime is erased at the FFI boundary, so the caller must ensure
/// the underlying data outlives the slice.
#[repr(C)]
pub struct FFISliceMut<T> {
    ptr: *mut T,
    len: usize,
}

impl<T> FFISliceMut<T> {
    /// Create a new FFISliceMut from a mutable Rust slice.
    pub fn from_slice(slice: &mut [T]) -> Self {
        Self {
            ptr: slice.as_mut_ptr(),
            len: slice.len(),
        }
    }

    /// Get the slice as a Rust slice reference.
    ///
    /// # Safety
    /// The caller must ensure:
    /// - The pointer is valid and properly aligned
    /// - The underlying data has not been freed
    /// - No other references exist to the same data
    pub unsafe fn as_slice(&self) -> &[T] {
        if self.ptr.is_null() || self.len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
        }
    }

    /// Get the slice as a mutable Rust slice reference.
    ///
    /// # Safety
    /// The caller must ensure:
    /// - The pointer is valid and properly aligned
    /// - The underlying data has not been freed
    /// - No other references exist to the same data
    pub unsafe fn as_slice_mut(&mut self) -> &mut [T] {
        if self.ptr.is_null() || self.len == 0 {
            &mut []
        } else {
            unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
        }
    }

    /// Get the length of the slice.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if the slice is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<T: WireType> WireType for FFISliceMut<T> {
    fn get_type_info() -> TypeInfo {
        let element_info = T::get_type_info();
        let type_name =
            Box::leak(format!("FFISliceMut<{}>", element_info.name()).into_boxed_str());

        TypeInfo::new(
            type_name,
            vec![],
            TypeKind::Slice {
                element_kind: Box::new(element_info.kind().clone()),
            },
            false,
        )
    }
}

// =============================================================================
// FFISliceRaw - Type-erased borrowed slice for FFI boundary
// =============================================================================

/// Type-erased borrowed slice for FFI boundary.
///
/// This is the raw representation used at the C FFI boundary.
#[repr(C)]
pub struct FFISliceRaw {
    pub ptr: *const c_void,
    pub len: usize,
}

impl WireType for FFISliceRaw {
    fn get_type_info() -> TypeInfo {
        let fields = vec![
            FieldInfo::new("ptr", <*const c_void as WireType>::get_type_info()),
            FieldInfo::new("len", <usize as WireType>::get_type_info()),
        ];
        TypeInfo::new("FFISliceRaw", fields, TypeKind::UserDefined, false)
    }
}

// =============================================================================
// OwnedSlice - Owned Vec transfer
// =============================================================================

/// Owned slice for transferring Vec ownership across FFI.
///
/// This type takes ownership of a Vec and transfers it across the FFI boundary.
/// The C# side receives an `OwnedArray<T>` which must be disposed to free the memory.
#[repr(C)]
pub struct OwnedSlice<T> {
    ptr: *mut T,
    len: usize,
    capacity: usize,
    element_size: usize,
    drop_fn: *const c_void,
}

impl<T> OwnedSlice<T> {
    /// Create an OwnedSlice from a Vec, transferring ownership.
    ///
    /// The Vec's memory will be managed by the FFI boundary.
    /// Call `drop_owned_slice` to free the memory.
    pub fn from_vec(vec: Vec<T>) -> Self {
        let mut vec = std::mem::ManuallyDrop::new(vec);

        unsafe extern "C" fn drop_vec<T>(ptr: *mut c_void, len: usize, capacity: usize) {
            unsafe {
                // Reconstruct and drop the Vec
                let _ = Vec::from_raw_parts(ptr as *mut T, len, capacity);
            }
        }

        Self {
            ptr: vec.as_mut_ptr(),
            len: vec.len(),
            capacity: vec.capacity(),
            element_size: std::mem::size_of::<T>(),
            drop_fn: drop_vec::<T> as *const c_void,
        }
    }

    /// Get the slice as a Rust slice reference.
    ///
    /// # Safety
    /// The caller must ensure the OwnedSlice has not been dropped.
    pub unsafe fn as_slice(&self) -> &[T] {
        if self.ptr.is_null() || self.len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
        }
    }

    /// Get the length of the slice.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if the slice is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<T: WireType> WireType for OwnedSlice<T> {
    fn get_type_info() -> TypeInfo {
        let element_info = T::get_type_info();
        let type_name =
            Box::leak(format!("OwnedSlice<{}>", element_info.name()).into_boxed_str());

        TypeInfo::new(
            type_name,
            vec![],
            TypeKind::OwnedSlice {
                element_kind: Box::new(element_info.kind().clone()),
            },
            true, // is_heap_allocated - needs cleanup
        )
    }
}

// =============================================================================
// OwnedSliceRaw - Type-erased owned slice for FFI boundary
// =============================================================================

/// Type-erased owned slice for FFI boundary.
///
/// This is the raw representation used at the C FFI boundary.
#[repr(C)]
pub struct OwnedSliceRaw {
    pub ptr: *mut c_void,
    pub len: usize,
    pub capacity: usize,
    pub element_size: usize,
    pub drop_fn: *const c_void,
}

impl WireType for OwnedSliceRaw {
    fn get_type_info() -> TypeInfo {
        let fields = vec![
            FieldInfo::new("ptr", <*mut c_void as WireType>::get_type_info()),
            FieldInfo::new("len", <usize as WireType>::get_type_info()),
            FieldInfo::new("capacity", <usize as WireType>::get_type_info()),
            FieldInfo::new("element_size", <usize as WireType>::get_type_info()),
            FieldInfo::new("drop_fn", <*const c_void as WireType>::get_type_info()),
        ];
        TypeInfo::new("OwnedSliceRaw", fields, TypeKind::UserDefined, false)
    }
}

// =============================================================================
// drop_owned_slice - FFI cleanup function
// =============================================================================

#[allow(non_camel_case_types)]
pub struct drop_owned_slice;

impl drop_owned_slice {
    #[unsafe(export_name = "drop_owned_slice")]
    pub extern "C" fn call(os: OwnedSliceRaw) {
        unsafe {
            if !os.ptr.is_null() && !os.drop_fn.is_null() {
                let dropper: unsafe extern "C" fn(*mut c_void, usize, usize) =
                    std::mem::transmute(os.drop_fn);
                dropper(os.ptr, os.len, os.capacity);
            }
        }
    }
}

impl WireFunction for drop_owned_slice {
    fn get_function_info() -> FunctionInfo {
        FunctionInfo::new(
            "drop_owned_slice",
            vec![FunctionParameter::new(
                "os",
                OwnedSliceRaw::get_type_info(),
            )],
            TypeInfo::new("()", Vec::new(), TypeKind::Void, false),
            false,
        )
    }
}
