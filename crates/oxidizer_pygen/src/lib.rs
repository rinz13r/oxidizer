use bon::Builder;
use oxidizer_core::{FunctionInfo, TypeInfo, TypeKind, registry::Registry};
use std::collections::HashMap;
pub use oxidizer_utils::{
    FFI_REPR_OWNED, FFI_REPR_OWNED_SLICE, FFI_REPR_SLICE, FFI_REPR_SLICE_CALLBACK,
    FFI_REPR_SLICE_MUT, FFI_SLICE_RAW_TYPE_ID, META_FFI_REPR, META_RAW_TYPE_ID, META_TYPE_ID,
    OWNED_RAW_TYPE_ID, OWNED_SLICE_RAW_TYPE_ID,
};

pub mod ir;
mod builder;
mod renderer;

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

/// Python code generator with configurable output
#[derive(Builder)]
pub struct PythonGenerator {
    /// Name of the native library (e.g., "librust_lib.so")
    #[builder(into)]
    library_name: String,
    /// Optional module docstring
    #[builder(into)]
    module_docstring: Option<String>,
    /// Indentation style for generated code
    #[builder(default)]
    indent_style: IndentStyle,
}

impl PythonGenerator {
    /// Generates complete Python bindings from a Registry.
    pub fn generate_python(&self, registry: &Registry) -> String {
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
        let type_name = self.rust_type_to_python_name(return_type, type_id_map);
        format!("_Registrar_{type_name}")
    }

    fn get_callback_type_name(
        &self,
        return_type: &TypeInfo,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> String {
        let type_name = self.rust_type_to_python_name(return_type, type_id_map);
        format!("_CallbackType_{type_name}")
    }

    /// Map Rust type to Python ctypes type (for FFI declarations and _fields_)
    fn rust_type_to_python_ctypes(
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
            TypeKind::U8 => "ctypes.c_uint8",
            TypeKind::U16 => "ctypes.c_uint16",
            TypeKind::U32 => "ctypes.c_uint32",
            TypeKind::U64 => "ctypes.c_uint64",
            TypeKind::I8 => "ctypes.c_int8",
            TypeKind::I16 => "ctypes.c_int16",
            TypeKind::I32 => "ctypes.c_int32",
            TypeKind::I64 => "ctypes.c_int64",
            TypeKind::F32 => "ctypes.c_float",
            TypeKind::F64 => "ctypes.c_double",
            TypeKind::Bool => "ctypes.c_uint8",
            TypeKind::Void => "None",
            TypeKind::Pointer => "ctypes.c_void_p",
            TypeKind::Struct => rust_type.name(),
        }
        .to_string()
    }

    /// Short name for a type (used in registrar/callback naming)
    fn rust_type_to_python_name(
        &self,
        rust_type: &TypeInfo,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> String {
        // For wrapper types, use the raw struct name
        match FFIRepr::from_type_info(rust_type) {
            FFIRepr::Owned | FFIRepr::OwnedSlice | FFIRepr::Slice | FFIRepr::SliceMut => {
                return self.rust_type_to_python_ctypes(rust_type, type_id_map);
            }
            FFIRepr::SliceCallback => return "SliceCallbackRaw".to_string(),
            FFIRepr::Direct => {}
        }

        match rust_type.kind() {
            TypeKind::U8 => "c_uint8".into(),
            TypeKind::U16 => "c_uint16".into(),
            TypeKind::U32 => "c_uint32".into(),
            TypeKind::U64 => "c_uint64".into(),
            TypeKind::I8 => "c_int8".into(),
            TypeKind::I16 => "c_int16".into(),
            TypeKind::I32 => "c_int32".into(),
            TypeKind::I64 => "c_int64".into(),
            TypeKind::F32 => "c_float".into(),
            TypeKind::F64 => "c_double".into(),
            TypeKind::Bool => "c_uint8".into(),
            TypeKind::Void => "None".into(),
            TypeKind::Pointer => "c_void_p".into(),
            TypeKind::Struct => rust_type.name().to_string(),
        }
    }

    /// Get the inner type name from generic_params
    fn get_inner_type_name(&self, type_info: &TypeInfo) -> String {
        type_info
            .generic_params()
            .first()
            .map(|inner| inner.name().to_string())
            .unwrap_or_else(|| type_info.name().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxidizer_core::{FunctionInfo, FunctionParameter, TypeInfo, TypeKind};

    fn default_generator() -> PythonGenerator {
        PythonGenerator::builder()
            .library_name("librust_lib.so")
            .build()
    }

    #[test]
    fn test_empty_registry() {
        let generator = default_generator();
        let registry = Registry::new();
        let output = generator.generate_python(&registry);

        assert!(output.contains("import ctypes"));
        assert!(output.contains("import asyncio"));
        assert!(output.contains("_lib = ctypes.CDLL("));
        assert!(output.contains("librust_lib.so"));
        // Should have wrapper classes
        assert!(output.contains("class OwnedHandle:"));
        assert!(output.contains("class OwnedSliceHandle:"));
        assert!(output.contains("class SliceHandle:"));
    }

    #[test]
    fn test_sync_function_binding() {
        let generator = default_generator();
        let return_type = TypeInfo::new("u64".to_string(), vec![], TypeKind::U64, vec![], &[]);
        let param_type = TypeInfo::new("u32".to_string(), vec![], TypeKind::U32, vec![], &[]);
        let param = FunctionParameter::new("value".to_string(), param_type);
        let function =
            FunctionInfo::new("test_func".to_string(), vec![param], return_type, false);

        let mut registry = Registry::new();
        registry.register_function_info(function);
        let output = generator.generate_python(&registry);

        assert!(output.contains("_lib.test_func.argtypes = [ctypes.c_uint32]"));
        assert!(output.contains("_lib.test_func.restype = ctypes.c_uint64"));
        assert!(output.contains("def test_func(value: int) -> int:"));
        assert!(output.contains("return _lib.test_func(value)"));
    }

    #[test]
    fn test_async_function_binding() {
        let generator = default_generator();
        let return_type = TypeInfo::new("u64".to_string(), vec![], TypeKind::U64, vec![], &[]);
        let param_type = TypeInfo::new("u32".to_string(), vec![], TypeKind::U32, vec![], &[]);
        let param = FunctionParameter::new("value".to_string(), param_type);
        let function =
            FunctionInfo::new("test_async_func".to_string(), vec![param], return_type, true);

        let mut registry = Registry::new();
        registry.register_function_info(function);
        let output = generator.generate_python(&registry);

        assert!(output.contains("_Registrar_c_uint64"));
        assert!(output.contains("_CallbackType_c_uint64"));
        assert!(output.contains("async def test_async_func(value: int) -> int:"));
        assert!(output.contains("loop = asyncio.get_running_loop()"));
        assert!(output.contains("registrar = _Registrar_c_uint64.instance()"));
        assert!(output.contains("cb_id, future = registrar.register(loop)"));
        assert!(output.contains("return await future"));
    }

    #[test]
    fn test_registrar_generation() {
        let generator = default_generator();
        let return_type = TypeInfo::new("f64".to_string(), vec![], TypeKind::F64, vec![], &[]);
        let function =
            FunctionInfo::new("do_async".to_string(), vec![], return_type, true);

        let mut registry = Registry::new();
        registry.register_function_info(function);
        let output = generator.generate_python(&registry);

        assert!(output.contains("class _Registrar_c_double:"));
        assert!(output.contains("_CallbackType_c_double = ctypes.CFUNCTYPE(None, ctypes.c_uint64, ctypes.c_double)"));
        assert!(output.contains("self._registrations = {}"));
        assert!(output.contains("self._lock = threading.Lock()"));
    }

    #[test]
    fn test_module_docstring() {
        let generator = PythonGenerator::builder()
            .library_name("test.so")
            .module_docstring("My bindings module.")
            .build();

        let registry = Registry::new();
        let output = generator.generate_python(&registry);

        assert!(output.starts_with("\"\"\"My bindings module.\"\"\""));
    }

    #[test]
    fn test_indent_style_spaces2() {
        let generator = PythonGenerator::builder()
            .library_name("test.so")
            .indent_style(IndentStyle::Spaces2)
            .build();

        let registry = Registry::new();
        let output = generator.generate_python(&registry);

        // 2-space indentation inside classes
        assert!(output.contains("\n  "));
    }

    #[test]
    fn test_indent_style_tabs() {
        let generator = PythonGenerator::builder()
            .library_name("test.so")
            .indent_style(IndentStyle::Tabs)
            .build();

        let registry = Registry::new();
        let output = generator.generate_python(&registry);

        assert!(output.contains("\t"));
    }

    #[test]
    fn test_user_struct() {
        let generator = default_generator();
        let field_x = oxidizer_core::FieldInfo::new(
            "x".to_string(),
            TypeInfo::new("u64".to_string(), vec![], TypeKind::U64, vec![], &[]),
        );
        let field_y = oxidizer_core::FieldInfo::new(
            "y".to_string(),
            TypeInfo::new("u64".to_string(), vec![], TypeKind::U64, vec![], &[]),
        );
        let struct_type = TypeInfo::new(
            "FFITy".to_string(),
            vec![field_x, field_y],
            TypeKind::Struct,
            vec![],
            &[],
        );

        let mut registry = Registry::new();
        registry.register_type_info(struct_type);
        let output = generator.generate_python(&registry);

        assert!(output.contains("class FFITy(ctypes.Structure):"));
        assert!(output.contains("(\"x\", ctypes.c_uint64)"));
        assert!(output.contains("(\"y\", ctypes.c_uint64)"));
    }

    #[test]
    fn test_void_return_function() {
        let generator = default_generator();
        let return_type = TypeInfo::new("void".to_string(), vec![], TypeKind::Void, vec![], &[]);
        let param_type = TypeInfo::new("u32".to_string(), vec![], TypeKind::U32, vec![], &[]);
        let param = FunctionParameter::new("x".to_string(), param_type);
        let function = FunctionInfo::new("do_stuff".to_string(), vec![param], return_type, false);

        let mut registry = Registry::new();
        registry.register_function_info(function);
        let output = generator.generate_python(&registry);

        assert!(output.contains("_lib.do_stuff.restype = None"));
        assert!(output.contains("def do_stuff(x: int):"));
        // No return annotation for void
        assert!(!output.contains("def do_stuff(x: int) -> "));
    }

    #[test]
    fn test_ir_builder_sync_function() {
        let generator = default_generator();
        let return_type = TypeInfo::new("u64".to_string(), vec![], TypeKind::U64, vec![], &[]);
        let param_type = TypeInfo::new("u32".to_string(), vec![], TypeKind::U32, vec![], &[]);
        let param = FunctionParameter::new("value".to_string(), param_type);
        let function =
            FunctionInfo::new("test_func".to_string(), vec![param], return_type, false);

        let mut registry = Registry::new();
        registry.register_function_info(function);
        let ir = generator.build_ir(&registry);

        // Should have wrapper classes + FFI declaration + wrapper function
        let has_function = ir.items.iter().any(|item| {
            matches!(item, ir::PythonItem::Function(f) if f.name == "test_func")
        });
        assert!(has_function);
    }
}
