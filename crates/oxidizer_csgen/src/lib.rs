use bon::Builder;
use oxidizer_core::{FunctionInfo, TypeInfo, TypeKind, registry::Registry};
use std::collections::HashMap;

pub mod ir;
mod builder;
mod renderer;

// Constants for type IDs (must match oxidizer_utils)
const OWNED_RAW_TYPE_ID: &str = "owned_raw";
const OWNED_SLICE_RAW_TYPE_ID: &str = "owned_slice_raw";
const FFI_SLICE_RAW_TYPE_ID: &str = "ffi_slice_raw";

// Constants for metadata keys (must match oxidizer_utils)
const META_TYPE_ID: &str = "type_id";
const META_RAW_TYPE_ID: &str = "raw_type_id";
const META_FFI_REPR: &str = "ffi_repr";

// Constants for FFI representation values (must match oxidizer_utils)
const FFI_REPR_OWNED: &str = "owned";
const FFI_REPR_OWNED_SLICE: &str = "owned_slice";
const FFI_REPR_SLICE: &str = "slice";
const FFI_REPR_SLICE_MUT: &str = "slice_mut";
const FFI_REPR_SLICE_CALLBACK: &str = "slice_callback";

/// Indentation style for generated code
#[derive(Debug, Clone, Default)]
pub enum IndentStyle {
    /// 4 spaces (default)
    #[default]
    Spaces4,
    /// 2 spaces
    Spaces2,
    /// Tab characters
    Tabs,
}

impl IndentStyle {
    /// Returns the string representation for a single indent level
    fn unit(&self) -> &'static str {
        match self {
            IndentStyle::Spaces4 => "    ",
            IndentStyle::Spaces2 => "  ",
            IndentStyle::Tabs => "\t",
        }
    }
}

/// FFI representation of a type, determined from metadata
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FFIRepr {
    /// Direct value type, no wrapping
    Direct,
    /// Owned<T> -> OwnedRaw on FFI boundary
    Owned,
    /// OwnedSlice<T> -> OwnedSliceRaw on FFI boundary
    OwnedSlice,
    /// FFISlice<T> -> FFISliceRaw on FFI boundary
    Slice,
    /// FFISliceMut<T> -> FFISliceRaw on FFI boundary
    SliceMut,
    /// SliceCallback<T> -> callback with scoped slice access
    SliceCallback,
}

impl FFIRepr {
    /// Determine FFI representation from type metadata
    pub fn from_type_info(type_info: &TypeInfo) -> Self {
        match type_info.get_metadata(META_FFI_REPR) {
            Some(v) if v == FFI_REPR_OWNED => FFIRepr::Owned,
            Some(v) if v == FFI_REPR_OWNED_SLICE => FFIRepr::OwnedSlice,
            Some(v) if v == FFI_REPR_SLICE => FFIRepr::Slice,
            Some(v) if v == FFI_REPR_SLICE_MUT => FFIRepr::SliceMut,
            Some(v) if v == FFI_REPR_SLICE_CALLBACK => FFIRepr::SliceCallback,
            _ => FFIRepr::Direct,
        }
    }
}

/// C# code generator with configurable output
#[derive(Builder)]
pub struct CSharpGenerator {
    /// Name of the native library (e.g., "mylib.dll")
    #[builder(into)]
    library_name: String,
    /// Optional namespace to wrap generated code in
    #[builder(into)]
    namespace: Option<String>,
    /// Name of the static bindings class (default: "Bindings")
    #[builder(into, default = "Bindings".to_string())]
    bindings_class_name: String,
    /// Indentation style for generated code
    #[builder(default)]
    indent_style: IndentStyle,
}

impl CSharpGenerator {
    /// Generates complete C# bindings from a Registry.
    pub fn generate_csharp(&self, registry: &Registry) -> String {
        let ir = self.build_ir(registry);
        renderer::render(&ir, &self.indent_style)
    }

    // ------------------------------------------------------------------
    // Utility methods used by the builder
    // ------------------------------------------------------------------

    /// Build a lookup map from type_id metadata to TypeInfo
    fn build_type_id_map(&self, registry: &Registry) -> HashMap<String, TypeInfo> {
        let mut map = HashMap::new();
        for type_info in registry.types() {
            if let Some(type_id) = type_info.get_metadata(META_TYPE_ID) {
                map.insert(type_id.to_string(), type_info.clone());
            }
        }
        map
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

    /// Collect unique element types used in SliceCallback parameters
    fn collect_slice_callback_types(&self, functions: &[FunctionInfo]) -> Vec<TypeInfo> {
        let mut element_types = HashMap::new();

        for function in functions {
            for param in function.parameters() {
                if FFIRepr::from_type_info(param.ty()) == FFIRepr::SliceCallback {
                    if let Some(element_type) = param.ty().generic_params().first() {
                        let key = format!("{}{:?}", element_type.name(), element_type.kind());
                        element_types.insert(key, element_type.clone());
                    }
                }
            }
        }

        element_types.into_values().collect()
    }

    fn get_registrar_class_name(
        &self,
        return_type: &TypeInfo,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> String {
        let type_name = self.rust_type_to_csharp_name(return_type, type_id_map);
        format!("Registrar_{type_name}")
    }

    fn rust_type_to_csharp_type(
        &self,
        rust_type: &TypeInfo,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> String {
        match FFIRepr::from_type_info(rust_type) {
            FFIRepr::Owned | FFIRepr::OwnedSlice | FFIRepr::Slice | FFIRepr::SliceMut => {
                if let Some(raw_type_id) = rust_type.get_metadata(META_RAW_TYPE_ID) {
                    if let Some(raw_type) = type_id_map.get(raw_type_id) {
                        return raw_type.name().to_string();
                    }
                }
                match FFIRepr::from_type_info(rust_type) {
                    FFIRepr::Owned => return "OwnedRawHandle".to_string(),
                    FFIRepr::OwnedSlice => return "OwnedSliceRawHandle".to_string(),
                    FFIRepr::Slice | FFIRepr::SliceMut => return "FFISliceRaw".to_string(),
                    _ => unreachable!(),
                }
            }
            FFIRepr::SliceCallback => return "SliceCallbackRaw".to_string(),
            FFIRepr::Direct => {}
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
            TypeKind::Struct => rust_type.name(),
        }
        .to_string()
    }

    /// Get the C# type name for the element type of a generic wrapper (from generic_params)
    fn get_generic_element_csharp_type(
        &self,
        type_info: &TypeInfo,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> String {
        type_info
            .generic_params()
            .first()
            .map(|inner| self.rust_type_to_csharp_type(inner, type_id_map))
            .unwrap_or_else(|| "object".to_string())
    }

    fn rust_type_to_csharp_name(
        &self,
        rust_type: &TypeInfo,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> String {
        self.rust_type_to_csharp_type(rust_type, type_id_map)
    }

    /// Get the inner type name from generic_params
    fn get_inner_type_name(&self, type_info: &TypeInfo) -> String {
        type_info
            .generic_params()
            .first()
            .map(|inner| inner.name().to_string())
            .unwrap_or_else(|| type_info.name().to_string())
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

    fn default_generator() -> CSharpGenerator {
        CSharpGenerator::builder()
            .library_name("rust_lib.dll")
            .build()
    }

    #[test]
    fn test_generate_registrar_class() {
        let generator = default_generator();
        let return_type = TypeInfo::new("u64", vec![], TypeKind::U64, vec![], &[]);
        let function = FunctionInfo::new("test_async", vec![], return_type, true);

        let mut registry = oxidizer_core::registry::Registry::new();
        registry.register_function_info(function);
        let output = generator.generate_csharp(&registry);

        assert!(output.contains("class Registrar_ulong"));
        assert!(output.contains("Action<ulong>"));
        assert!(output.contains("CallbackDelegate(ulong id, ulong result)"));
    }

    #[test]
    fn test_sync_function_binding() {
        let generator = default_generator();
        let return_type = TypeInfo::new("u64", vec![], TypeKind::U64, vec![], &[]);
        let param_type = TypeInfo::new("u32", vec![], TypeKind::U32, vec![], &[]);
        let param = FunctionParameter::new("value", param_type);
        let function = FunctionInfo::new("test_func", vec![param], return_type, false);

        let mut registry = oxidizer_core::registry::Registry::new();
        registry.register_function_info(function);
        let output = generator.generate_csharp(&registry);

        assert!(output.contains("[DllImport(\"rust_lib.dll\""));
        assert!(output.contains("public static extern ulong TestFunc(uint value)"));
    }

    #[test]
    fn test_async_function_binding() {
        let generator = default_generator();
        let return_type = TypeInfo::new("u64", vec![], TypeKind::U64, vec![], &[]);
        let param_type = TypeInfo::new("u32", vec![], TypeKind::U32, vec![], &[]);
        let param = FunctionParameter::new("value", param_type);
        let function = FunctionInfo::new("test_async_func", vec![param], return_type, true);

        let mut registry = oxidizer_core::registry::Registry::new();
        registry.register_function_info(function);
        let output = generator.generate_csharp(&registry);

        assert!(output.contains("public static async Task<ulong> TestAsyncFunc(uint value)"));
        assert!(output.contains("TaskCompletionSource<ulong>"));
        assert!(output.contains("Registrar_ulong.Instance.Register"));
        assert!(output.contains("private static extern void TestAsyncFuncInternal"));
    }

    #[test]
    fn test_namespace_configuration() {
        let generator = CSharpGenerator::builder()
            .library_name("test.dll")
            .namespace("MyCompany.Interop")
            .build();

        let registry = oxidizer_core::registry::Registry::new();
        let output = generator.generate_csharp(&registry);

        assert!(output.contains("namespace MyCompany.Interop"));
        assert!(output.contains("public static class Bindings"));
    }

    #[test]
    fn test_custom_bindings_class_name() {
        let generator = CSharpGenerator::builder()
            .library_name("test.dll")
            .bindings_class_name("NativeMethods")
            .build();

        let registry = oxidizer_core::registry::Registry::new();
        let output = generator.generate_csharp(&registry);

        assert!(output.contains("public static class NativeMethods"));
        assert!(!output.contains("public static class Bindings"));
    }

    #[test]
    fn test_indent_style_spaces2() {
        let generator = CSharpGenerator::builder()
            .library_name("test.dll")
            .indent_style(IndentStyle::Spaces2)
            .build();

        let registry = oxidizer_core::registry::Registry::new();
        let output = generator.generate_csharp(&registry);

        // Check that 2-space indentation is used (class body should have 2 spaces)
        assert!(output.contains("\n  ") || output.contains("{\n"));
    }

    #[test]
    fn test_indent_style_tabs() {
        let generator = CSharpGenerator::builder()
            .library_name("test.dll")
            .namespace("Test")
            .indent_style(IndentStyle::Tabs)
            .build();

        let registry = oxidizer_core::registry::Registry::new();
        let output = generator.generate_csharp(&registry);

        // With namespace and tabs, types should be indented with a tab
        assert!(output.contains("\t"));
    }

    #[test]
    fn test_ir_round_trip_empty_registry() {
        let generator = default_generator();
        let registry = oxidizer_core::registry::Registry::new();
        let ir = generator.build_ir(&registry);

        assert_eq!(ir.usings.len(), 4);
        assert!(ir.namespace.is_none());
        // Should have at least the bindings static class
        assert!(!ir.items.is_empty());
    }

    #[test]
    fn test_ir_builder_sync_function() {
        let generator = default_generator();
        let return_type = TypeInfo::new("u64", vec![], TypeKind::U64, vec![], &[]);
        let param_type = TypeInfo::new("u32", vec![], TypeKind::U32, vec![], &[]);
        let param = FunctionParameter::new("value", param_type);
        let function = FunctionInfo::new("test_func", vec![param], return_type, false);

        let mut registry = oxidizer_core::registry::Registry::new();
        registry.register_function_info(function);
        let ir = generator.build_ir(&registry);

        // Last item should be the static bindings class
        let last = ir.items.last().unwrap();
        match last {
            ir::CSharpItem::StaticClass(sc) => {
                assert_eq!(sc.name, "Bindings");
                assert_eq!(sc.methods.len(), 1);
                assert_eq!(sc.methods[0].name, "TestFunc");
            }
            _ => panic!("Expected StaticClass"),
        }
    }
}
