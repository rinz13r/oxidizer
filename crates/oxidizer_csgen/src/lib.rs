use oxidizer_core::{FunctionInfo, TypeInfo, TypeKind, registry::Registry};
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

        // Generate HeapAllocatedRaw struct (internal) and HeapHandle<T> class
        output.push_str(&self.generate_heap_infrastructure());
        output.push('\n');

        // Generate struct definitions
        for type_info in registry.types() {
            if type_info.is_heap_allocated() {
                // For heap types, generate a marker struct (empty)
                output.push_str(&self.generate_marker_struct(type_info));
                output.push('\n');
            } else if matches!(type_info.kind(), TypeKind::UserDefined) {
                // For value types with fields, generate full struct
                if !type_info.fields().is_empty() {
                    output.push_str(&self.generate_struct(type_info));
                    output.push('\n');
                }
            }
            // Primitives don't need struct generation
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

        output.push_str(&format!("class {class_name}\n{{\n"));
        output.push_str(&format!(
            "    public static readonly {class_name} Instance = new();\n\n"
        ));

        // Generate delegate
        output.push_str(&format!(
            "    public delegate void CallbackDelegate(ulong id, {csharp_type} result);\n\n"
        ));

        // Generate dictionary and other fields
        output.push_str(&format!(
            "    private readonly Dictionary<ulong, Action<{csharp_type}>> registrations = new();\n"
        ));
        output.push_str("    private ulong id = 0;\n");
        output.push_str("    private readonly object lockObj = new();\n\n");

        // Private constructor
        output.push_str(&format!("    private {class_name}()\n    {{\n    }}\n\n"));

        // Register method
        output.push_str(&format!(
            "    public ulong Register(Action<{csharp_type}> callback)\n"
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
            "    public static void Callback(ulong id, {csharp_type} result)\n"
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

    /// Generate the HeapAllocatedRaw struct (internal) and HeapHandle<T> class
    fn generate_heap_infrastructure(&self) -> String {
        let mut output = String::new();

        // Internal HeapAllocatedRaw struct
        output.push_str("[StructLayout(LayoutKind.Sequential)]\n");
        output.push_str("public struct HeapAllocatedRaw\n");
        output.push_str("{\n");
        output.push_str("    public IntPtr Ptr;\n");
        output.push_str("    public IntPtr DropFn;\n");
        output.push_str("}\n\n");

        // Generic HeapHandle<T> class
        output.push_str("/// <summary>\n");
        output.push_str("/// Type-safe wrapper for heap-allocated Rust objects.\n");
        output
            .push_str("/// Implements IDisposable to ensure proper cleanup of native resources.\n");
        output.push_str("/// </summary>\n");
        output.push_str("public sealed class HeapHandle<T> : IDisposable\n");
        output.push_str("{\n");
        output.push_str("    private HeapAllocatedRaw _raw;\n");
        output.push_str("    private bool _disposed;\n\n");

        output.push_str("    internal HeapHandle(HeapAllocatedRaw raw) => _raw = raw;\n");
        output.push_str("    internal HeapAllocatedRaw Raw => _raw;\n\n");

        output.push_str("    public void Dispose()\n");
        output.push_str("    {\n");
        output.push_str("        if (_disposed) return;\n");
        output.push_str("        _disposed = true;\n\n");
        output.push_str("        if (_raw.Ptr != IntPtr.Zero)\n");
        output.push_str("        {\n");
        output.push_str("            Bindings.DropHeapAllocated(_raw);\n");
        output.push_str("            _raw.Ptr = IntPtr.Zero;\n");
        output.push_str("        }\n");
        output.push_str("    }\n");
        output.push_str("}\n");

        output
    }

    /// Generate an empty marker struct for heap-only types
    fn generate_marker_struct(&self, type_info: &TypeInfo) -> String {
        let mut output = String::new();

        // Extract the inner type name from HeapAllocated<T> format
        let type_name = type_info.name();
        let marker_name = self.extract_heap_inner_type(type_name);

        output.push_str(&format!(
            "/// <summary>Marker struct for heap-allocated {marker_name} instances.</summary>\n"
        ));
        output.push_str(&format!("public struct {marker_name} {{ }}\n"));

        output
    }

    /// Extract inner type from "HeapAllocated<TypeName>" -> "TypeName"
    fn extract_heap_inner_type(&self, type_name: &str) -> String {
        if type_name.starts_with("HeapAllocated<") && type_name.ends_with(">") {
            type_name["HeapAllocated<".len()..type_name.len() - 1].to_string()
        } else if type_name.ends_with("HeapHandle") {
            // Legacy format
            type_name[..type_name.len() - "HeapHandle".len()].to_string()
        } else {
            type_name.to_string()
        }
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
            let csharp_type = self.rust_type_to_csharp_type(field.ty());
            let field_name = self.to_pascal_case(field.name());
            output.push_str(&format!("    public {csharp_type} {field_name};\n"));
        }

        output.push_str("}\n");
        output
    }

    fn generate_async_function_binding(&self, function: &FunctionInfo) -> String {
        let mut output = String::new();
        let function_name = self.to_pascal_case(function.name());
        let raw_return_type = self.rust_type_to_csharp_type(function.return_type());
        let registrar_class = self.get_registrar_class_name(function.return_type());
        let returns_heap = self.is_heap_type(function.return_type());

        // Determine the public return type (wrapped for heap types)
        let public_return_type = if returns_heap {
            let marker_type = self.get_heap_marker_type(function.return_type());
            format!("HeapHandle<{marker_type}>")
        } else {
            raw_return_type.clone()
        };

        // Generate public async method
        output.push_str(&format!(
            "    public static async Task<{public_return_type}> {function_name}("
        ));

        let params: Vec<String> = function
            .parameters()
            .iter()
            .map(|param| {
                let param_type = self.rust_type_to_csharp_type(param.ty());
                let param_name = param.name().to_lowercase();
                format!("{param_type} {param_name}")
            })
            .collect();

        output.push_str(&params.join(", "));
        output.push_str(")\n    {\n");

        // Implementation - TaskCompletionSource always uses raw type
        output.push_str(&format!(
            "        var tcs = new TaskCompletionSource<{raw_return_type}>();\n\n"
        ));
        output.push_str(&format!(
            "        var id = {registrar_class}.Instance.Register(\n"
        ));
        output.push_str(&format!("            ({raw_return_type} res) =>\n"));
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
            .chain(std::iter::once(format!("{registrar_class}.Callback")))
            .collect();

        output.push_str(&format!(
            "        {}Internal({});\n\n",
            function_name,
            param_names.join(", ")
        ));

        // Return statement - wrap in HeapHandle for heap types
        if returns_heap {
            let marker_type = self.get_heap_marker_type(function.return_type());
            output.push_str(&format!(
                "        return new HeapHandle<{marker_type}>(await tcs.Task);\n"
            ));
        } else {
            output.push_str("        return await tcs.Task;\n");
        }
        output.push_str("    }\n\n");

        // Generate private DllImport method
        output.push_str(&format!(
            "    [DllImport(\"{}\", EntryPoint = \"{}\", CallingConvention = CallingConvention.Cdecl)]\n",
            self.library_name, function.name()
        ));

        let internal_params: Vec<String> = std::iter::once("ulong id".to_string())
            .chain(params)
            .chain(std::iter::once(format!(
                "{registrar_class}.CallbackDelegate cb"
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
        let returns_heap = self.is_heap_type(function.return_type());
        let has_heap_params = function
            .parameters()
            .iter()
            .any(|p| self.is_heap_type(p.ty()));

        // Generate raw params (for DllImport - uses HeapAllocatedRaw)
        let raw_params: Vec<String> = function
            .parameters()
            .iter()
            .map(|param| {
                let param_type = self.rust_type_to_csharp_type(param.ty());
                let param_name = param.name().to_lowercase();
                format!("{param_type} {param_name}")
            })
            .collect();

        // Generate public params (for wrapper - uses HeapHandle<T>)
        let public_params: Vec<String> = function
            .parameters()
            .iter()
            .map(|param| {
                let param_name = param.name().to_lowercase();
                if self.is_heap_type(param.ty()) {
                    let marker_type = self.get_heap_marker_type(param.ty());
                    format!("HeapHandle<{marker_type}> {param_name}")
                } else {
                    let param_type = self.rust_type_to_csharp_type(param.ty());
                    format!("{param_type} {param_name}")
                }
            })
            .collect();

        // Generate call arguments (extracts .Raw from HeapHandle params)
        let call_args: Vec<String> = function
            .parameters()
            .iter()
            .map(|param| {
                let param_name = param.name().to_lowercase();
                if self.is_heap_type(param.ty()) {
                    format!("{param_name}.Raw")
                } else {
                    param_name
                }
            })
            .collect();

        let needs_wrapper = returns_heap || has_heap_params;

        if needs_wrapper {
            // Private raw DllImport
            output.push_str(&format!(
                "    [DllImport(\"{}\", EntryPoint = \"{}\", CallingConvention = CallingConvention.Cdecl)]\n",
                self.library_name, function.name()
            ));

            let raw_return_type = self.rust_type_to_csharp_type(function.return_type());
            output.push_str(&format!(
                "    private static extern {raw_return_type} {function_name}Internal({});\n\n",
                raw_params.join(", ")
            ));

            // Public typed wrapper
            let public_return_type = if returns_heap {
                let marker_type = self.get_heap_marker_type(function.return_type());
                format!("HeapHandle<{marker_type}>")
            } else {
                self.rust_type_to_csharp_type(function.return_type())
            };

            output.push_str(&format!(
                "    public static {public_return_type} {function_name}({})\n",
                public_params.join(", ")
            ));
            output.push_str("    {\n");

            if returns_heap {
                let marker_type = self.get_heap_marker_type(function.return_type());
                output.push_str(&format!(
                    "        return new HeapHandle<{marker_type}>({function_name}Internal({}));\n",
                    call_args.join(", ")
                ));
            } else {
                let return_type = self.rust_type_to_csharp_type(function.return_type());
                if return_type == "void" {
                    output.push_str(&format!(
                        "        {function_name}Internal({});\n",
                        call_args.join(", ")
                    ));
                } else {
                    output.push_str(&format!(
                        "        return {function_name}Internal({});\n",
                        call_args.join(", ")
                    ));
                }
            }
            output.push_str("    }\n");
        } else {
            // Standard sync function binding (no heap types)
            let return_type = self.rust_type_to_csharp_type(function.return_type());

            output.push_str(&format!(
                "    [DllImport(\"{}\", EntryPoint = \"{}\", CallingConvention = CallingConvention.Cdecl)]\n",
                self.library_name, function.name()
            ));

            output.push_str(&format!(
                "    public static extern {return_type} {function_name}({});\n",
                raw_params.join(", ")
            ));
        }

        output
    }

    fn get_registrar_class_name(&self, return_type: &TypeInfo) -> String {
        let type_name = self.rust_type_to_csharp_name(return_type);
        format!("Registrar_{type_name}")
    }

    fn rust_type_to_csharp_type(&self, rust_type: &TypeInfo) -> String {
        // For heap-allocated types, use HeapAllocatedRaw at FFI boundary
        if rust_type.is_heap_allocated() {
            return "HeapAllocatedRaw".to_string();
        }

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
            TypeKind::Pointer => "IntPtr",
            TypeKind::UserDefined => rust_type.name(),
        }
        .to_string()
    }

    /// Check if a type is heap-allocated
    fn is_heap_type(&self, type_info: &TypeInfo) -> bool {
        type_info.is_heap_allocated()
    }

    /// Get the marker type name for a heap handle
    /// Extracts inner type from "HeapAllocated<T>" or legacy "THeapHandle" format
    fn get_heap_marker_type(&self, type_info: &TypeInfo) -> String {
        self.extract_heap_inner_type(type_info.name())
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
    use oxidizer_core::{FunctionInfo, FunctionParameter, TypeInfo, TypeKind};

    #[test]
    fn test_generate_registrar_class() {
        let generator = CSharpGenerator::default();
        let return_type = TypeInfo::new("u64", vec![], TypeKind::U64, false);
        let registrar = generator.generate_registrar_class(&return_type);

        assert!(registrar.contains("class Registrar_ulong"));
        assert!(registrar.contains("Action<ulong>"));
        assert!(registrar.contains("CallbackDelegate(ulong id, ulong result)"));
    }

    #[test]
    fn test_sync_function_binding() {
        let generator = CSharpGenerator::default();
        let return_type = TypeInfo::new("u64", vec![], TypeKind::U64, false);
        let param_type = TypeInfo::new("u32", vec![], TypeKind::U32, false);
        let param = FunctionParameter::new("value", param_type);
        let function = FunctionInfo::new("test_func", vec![param], return_type, false);

        let binding = generator.generate_sync_function_binding(&function);

        assert!(binding.contains("[DllImport(\"rust_lib.dll\""));
        assert!(binding.contains("public static extern ulong TestFunc(uint value)"));
    }

    #[test]
    fn test_async_function_binding() {
        let generator = CSharpGenerator::default();
        let return_type = TypeInfo::new("u64", vec![], TypeKind::U64, false);
        let param_type = TypeInfo::new("u32", vec![], TypeKind::U32, false);
        let param = FunctionParameter::new("value", param_type);
        let function = FunctionInfo::new("test_async_func", vec![param], return_type, true);

        let binding = generator.generate_async_function_binding(&function);

        assert!(binding.contains("public static async Task<ulong> TestAsyncFunc(uint value)"));
        assert!(binding.contains("TaskCompletionSource<ulong>"));
        assert!(binding.contains("Registrar_ulong.Instance.Register"));
        assert!(binding.contains("private static extern void TestAsyncFuncInternal"));
    }
}
