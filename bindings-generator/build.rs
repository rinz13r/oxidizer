use std::env;
use std::fs;
use std::path::Path;

use oxidize_generator::CSharpGenerator;

fn main() {
    println!("Build tools starting...");

    let registry = rust_lib::get_ffi_types_registry();

    let csharp_contents = CSharpGenerator::generate_csharp(&registry);

    // Now generate a csharp file in src directory with contents as csharp_contents
    let output_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let dest_path = Path::new(&output_dir).join("src").join("Generated.cs");
    fs::write(&dest_path, csharp_contents).expect("Failed to write C# file");

    println!("Build tools completed successfully!");
}
