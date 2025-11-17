use oxidize_macro::{ffi_function, ffi_type};

#[ffi_function]
fn add(x: u64, y: u64) -> FFITy {
    FFITy { x, y }
}

#[ffi_type]
pub struct FFITy {
    pub x: u64,
    pub y: u64,
}

pub fn get_ffi_types_registry() -> oxidize_core::registry::Registry {
    let mut registry = oxidize_core::registry::Registry::new();

    registry.register_type::<FFITy>().register_function::<add>();

    registry
}
