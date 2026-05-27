use std::fs;
use std::path::{Path, PathBuf};

use oxidizer_csgen::CSharpGenerator;
use oxidizer_pygen::PythonGenerator;

fn native_library_filename(base_name: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{base_name}.dll")
    } else if cfg!(target_os = "macos") {
        format!("lib{base_name}.dylib")
    } else {
        format!("lib{base_name}.so")
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("bindings-generator should live under tests/e2e/bindings-generator")
        .to_path_buf()
}

fn generated_bindings_dir() -> PathBuf {
    workspace_root()
        .join("target")
        .join("generated")
        .join("e2e")
}

fn main() {
    let registry = rust_lib::get_ffi_types_registry();
    let native_library_name = native_library_filename("rust_lib");

    let csharp_generator = CSharpGenerator::builder()
        .library_name("rust_lib")
        .bindings_class_name("MyBindings")
        .namespace("Native.Interop")
        .indent_style(oxidizer_csgen::IndentStyle::Tabs)
        .build();
    let csharp_contents = csharp_generator.generate_csharp(&registry);

    let python_generator = PythonGenerator::builder()
        .library_name(native_library_name)
        .build();
    let python_contents = python_generator.generate_python(&registry);

    let output_dir = generated_bindings_dir();
    fs::create_dir_all(&output_dir).expect("failed to create generated bindings directory");
    fs::write(output_dir.join("Generated.cs"), csharp_contents).expect("failed to write C# file");
    fs::write(output_dir.join("Generated.py"), python_contents)
        .expect("failed to write Python file");

    println!("Generated bindings in {}", output_dir.display());
}
