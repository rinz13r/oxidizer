use oxidizer::ffi_function;

#[ffi_function]
fn foo((a, b): (u32, u32)) -> u32 {
    a + b
}

fn main() {}
