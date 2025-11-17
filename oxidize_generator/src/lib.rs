use oxidize_core::{FunctionSignature, TypeInfo};

pub struct CSharpGenerator;

impl CSharpGenerator {
    pub fn generate_csharp(registry: &oxidize_core::registry::Registry) -> String {
        let mut output = String::new();

        // Add file header
        output.push_str(&Self::generate_file_header());

        // Generate struct definitions
        for type_info in registry.types() {
            if !type_info.fields.is_empty() {
                output.push_str(&Self::generate_struct(type_info));
                output.push('\n');
            }
        }

        // Generate function bindings
        output.push_str(&Self::generate_methods_class(registry.functions()));

        output.push_str("}\n");

        output
    }

    fn generate_file_header() -> String {
        "using System.Runtime.InteropServices;\n\npublic static class FFIMethods\n{\n".to_string()
    }

    fn generate_struct(type_info: &TypeInfo) -> String {
        let mut output = String::new();

        output.push_str("    [StructLayout(LayoutKind.Sequential)]\n");
        output.push_str(&format!(
            "    public struct {}\n",
            Self::rust_type_to_csharp_name(type_info)
        ));
        output.push_str("    {\n");

        for field in &type_info.fields {
            let csharp_type = Self::rust_type_to_csharp_name(&field.ty);
            let field_name = Self::to_pascal_case(field.name);
            output.push_str(&format!("        public {} {};\n", csharp_type, field_name));
        }

        output.push_str("    }\n");
        output
    }

    fn generate_methods_class(functions: &[FunctionSignature]) -> String {
        let mut output = String::new();

        for function in functions {
            output.push_str(&Self::generate_function_binding(function));
            output.push('\n');
        }

        output
    }

    fn generate_function_binding(function: &FunctionSignature) -> String {
        let mut output = String::new();

        let return_type = Self::rust_type_to_csharp_name(&function.return_type);
        let function_name = Self::to_pascal_case(function.name);

        output.push_str(&format!(
            "    [DllImport(\"rust_lib.dll\", EntryPoint = \"{}\", CallingConvention = CallingConvention.Cdecl)]\n",
            function.name
        ));

        output.push_str(&format!(
            "    public static extern {} {}(",
            return_type, function_name
        ));

        let params: Vec<String> = function
            .parameters
            .iter()
            .map(|param| {
                let param_type = Self::rust_type_to_csharp_name(param.ty());
                let param_name = param.name().to_lowercase().to_string();

                format!("{} {}", param_type, param_name)
            })
            .collect();

        output.push_str(&params.join(", "));
        output.push_str(");\n");

        output
    }

    fn rust_type_to_csharp_name(rust_type: &TypeInfo) -> String {
        match rust_type.kind {
            oxidize_core::TypeKind::U8 => "byte",
            oxidize_core::TypeKind::U16 => "ushort",
            oxidize_core::TypeKind::U32 => "uint",
            oxidize_core::TypeKind::U64 => "ulong",
            oxidize_core::TypeKind::I8 => "sbyte",
            oxidize_core::TypeKind::I16 => "short",
            oxidize_core::TypeKind::I32 => "int",
            oxidize_core::TypeKind::I64 => "long",
            oxidize_core::TypeKind::F32 => "float",
            oxidize_core::TypeKind::F64 => "double",
            oxidize_core::TypeKind::Bool => "bool",
            oxidize_core::TypeKind::UserDefined => &rust_type.name,
        }
        .to_string()
    }

    fn to_pascal_case(snake_case: &str) -> String {
        snake_case
            .split('_')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect()
    }
}
