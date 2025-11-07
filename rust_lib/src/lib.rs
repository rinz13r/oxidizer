// #[ffi_function]
#[unsafe(no_mangle)]
pub extern "C" fn add(left: u64, right: u64) -> u64 {
    left + right
}

// #[ffi_function]
fn create_ffi_ty(x: u64, y: u64) -> FFITy {
    FFITy { x, y }
}

// #[ffi_type]
pub struct FFITy {
    pub x: u64,
    pub y: u64,
}

pub fn get_ffi_types_registry() -> oxidize_core::registry::Registry {
    // let registry = oxidize_core::registry::Registry::new();
    // registry.register_type::<FFITy>();
    // registry.register_function("add", add as fn(u64, u64) -> u64);

    todo!()
}

/*
#[unsafe(no_mangle)]
pub extern "C" fn add(left: u64, right: u64) -> u64 {
    left + right
}
*/
