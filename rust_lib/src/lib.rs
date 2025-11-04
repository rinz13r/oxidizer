// #[ffi_function]
fn add(left: u64, right: u64) -> u64 {
    left + right
}

// #[ffi_function]
fn create_ffi_ty(x: u64, y: u64) -> FFITy {
    FFITy { x, y }
}

// #[ffi_type]
struct FFITy
{
    x: u64,
    y: u64,
}

/*
#[unsafe(no_mangle)]
pub extern "C" fn add(left: u64, right: u64) -> u64 {
    left + right
}
*/


fn main() {
    // Generates the FFI Bindings to C#

    let generator = Generator::new();
    generator.register_struct::<FFITy>();
    generator.register_function("add", add as fn(u64, u64) -> u64);
}