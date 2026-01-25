use crate::{TypeInfo, TypeKind, WireType};
use std::ffi::c_void;

// Helper macro to implement WireType for primitive types
macro_rules! impl_wire_type_primitive {
    ($($ty:ty => ($name:expr, $kind:expr)),*) => {
        $(
            impl WireType for $ty {
                fn get_type_info() -> TypeInfo {
                    TypeInfo::new($name, Vec::new(), $kind, false)
                }
            }
        )*
    };
}

// Implement WireType for common primitive types
impl_wire_type_primitive! {
    u8 => ("u8", TypeKind::U8),
    u16 => ("u16", TypeKind::U16),
    u32 => ("u32", TypeKind::U32),
    u64 => ("u64", TypeKind::U64),
    usize => ("usize", TypeKind::U64), // usize maps to U64 for FFI (platform-dependent)
    i8 => ("i8", TypeKind::I8),
    i16 => ("i16", TypeKind::I16),
    i32 => ("i32", TypeKind::I32),
    i64 => ("i64", TypeKind::I64),
    f32 => ("f32", TypeKind::F32),
    f64 => ("f64", TypeKind::F64),
    bool => ("bool", TypeKind::Bool),
    *mut c_void => ("*mut c_void", TypeKind::Pointer),
    *const c_void => ("*const c_void", TypeKind::Pointer)
}
