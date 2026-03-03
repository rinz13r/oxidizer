//! Oxidizer - Unified FFI bindings framework for Rust to C# interop.
//!
//! This crate re-exports all core oxidizer functionality. Use `oxidizer::prelude::*`
//! for common imports, or import specific items as needed.
//!
//! # Example
//! ```ignore
//! use oxidizer::prelude::*;
//!
//! #[ffi_type]
//! pub struct MyStruct {
//!     pub x: u64,
//! }
//!
//! #[ffi_function]
//! fn my_function(value: u64) -> MyStruct {
//!     MyStruct { x: value }
//! }
//! ```

// Private module for macro-generated code paths
// The macros reference ::oxidizer::__private::core::*
#[doc(hidden)]
pub mod __private {
    pub use oxidizer_core as core;
}

// Re-export macros at crate root for convenience
pub use oxidizer_macro::{ffi_function, ffi_type};

// Re-export core types
pub use oxidizer_core::{
    FieldInfo, FunctionInfo, FunctionParameter, TypeInfo, TypeKind, ReflectFunction, ReflectType,
};

// Re-export registry
pub use oxidizer_core::registry::Registry;

// Re-export Python code generator (optional)
#[cfg(feature = "pygen")]
pub use oxidizer_pygen;

// Re-export FFI utilities
pub use oxidizer_utils::{
    FFISlice, FFISliceMut, FFISliceRaw, Owned, OwnedRaw, OwnedSlice, OwnedSliceRaw, SliceCallback,
    get_utils_registry,
};

/// Prelude module for convenient imports.
///
/// Use `use oxidizer::prelude::*;` to import commonly used items:
/// - `ffi_function` - Attribute macro for FFI functions
/// - `ffi_type` - Attribute macro for FFI types
/// - `Owned` - Wrapper for owned FFI values
/// - `FFISlice` - Borrowed immutable slice for FFI
/// - `FFISliceMut` - Borrowed mutable slice for FFI
/// - `OwnedSlice` - Owned Vec transfer across FFI
/// - `SliceCallback` - Callback for scoped slice access
pub mod prelude {
    pub use crate::{
        FFISlice, FFISliceMut, Owned, OwnedSlice, SliceCallback, ffi_function, ffi_type,
    };
}
