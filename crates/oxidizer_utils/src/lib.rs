use std::ffi::c_void;

use oxidizer_core::{
    FieldInfo, FunctionInfo, FunctionParameter, TypeInfo, TypeKind, ReflectFunction, ReflectType,
};

// Note: We manually implement ReflectType/ReflectFunction here instead of using the macros
// because the macros generate code that references ::oxidizer::, which would create
// a circular dependency (oxidizer depends on oxidizer_utils).

// Constants for raw type names (used in C# generation)
const OWNED_RAW_NAME: &str = "OwnedRawHandle";
const OWNED_SLICE_RAW_NAME: &str = "OwnedSliceRawHandle";
const FFI_SLICE_RAW_NAME: &str = "FFISliceRaw";

// Constants for type IDs (used in metadata to link wrapper types to raw types)
pub const OWNED_RAW_TYPE_ID: &str = "owned_raw";
pub const OWNED_SLICE_RAW_TYPE_ID: &str = "owned_slice_raw";
pub const FFI_SLICE_RAW_TYPE_ID: &str = "ffi_slice_raw";

// Constants for metadata keys
pub const META_TYPE_ID: &str = "type_id";
pub const META_RAW_TYPE_ID: &str = "raw_type_id";
pub const META_FFI_REPR: &str = "ffi_repr";

// Constants for FFI representation values
pub const FFI_REPR_OWNED: &str = "owned";
pub const FFI_REPR_OWNED_SLICE: &str = "owned_slice";
pub const FFI_REPR_SLICE: &str = "slice";
pub const FFI_REPR_SLICE_MUT: &str = "slice_mut";
pub const FFI_REPR_SLICE_CALLBACK: &str = "slice_callback";

#[repr(C)]
pub struct OwnedRaw {
    ptr: *mut c_void,
    drop_fn: *const c_void,
}

impl ReflectType for OwnedRaw {
    fn get_type_info() -> TypeInfo {
        let fields = vec![
            FieldInfo::new("ptr".to_string(), <*mut c_void as ReflectType>::get_type_info()),
            FieldInfo::new("drop_fn".to_string(), <*const c_void as ReflectType>::get_type_info()),
        ];
        TypeInfo::new(
            OWNED_RAW_NAME.to_string(),
            fields,
            TypeKind::Struct,
            vec![],
            &[(META_TYPE_ID, OWNED_RAW_TYPE_ID)],
        )
    }
}

#[allow(non_camel_case_types)]
pub struct drop_owned;

impl drop_owned {
    #[unsafe(export_name = "drop_owned")]
    pub extern "C" fn call(owned: OwnedRaw) {
        unsafe {
            if !owned.ptr.is_null() && !owned.drop_fn.is_null() {
                let dropper: unsafe extern "C" fn(*mut c_void) = std::mem::transmute(owned.drop_fn);
                dropper(owned.ptr);
            }
        }
    }
}

impl ReflectFunction for drop_owned {
    fn get_function_info() -> FunctionInfo {
        FunctionInfo::new(
            "drop_owned".to_string(),
            vec![FunctionParameter::new("owned".to_string(), OwnedRaw::get_type_info())],
            TypeInfo::new("()".to_string(), Vec::new(), TypeKind::Void, vec![], &[]),
            false,
        )
    }
}

impl OwnedRaw {
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

    // Register raw types so the C# generator can generate them from TypeInfo
    registry.register_type::<OwnedRaw>();
    registry.register_type::<OwnedSliceRaw>();
    registry.register_type::<FFISliceRaw>();

    // Register drop function so the caller can dispose owned allocations
    registry.register_function::<drop_owned>();

    // Register drop function for owned slices
    registry.register_function::<drop_owned_slice>();

    registry
}

/// Type-safe wrapper for owned values passed across FFI.
///
/// This is the public API for creating owned objects that can be
/// passed across FFI. The caller receives an opaque handle.
#[repr(transparent)]
pub struct Owned<T> {
    inner: OwnedRaw,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Owned<T> {
    /// Create a new owned value.
    ///
    /// The value is boxed and ownership is transferred to the FFI boundary.
    /// The caller is responsible for disposing the handle.
    pub fn new(value: T) -> Self {
        Self {
            inner: OwnedRaw::new(value),
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

impl<T> ReflectType for Owned<T>
where
    T: ReflectType,
{
    fn get_type_info() -> TypeInfo {
        // Get the inner type's info to build the Owned type name
        let inner_type_info = T::get_type_info();
        let type_name = format!("Owned<{}>", inner_type_info.name());

        // Owned<T> has the same layout as OwnedRaw due to #[repr(transparent)]
        let raw_info = OwnedRaw::get_type_info();
        TypeInfo::new(
            type_name,
            raw_info.fields().clone(),
            TypeKind::Struct,
            vec![inner_type_info],
            &[
                (META_FFI_REPR, FFI_REPR_OWNED),
                (META_RAW_TYPE_ID, OWNED_RAW_TYPE_ID),
            ],
        )
    }
}

// =============================================================================
// FFISlice - Borrowed immutable slice (caller -> Rust only)
// =============================================================================

/// Borrowed immutable slice for FFI, for receiving data from the caller.
///
/// This type can only be received from the caller, not constructed in Rust.
/// For providing slice data from Rust to the caller, use [`SliceCallback`]
/// instead, which provides safe scoped access.
///
/// # Safety
/// The caller must ensure the underlying data outlives the FFI call.
#[repr(C)]
pub struct FFISlice<T> {
    ptr: *const T,
    len: usize,
}

impl<T> FFISlice<T> {
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

impl<T: ReflectType> ReflectType for FFISlice<T> {
    fn get_type_info() -> TypeInfo {
        let element_info = T::get_type_info();
        let type_name = format!("FFISlice<{}>", element_info.name());

        TypeInfo::new(
            type_name,
            vec![],
            TypeKind::Struct,
            vec![element_info],
            &[
                (META_FFI_REPR, FFI_REPR_SLICE),
                (META_RAW_TYPE_ID, FFI_SLICE_RAW_TYPE_ID),
            ],
        )
    }
}

// =============================================================================
// FFISliceMut - Borrowed mutable slice (caller -> Rust only)
// =============================================================================

/// Borrowed mutable slice for FFI, for receiving data from the caller.
///
/// This type can only be received from the caller, not constructed in Rust.
/// For providing slice data from Rust to the caller, use [`SliceCallback`]
/// instead, which provides safe scoped access.
///
/// # Safety
/// The caller must ensure the underlying data outlives the FFI call
/// and that no other references to the data exist during the call.
#[repr(C)]
pub struct FFISliceMut<T> {
    ptr: *mut T,
    len: usize,
}

impl<T> FFISliceMut<T> {
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

impl<T: ReflectType> ReflectType for FFISliceMut<T> {
    fn get_type_info() -> TypeInfo {
        let element_info = T::get_type_info();
        let type_name = format!("FFISliceMut<{}>", element_info.name());

        TypeInfo::new(
            type_name,
            vec![],
            TypeKind::Struct,
            vec![element_info],
            &[
                (META_FFI_REPR, FFI_REPR_SLICE_MUT),
                (META_RAW_TYPE_ID, FFI_SLICE_RAW_TYPE_ID),
            ],
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

impl ReflectType for FFISliceRaw {
    fn get_type_info() -> TypeInfo {
        let fields = vec![
            FieldInfo::new("ptr".to_string(), <*const c_void as ReflectType>::get_type_info()),
            FieldInfo::new("len".to_string(), <usize as ReflectType>::get_type_info()),
        ];
        TypeInfo::new(
            FFI_SLICE_RAW_NAME.to_string(),
            fields,
            TypeKind::Struct,
            vec![],
            &[(META_TYPE_ID, FFI_SLICE_RAW_TYPE_ID)],
        )
    }
}

// =============================================================================
// OwnedSlice - Owned Vec transfer
// =============================================================================

/// Owned slice for transferring Vec ownership across FFI.
///
/// This type takes ownership of a Vec and transfers it across the FFI boundary.
/// The caller receives ownership and must call the drop function to free the memory.
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

impl<T: ReflectType> ReflectType for OwnedSlice<T> {
    fn get_type_info() -> TypeInfo {
        let element_info = T::get_type_info();
        let type_name = format!("OwnedSlice<{}>", element_info.name());

        TypeInfo::new(
            type_name,
            vec![],
            TypeKind::Struct,
            vec![element_info],
            &[
                (META_FFI_REPR, FFI_REPR_OWNED_SLICE),
                (META_RAW_TYPE_ID, OWNED_SLICE_RAW_TYPE_ID),
            ],
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

impl ReflectType for OwnedSliceRaw {
    fn get_type_info() -> TypeInfo {
        let fields = vec![
            FieldInfo::new("ptr".to_string(), <*mut c_void as ReflectType>::get_type_info()),
            FieldInfo::new("len".to_string(), <usize as ReflectType>::get_type_info()),
            FieldInfo::new("capacity".to_string(), <usize as ReflectType>::get_type_info()),
            FieldInfo::new("element_size".to_string(), <usize as ReflectType>::get_type_info()),
            FieldInfo::new("drop_fn".to_string(), <*const c_void as ReflectType>::get_type_info()),
        ];
        TypeInfo::new(
            OWNED_SLICE_RAW_NAME.to_string(),
            fields,
            TypeKind::Struct,
            vec![],
            &[(META_TYPE_ID, OWNED_SLICE_RAW_TYPE_ID)],
        )
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

impl ReflectFunction for drop_owned_slice {
    fn get_function_info() -> FunctionInfo {
        FunctionInfo::new(
            "drop_owned_slice".to_string(),
            vec![FunctionParameter::new("os".to_string(), OwnedSliceRaw::get_type_info())],
            TypeInfo::new("()".to_string(), Vec::new(), TypeKind::Void, vec![], &[]),
            false,
        )
    }
}

// =============================================================================
// SliceCallback - Callback for scoped slice access
// =============================================================================

/// A callback that receives a borrowed slice.
///
/// This provides safe scoped access to slice data across FFI. The slice is only
/// valid during the callback invocation - Rust controls the lifetime.
///
/// # Example
/// ```ignore
/// #[ffi_function]
/// fn with_data(callback: SliceCallback<u64>) {
///     let data: Vec<u64> = vec![1, 2, 3, 4, 5];
///     callback.call(&data);
/// }
/// ```
#[repr(C)]
pub struct SliceCallback<T> {
    id: u64,
    func: extern "C" fn(u64, FFISlice<T>),
}

impl<T> SliceCallback<T> {
    /// Invoke the callback with the given slice.
    ///
    /// The slice data is only valid for the duration of this call.
    pub fn call(&self, slice: &[T]) {
        let ffi_slice = FFISlice {
            ptr: slice.as_ptr(),
            len: slice.len(),
        };
        (self.func)(self.id, ffi_slice);
    }
}

impl<T: ReflectType> ReflectType for SliceCallback<T> {
    fn get_type_info() -> TypeInfo {
        let element_info = T::get_type_info();
        let type_name = format!("SliceCallback<{}>", element_info.name());

        TypeInfo::new(
            type_name,
            vec![],
            TypeKind::Struct,
            vec![element_info],
            &[(META_FFI_REPR, FFI_REPR_SLICE_CALLBACK)],
        )
    }
}
