use oxidize_core::{FunctionInfo, TypeInfo, TypeKind, registry::Registry};
use std::collections::HashMap;

pub struct CSharpGenerator {
    library_name: String,
}

impl Default for CSharpGenerator {
    fn default() -> Self {
        Self::new("rust_lib.dll".to_string())
    }
}

impl CSharpGenerator {
    pub fn new(library_name: String) -> Self {
        Self { library_name }
    }

    /// Generates complete C# bindings from a Registry according to the async strategy
    pub fn generate_csharp(&self, registry: &Registry) -> String {
        let mut output = String::new();

        // Add file header and usings
        output.push_str(&self.generate_file_header());

        // Collect unique return types for async functions to generate registrars
        let async_return_types = self.collect_async_return_types(registry.functions());

        // Generate registrar classes for async functions
        for return_type in &async_return_types {
            output.push_str(&self.generate_registrar_class(return_type));
            output.push('\n');
        }

        // Generate struct definitions
        for type_info in registry.types() {
            if !type_info.fields().is_empty() {
                output.push_str(&self.generate_struct(type_info));
                output.push('\n');
            }
        }

        // Generate bindings class
        output.push_str("public static class Bindings\n{\n");

        // Generate function bindings
        for function in registry.functions() {
            if *function.is_async() {
                output.push_str(&self.generate_async_function_binding(function));
            } else {
                output.push_str(&self.generate_sync_function_binding(function));
            }
            output.push('\n');
        }

        output.push_str("}\n");

        output
    }

    fn generate_file_header(&self) -> String {
        "using System;\nusing System.Collections.Generic;\nusing System.Runtime.InteropServices;\nusing System.Threading.Tasks;\n\n".to_string()
    }

    fn collect_async_return_types(&self, functions: &[FunctionInfo]) -> Vec<TypeInfo> {
        let mut return_types = HashMap::new();

        for function in functions {
            if *function.is_async() {
                let return_type = function.return_type();
                let key = format!("{}{:?}", return_type.name(), return_type.kind());
                return_types.insert(key, return_type.clone());
            }
        }

        return_types.into_values().collect()
    }

    fn generate_registrar_class(&self, return_type: &TypeInfo) -> String {
        let mut output = String::new();
        let class_name = self.get_registrar_class_name(return_type);
        let csharp_type = self.rust_type_to_csharp_type(return_type);

        output.push_str(&format!("class {}\n{{\n", class_name));
        output.push_str(&format!(
            "    public static readonly {} Instance = new();\n\n",
            class_name
        ));

        // Generate delegate
        output.push_str(&format!(
            "    public delegate void CallbackDelegate(ulong id, {} result);\n\n",
            csharp_type
        ));

        // Generate dictionary and other fields
        output.push_str(&format!(
            "    private readonly Dictionary<ulong, Action<{}>> registrations = new();\n",
            csharp_type
        ));
        output.push_str("    private ulong id = 0;\n");
        output.push_str("    private readonly object lockObj = new();\n\n");

        // Private constructor
        output.push_str(&format!("    private {}()\n    {{\n    }}\n\n", class_name));

        // Register method
        output.push_str(&format!(
            "    public ulong Register(Action<{}> callback)\n",
            csharp_type
        ));
        output.push_str("    {\n");
        output.push_str("        ulong currentId;\n\n");
        output.push_str("        lock (lockObj)\n        {\n");
        output.push_str("            currentId = id;\n");
        output.push_str("            registrations[currentId] = callback;\n");
        output.push_str("            id++;\n");
        output.push_str("        }\n\n");
        output.push_str("        return currentId;\n");
        output.push_str("    }\n\n");

        // Static callback method
        output.push_str(&format!(
            "    public static void Callback(ulong id, {} result)\n",
            csharp_type
        ));
        output.push_str("    {\n");
        output.push_str("        if (Instance.registrations.TryGetValue(id, out var callback))\n");
        output.push_str("        {\n");
        output.push_str("            lock (Instance.lockObj)\n            {\n");
        output.push_str("                Instance.registrations.Remove(id);\n");
        output.push_str("            }\n");
        output.push_str("            callback(result);\n");
        output.push_str("        }\n");
        output.push_str("    }\n");
        output.push_str("}\n");

        output
    }

    fn generate_struct(&self, type_info: &TypeInfo) -> String {
        let mut output = String::new();

        output.push_str("[StructLayout(LayoutKind.Sequential)]\n");
        output.push_str(&format!(
            "public struct {}\n",
            self.rust_type_to_csharp_name(type_info)
        ));
        output.push_str("{\n");

        for field in type_info.fields() {
            let csharp_type = self.rust_type_to_csharp_type(&field.ty());
            let field_name = self.to_pascal_case(field.name());
            output.push_str(&format!("    public {} {};\n", csharp_type, field_name));
        }

        output.push_str("}\n");
        output
    }

    fn generate_async_function_binding(&self, function: &FunctionInfo) -> String {
        let mut output = String::new();
        let function_name = self.to_pascal_case(function.name());
        let return_type = self.rust_type_to_csharp_type(function.return_type());
        let registrar_class = self.get_registrar_class_name(function.return_type());

        // Generate public async method
        output.push_str(&format!(
            "    public static async Task<{}> {}(",
            return_type, function_name
        ));

        let params: Vec<String> = function
            .parameters()
            .iter()
            .map(|param| {
                let param_type = self.rust_type_to_csharp_type(param.ty());
                let param_name = param.name().to_lowercase();
                format!("{} {}", param_type, param_name)
            })
            .collect();

        output.push_str(&params.join(", "));
        output.push_str(")\n    {\n");

        // Implementation
        output.push_str(&format!(
            "        var tcs = new TaskCompletionSource<{}>();\n\n",
            return_type
        ));
        output.push_str(&format!(
            "        var id = {}.Instance.Register(\n",
            registrar_class
        ));
        output.push_str(&format!("            ({} res) =>\n", return_type));
        output.push_str("            {\n");
        output.push_str("                tcs.SetResult(res);\n");
        output.push_str("            });\n\n");

        // Call internal method
        let param_names: Vec<String> = std::iter::once("id".to_string())
            .chain(
                function
                    .parameters()
                    .iter()
                    .map(|p| p.name().to_lowercase()),
            )
            .chain(std::iter::once(format!("{}.Callback", registrar_class)))
            .collect();

        output.push_str(&format!(
            "        {}Internal({});\n\n",
            function_name,
            param_names.join(", ")
        ));
        output.push_str("        return await tcs.Task;\n");
        output.push_str("    }\n\n");

        // Generate private DllImport method
        output.push_str(&format!(
            "    [DllImport(\"{}\", EntryPoint = \"{}\", CallingConvention = CallingConvention.Cdecl)]\n",
            self.library_name, function.name()
        ));

        let internal_params: Vec<String> = std::iter::once("ulong id".to_string())
            .chain(params.into_iter())
            .chain(std::iter::once(format!(
                "{}.CallbackDelegate cb",
                registrar_class
            )))
            .collect();

        output.push_str(&format!(
            "    private static extern void {}Internal({});\n",
            function_name,
            internal_params.join(", ")
        ));

        output
    }

    fn generate_sync_function_binding(&self, function: &FunctionInfo) -> String {
        let mut output = String::new();
        let function_name = self.to_pascal_case(function.name());
        let return_type = self.rust_type_to_csharp_type(function.return_type());

        output.push_str(&format!(
            "    [DllImport(\"{}\", EntryPoint = \"{}\", CallingConvention = CallingConvention.Cdecl)]\n",
            self.library_name, function.name()
        ));

        output.push_str(&format!(
            "    public static extern {} {}(",
            return_type, function_name
        ));

        let params: Vec<String> = function
            .parameters()
            .iter()
            .map(|param| {
                let param_type = self.rust_type_to_csharp_type(param.ty());
                let param_name = param.name().to_lowercase();
                format!("{} {}", param_type, param_name)
            })
            .collect();

        output.push_str(&params.join(", "));
        output.push_str(");\n");

        output
    }

    fn get_registrar_class_name(&self, return_type: &TypeInfo) -> String {
        let type_name = self.rust_type_to_csharp_name(return_type);
        format!("Registrar_{}", type_name)
    }

    fn rust_type_to_csharp_type(&self, rust_type: &TypeInfo) -> String {
        match rust_type.kind() {
            TypeKind::U8 => "byte",
            TypeKind::U16 => "ushort",
            TypeKind::U32 => "uint",
            TypeKind::U64 => "ulong",
            TypeKind::I8 => "sbyte",
            TypeKind::I16 => "short",
            TypeKind::I32 => "int",
            TypeKind::I64 => "long",
            TypeKind::F32 => "float",
            TypeKind::F64 => "double",
            TypeKind::Bool => "bool",
            TypeKind::Void => "void",
            TypeKind::UserDefined => &rust_type.name(),
        }
        .to_string()
    }

    fn rust_type_to_csharp_name(&self, rust_type: &TypeInfo) -> String {
        self.rust_type_to_csharp_type(rust_type)
    }

    fn to_pascal_case(&self, snake_case: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use oxidize_core::{FunctionInfo, FunctionParameter, TypeInfo, TypeKind};

    #[test]
    fn test_generate_registrar_class() {
        let generator = CSharpGenerator::default();
        let return_type = TypeInfo::new("u64", 8, vec![], TypeKind::U64);
        let registrar = generator.generate_registrar_class(&return_type);

        assert!(registrar.contains("class Registrar_ulong"));
        assert!(registrar.contains("Action<ulong>"));
        assert!(registrar.contains("CallbackDelegate(long id, ulong result)"));
    }

    #[test]
    fn test_sync_function_binding() {
        let generator = CSharpGenerator::default();
        let return_type = TypeInfo::new("u64", 8, vec![], TypeKind::U64);
        let param_type = TypeInfo::new("u32", 4, vec![], TypeKind::U32);
        let param = FunctionParameter::new("value", param_type);
        let function = FunctionInfo::new("test_func", vec![param], return_type, false);

        let binding = generator.generate_sync_function_binding(&function);

        assert!(binding.contains("[DllImport(\"rust_lib.dll\""));
        assert!(binding.contains("public static extern ulong TestFunc(uint value)"));
    }

    #[test]
    fn test_async_function_binding() {
        let generator = CSharpGenerator::default();
        let return_type = TypeInfo::new("u64", 8, vec![], TypeKind::U64);
        let param_type = TypeInfo::new("u32", 4, vec![], TypeKind::U32);
        let param = FunctionParameter::new("value", param_type);
        let function = FunctionInfo::new("test_async_func", vec![param], return_type, true);

        let binding = generator.generate_async_function_binding(&function);

        assert!(binding.contains("public static async Task<ulong> TestAsyncFunc(uint value)"));
        assert!(binding.contains("TaskCompletionSource<ulong>"));
        assert!(binding.contains("Registrar_ulong.Instance.Register"));
        assert!(binding.contains("private static extern void TestAsyncFuncInternal"));
    }
}
