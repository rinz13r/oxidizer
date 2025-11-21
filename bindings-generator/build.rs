use std::env;
use std::fs;
use std::path::Path;

use oxidizer_csgen::CSharpGenerator;

fn main() {
    println!("Build tools starting...");

    let registry = rust_lib::get_ffi_types_registry();

    let csharp_generator = CSharpGenerator::new("rust_lib.dll".to_owned());
    let csharp_contents = csharp_generator.generate_csharp(&registry);

    // Now generate a csharp file in src directory with contents as csharp_contents
    let output_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let dest_path = Path::new(&output_dir).join("src").join("Generated.cs");
    fs::write(&dest_path, csharp_contents).expect("Failed to write C# file");

    println!("Build tools completed successfully!");
}
