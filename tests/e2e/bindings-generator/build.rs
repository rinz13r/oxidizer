use std::env;
use std::fs;
use std::path::Path;

use oxidizer_csgen::CSharpGenerator;
use oxidizer_pygen::PythonGenerator;

fn native_library_filename(base_name: &str) -> String {
    match env::var("CARGO_CFG_TARGET_OS").as_deref() {
        Ok("windows") => format!("{base_name}.dll"),
        Ok("macos") => format!("lib{base_name}.dylib"),
        _ => format!("lib{base_name}.so"),
    }
}

fn main() {
    println!("Build tools starting...");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_OS");

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

    let output_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let src_dir = Path::new(&output_dir).join("src");

    fs::write(src_dir.join("Generated.cs"), csharp_contents).expect("Failed to write C# file");
    fs::write(src_dir.join("Generated.py"), python_contents).expect("Failed to write Python file");

    println!("Build tools completed successfully!");
}
