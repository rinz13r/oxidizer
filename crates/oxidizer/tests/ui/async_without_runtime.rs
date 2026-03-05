use oxidizer::ffi_function;

#[ffi_function]
async fn missing_runtime() -> u64 {
    42
}

fn main() {}
