use oxidizer_core::{FunctionInfo, TypeInfo, TypeKind, registry::Registry};
use std::collections::HashMap;

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

    /// Generates complete C# bindings from a Registry according to the async strategy
    pub fn generate_csharp(&self, registry: &Registry) -> String {
        let mut output = String::new();

        // Build type_id lookup map for infrastructure types
        let type_id_map = self.build_type_id_map(registry);

        // Add file header and usings
        output.push_str(&self.generate_file_header());

        // Collect unique return types for async functions to generate registrars
        let async_return_types = self.collect_async_return_types(registry.functions());

        // Generate registrar classes for async functions
        for return_type in &async_return_types {
            output.push_str(&self.generate_registrar_class(return_type, &type_id_map));
            output.push('\n');
        }

        // Collect unique slice callback types to generate their registrars
        let slice_callback_types = self.collect_slice_callback_types(registry.functions());

        // Generate slice callback registrar classes
        for element_type in &slice_callback_types {
            output.push_str(&self.generate_slice_callback_registrar(element_type, &type_id_map));
            output.push('\n');
        }

        // Generate infrastructure types from registry (raw structs)
        output.push_str(&self.generate_infrastructure_types(&type_id_map));

        // Generate wrapper classes for owned types
        output.push_str(&self.generate_owned_wrapper_class(&type_id_map));
        output.push('\n');

        // Generate slice wrapper classes
        output.push_str(&self.generate_slice_wrapper_classes(&type_id_map));
        output.push('\n');

        // Generate slice callback struct if needed
        if !slice_callback_types.is_empty() {
            output.push_str(&self.generate_slice_callback_struct());
            output.push('\n');
        }

        // Generate struct definitions
        for type_info in registry.types() {
            match FFIRepr::from_type_info(type_info) {
                FFIRepr::Owned => {
                    // For owned types, generate a marker struct (empty)
                    output.push_str(&self.generate_marker_struct(type_info));
                    output.push('\n');
                }
                FFIRepr::OwnedSlice
                | FFIRepr::Slice
                | FFIRepr::SliceMut
                | FFIRepr::SliceCallback => {
                    // Slice types use infrastructure types, no custom struct needed
                }
                FFIRepr::Direct => {
                    // For value types with fields, generate full struct
                    // Skip types that have a type_id (they are infrastructure types, already generated)
                    if matches!(type_info.kind(), TypeKind::Struct)
                        && !type_info.fields().is_empty()
                        && type_info.get_metadata(META_TYPE_ID).is_none()
                    {
                        output.push_str(&self.generate_struct(type_info, &type_id_map));
                        output.push('\n');
                    }
                }
            }
            // Primitives don't need struct generation
        }

        // Generate bindings class
        output.push_str("public static class Bindings\n{\n");

        // Generate function bindings
        for function in registry.functions() {
            if *function.is_async() {
                output.push_str(&self.generate_async_function_binding(function, &type_id_map));
            } else {
                output.push_str(&self.generate_sync_function_binding(function, &type_id_map));
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

    /// Collect unique element types used in SliceCallback parameters
    fn collect_slice_callback_types(&self, functions: &[FunctionInfo]) -> Vec<TypeInfo> {
        let mut element_types = HashMap::new();

        for function in functions {
            for param in function.parameters() {
                if FFIRepr::from_type_info(param.ty()) == FFIRepr::SliceCallback {
                    // Get the element type from generic_params
                    if let Some(element_type) = param.ty().generic_params().first() {
                        let key = format!("{}{:?}", element_type.name(), element_type.kind());
                        element_types.insert(key, element_type.clone());
                    }
                }
            }
        }

        element_types.into_values().collect()
    }

    /// Generate infrastructure types (raw structs) from TypeInfo in the registry
    fn generate_infrastructure_types(&self, type_id_map: &HashMap<String, TypeInfo>) -> String {
        let mut output = String::new();

        // Generate each infrastructure type from its TypeInfo
        for type_id in [
            OWNED_RAW_TYPE_ID,
            OWNED_SLICE_RAW_TYPE_ID,
            FFI_SLICE_RAW_TYPE_ID,
        ] {
            if let Some(type_info) = type_id_map.get(type_id) {
                output.push_str(&self.generate_infrastructure_struct(type_info, type_id_map));
                output.push('\n');
            }
        }

        output
    }

    /// Generate a single infrastructure struct from TypeInfo
    fn generate_infrastructure_struct(
        &self,
        type_info: &TypeInfo,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> String {
        let mut output = String::new();

        output.push_str(&format!(
            "/// <summary>FFI infrastructure type: {}</summary>\n",
            type_info.name()
        ));
        output.push_str("[StructLayout(LayoutKind.Sequential)]\n");
        output.push_str(&format!("public struct {}\n", type_info.name()));
        output.push_str("{\n");

        for field in type_info.fields() {
            let csharp_type = self.rust_type_to_csharp_type(field.ty(), type_id_map);
            let field_name = self.to_pascal_case(field.name());
            output.push_str(&format!("    public {csharp_type} {field_name};\n"));
        }

        output.push_str("}\n");
        output
    }

    /// Generate the OwnedHandle<T> wrapper class
    fn generate_owned_wrapper_class(&self, type_id_map: &HashMap<String, TypeInfo>) -> String {
        let mut output = String::new();

        // Get the raw type name from the registry
        let raw_type_name = type_id_map
            .get(OWNED_RAW_TYPE_ID)
            .map(|t| t.name().to_string())
            .unwrap_or_else(|| "OwnedRawHandle".to_string());

        output.push_str("/// <summary>\n");
        output.push_str("/// Type-safe wrapper for owned Rust objects.\n");
        output
            .push_str("/// Implements IDisposable to ensure proper cleanup of native resources.\n");
        output.push_str("/// </summary>\n");
        output.push_str("public sealed class OwnedHandle<T> : IDisposable\n");
        output.push_str("{\n");
        output.push_str(&format!("    private {raw_type_name} _raw;\n"));
        output.push_str("    private bool _disposed;\n\n");

        output.push_str(&format!(
            "    internal OwnedHandle({raw_type_name} raw) => _raw = raw;\n"
        ));
        output.push_str(&format!("    internal {raw_type_name} Raw => _raw;\n\n"));

        output.push_str("    public void Dispose()\n");
        output.push_str("    {\n");
        output.push_str("        if (_disposed) return;\n");
        output.push_str("        _disposed = true;\n\n");
        output.push_str("        if (_raw.Ptr != IntPtr.Zero)\n");
        output.push_str("        {\n");
        output.push_str("            Bindings.DropOwned(_raw);\n");
        output.push_str("            _raw.Ptr = IntPtr.Zero;\n");
        output.push_str("        }\n");
        output.push_str("    }\n");
        output.push_str("}\n");

        output
    }

    /// Generate slice wrapper classes (ReadOnlySliceHandle, SliceHandle, OwnedSliceHandle)
    fn generate_slice_wrapper_classes(&self, type_id_map: &HashMap<String, TypeInfo>) -> String {
        let mut output = String::new();

        let ffi_slice_raw_name = type_id_map
            .get(FFI_SLICE_RAW_TYPE_ID)
            .map(|t| t.name().to_string())
            .unwrap_or_else(|| "FFISliceRaw".to_string());

        let owned_slice_raw_name = type_id_map
            .get(OWNED_SLICE_RAW_TYPE_ID)
            .map(|t| t.name().to_string())
            .unwrap_or_else(|| "OwnedSliceRawHandle".to_string());

        // ReadOnlySliceHandle<T>
        output.push_str("/// <summary>\n");
        output.push_str("/// Read-only view into a borrowed Rust slice.\n");
        output.push_str("/// The underlying data is owned by Rust and must not be modified.\n");
        output.push_str("/// </summary>\n");
        output.push_str("public readonly ref struct ReadOnlySliceHandle<T> where T : unmanaged\n");
        output.push_str("{\n");
        output.push_str(&format!(
            "    private readonly {ffi_slice_raw_name} _raw;\n\n"
        ));
        output.push_str(&format!(
            "    internal ReadOnlySliceHandle({ffi_slice_raw_name} raw) => _raw = raw;\n\n"
        ));
        output.push_str("    public int Length => (int)_raw.Len;\n\n");
        output.push_str("    public unsafe ReadOnlySpan<T> AsSpan()\n");
        output.push_str("    {\n");
        output.push_str("        if (_raw.Ptr == IntPtr.Zero || _raw.Len == 0)\n");
        output.push_str("            return ReadOnlySpan<T>.Empty;\n");
        output.push_str("        return new ReadOnlySpan<T>((void*)_raw.Ptr, (int)_raw.Len);\n");
        output.push_str("    }\n");
        output.push_str("}\n\n");

        // SliceHandle<T>
        output.push_str("/// <summary>\n");
        output.push_str("/// Mutable view into a borrowed Rust slice.\n");
        output.push_str("/// </summary>\n");
        output.push_str("public readonly ref struct SliceHandle<T> where T : unmanaged\n");
        output.push_str("{\n");
        output.push_str(&format!(
            "    private readonly {ffi_slice_raw_name} _raw;\n\n"
        ));
        output.push_str(&format!(
            "    internal SliceHandle({ffi_slice_raw_name} raw) => _raw = raw;\n\n"
        ));
        output.push_str("    public int Length => (int)_raw.Len;\n\n");
        output.push_str("    public unsafe Span<T> AsSpan()\n");
        output.push_str("    {\n");
        output.push_str("        if (_raw.Ptr == IntPtr.Zero || _raw.Len == 0)\n");
        output.push_str("            return Span<T>.Empty;\n");
        output.push_str("        return new Span<T>((void*)_raw.Ptr, (int)_raw.Len);\n");
        output.push_str("    }\n");
        output.push_str("}\n\n");

        // OwnedSliceHandle<T>
        output.push_str("/// <summary>\n");
        output.push_str("/// Owned array transferred from Rust.\n");
        output
            .push_str("/// Implements IDisposable to ensure proper cleanup of native resources.\n");
        output.push_str("/// </summary>\n");
        output.push_str(
            "public sealed class OwnedSliceHandle<T> : IDisposable where T : unmanaged\n",
        );
        output.push_str("{\n");
        output.push_str(&format!("    private {owned_slice_raw_name} _raw;\n"));
        output.push_str("    private bool _disposed;\n\n");

        output.push_str(&format!(
            "    internal OwnedSliceHandle({owned_slice_raw_name} raw) => _raw = raw;\n"
        ));
        output.push_str(&format!(
            "    internal {owned_slice_raw_name} Raw => _raw;\n\n"
        ));

        output.push_str("    public int Length => (int)_raw.Len;\n\n");

        output.push_str("    public unsafe ReadOnlySpan<T> AsSpan()\n");
        output.push_str("    {\n");
        output.push_str("        if (_disposed) throw new ObjectDisposedException(nameof(OwnedSliceHandle<T>));\n");
        output.push_str("        if (_raw.Ptr == IntPtr.Zero || _raw.Len == 0)\n");
        output.push_str("            return ReadOnlySpan<T>.Empty;\n");
        output.push_str("        return new ReadOnlySpan<T>((void*)_raw.Ptr, (int)_raw.Len);\n");
        output.push_str("    }\n\n");

        output.push_str("    public T this[int index]\n");
        output.push_str("    {\n");
        output.push_str("        get\n");
        output.push_str("        {\n");
        output.push_str("            if (_disposed) throw new ObjectDisposedException(nameof(OwnedSliceHandle<T>));\n");
        output.push_str("            if (index < 0 || index >= (int)_raw.Len)\n");
        output.push_str("                throw new IndexOutOfRangeException();\n");
        output.push_str("            unsafe { return ((T*)_raw.Ptr)[index]; }\n");
        output.push_str("        }\n");
        output.push_str("    }\n\n");

        output.push_str("    public void Dispose()\n");
        output.push_str("    {\n");
        output.push_str("        if (_disposed) return;\n");
        output.push_str("        _disposed = true;\n\n");
        output.push_str("        if (_raw.Ptr != IntPtr.Zero)\n");
        output.push_str("        {\n");
        output.push_str("            Bindings.DropOwnedSlice(_raw);\n");
        output.push_str("            _raw.Ptr = IntPtr.Zero;\n");
        output.push_str("        }\n");
        output.push_str("    }\n");
        output.push_str("}\n");

        output
    }

    /// Generate the SliceCallback struct (generic, only needs to be emitted once)
    fn generate_slice_callback_struct(&self) -> String {
        let mut output = String::new();

        output.push_str("/// <summary>Callback struct for scoped slice access.</summary>\n");
        output.push_str("[StructLayout(LayoutKind.Sequential)]\n");
        output.push_str("internal struct SliceCallbackRaw\n");
        output.push_str("{\n");
        output.push_str("    public ulong Id;\n");
        output.push_str("    public IntPtr Func;\n");
        output.push_str("}\n");

        output
    }

    /// Generate a registrar class for slice callbacks of a specific element type
    fn generate_slice_callback_registrar(
        &self,
        element_type: &TypeInfo,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> String {
        let mut output = String::new();
        let csharp_element_type = self.rust_type_to_csharp_type(element_type, type_id_map);
        let class_name = format!("SliceCallbackRegistrar_{csharp_element_type}");

        output.push_str(&format!("class {class_name}\n{{\n"));
        output.push_str(&format!(
            "    public static readonly {class_name} Instance = new();\n\n"
        ));

        // Delegate for the native callback
        output.push_str(&format!(
            "    public delegate void CallbackDelegate(ulong id, FFISliceRaw slice);\n\n"
        ));

        // Dictionary and fields
        output.push_str(&format!(
            "    private readonly Dictionary<ulong, Action<ReadOnlySpan<{csharp_element_type}>>> registrations = new();\n"
        ));
        output.push_str("    private ulong id = 0;\n");
        output.push_str("    private readonly object lockObj = new();\n\n");

        // Private constructor
        output.push_str(&format!("    private {class_name}()\n    {{\n    }}\n\n"));

        // Register method
        output.push_str(&format!(
            "    public ulong Register(Action<ReadOnlySpan<{csharp_element_type}>> callback)\n"
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
        output.push_str("    public static void Callback(ulong id, FFISliceRaw slice)\n");
        output.push_str("    {\n");
        output.push_str("        if (Instance.registrations.TryGetValue(id, out var callback))\n");
        output.push_str("        {\n");
        output.push_str("            lock (Instance.lockObj)\n            {\n");
        output.push_str("                Instance.registrations.Remove(id);\n");
        output.push_str("            }\n");
        output.push_str("            unsafe\n");
        output.push_str("            {\n");
        output.push_str(&format!(
            "                var span = new ReadOnlySpan<{csharp_element_type}>((void*)slice.Ptr, (int)slice.Len);\n"
        ));
        output.push_str("                callback(span);\n");
        output.push_str("            }\n");
        output.push_str("        }\n");
        output.push_str("    }\n");
        output.push_str("}\n");

        output
    }

    fn generate_registrar_class(
        &self,
        return_type: &TypeInfo,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> String {
        let mut output = String::new();
        let class_name = self.get_registrar_class_name(return_type, type_id_map);
        let csharp_type = self.rust_type_to_csharp_type(return_type, type_id_map);

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

    /// Generate an empty marker struct for heap-only types
    fn generate_marker_struct(&self, type_info: &TypeInfo) -> String {
        let mut output = String::new();

        // Get the inner type name from generic_params
        let marker_name = self.get_inner_type_name(type_info);

        output.push_str(&format!(
            "/// <summary>Marker struct for heap-allocated {marker_name} instances.</summary>\n"
        ));
        output.push_str(&format!("public struct {marker_name} {{ }}\n"));

        output
    }

    /// Get the inner type name from generic_params
    fn get_inner_type_name(&self, type_info: &TypeInfo) -> String {
        type_info
            .generic_params()
            .first()
            .map(|inner| inner.name().to_string())
            .unwrap_or_else(|| type_info.name().to_string())
    }

    fn generate_struct(
        &self,
        type_info: &TypeInfo,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> String {
        let mut output = String::new();

        output.push_str("[StructLayout(LayoutKind.Sequential)]\n");
        output.push_str(&format!(
            "public struct {}\n",
            self.rust_type_to_csharp_name(type_info, type_id_map)
        ));
        output.push_str("{\n");

        for field in type_info.fields() {
            let csharp_type = self.rust_type_to_csharp_type(field.ty(), type_id_map);
            let field_name = self.to_pascal_case(field.name());
            output.push_str(&format!("    public {csharp_type} {field_name};\n"));
        }

        output.push_str("}\n");
        output
    }

    fn generate_async_function_binding(
        &self,
        function: &FunctionInfo,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> String {
        let mut output = String::new();
        let function_name = self.to_pascal_case(function.name());
        let raw_return_type = self.rust_type_to_csharp_type(function.return_type(), type_id_map);
        let registrar_class = self.get_registrar_class_name(function.return_type(), type_id_map);
        let return_repr = FFIRepr::from_type_info(function.return_type());

        // Determine the public return type (wrapped for owned types or owned slices)
        let public_return_type = match return_repr {
            FFIRepr::OwnedSlice => {
                let element_type =
                    self.get_generic_element_csharp_type(function.return_type(), type_id_map);
                format!("OwnedSliceHandle<{element_type}>")
            }
            FFIRepr::Owned => {
                let marker_type = self.get_inner_type_name(function.return_type());
                format!("OwnedHandle<{marker_type}>")
            }
            _ => raw_return_type.clone(),
        };

        // Generate public async method
        output.push_str(&format!(
            "    public static async Task<{public_return_type}> {function_name}("
        ));

        let params: Vec<String> = function
            .parameters()
            .iter()
            .map(|param| {
                let param_type = self.rust_type_to_csharp_type(param.ty(), type_id_map);
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

        // Return statement - wrap in OwnedSliceHandle for owned slices, OwnedHandle for owned types
        match return_repr {
            FFIRepr::OwnedSlice => {
                let element_type =
                    self.get_generic_element_csharp_type(function.return_type(), type_id_map);
                output.push_str(&format!(
                    "        return new OwnedSliceHandle<{element_type}>(await tcs.Task);\n"
                ));
            }
            FFIRepr::Owned => {
                let marker_type = self.get_inner_type_name(function.return_type());
                output.push_str(&format!(
                    "        return new OwnedHandle<{marker_type}>(await tcs.Task);\n"
                ));
            }
            _ => {
                output.push_str("        return await tcs.Task;\n");
            }
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

    fn generate_sync_function_binding(
        &self,
        function: &FunctionInfo,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> String {
        let mut output = String::new();
        let function_name = self.to_pascal_case(function.name());
        let return_repr = FFIRepr::from_type_info(function.return_type());
        let has_owned_params = function
            .parameters()
            .iter()
            .any(|p| FFIRepr::from_type_info(p.ty()) == FFIRepr::Owned);
        let has_slice_callback_params = function
            .parameters()
            .iter()
            .any(|p| FFIRepr::from_type_info(p.ty()) == FFIRepr::SliceCallback);

        // Generate raw params (for DllImport - uses raw types)
        let raw_params: Vec<String> = function
            .parameters()
            .iter()
            .map(|param| {
                let param_type = self.rust_type_to_csharp_type(param.ty(), type_id_map);
                let param_name = param.name().to_lowercase();
                format!("{param_type} {param_name}")
            })
            .collect();

        // Generate public params (for wrapper - uses friendly types)
        let public_params: Vec<String> = function
            .parameters()
            .iter()
            .map(|param| {
                let param_name = param.name().to_lowercase();
                match FFIRepr::from_type_info(param.ty()) {
                    FFIRepr::Owned => {
                        let marker_type = self.get_inner_type_name(param.ty());
                        format!("OwnedHandle<{marker_type}> {param_name}")
                    }
                    FFIRepr::SliceCallback => {
                        let element_type =
                            self.get_generic_element_csharp_type(param.ty(), type_id_map);
                        format!("Action<ReadOnlySpan<{element_type}>> {param_name}")
                    }
                    _ => {
                        let param_type = self.rust_type_to_csharp_type(param.ty(), type_id_map);
                        format!("{param_type} {param_name}")
                    }
                }
            })
            .collect();

        let needs_wrapper = matches!(return_repr, FFIRepr::Owned | FFIRepr::OwnedSlice)
            || has_owned_params
            || has_slice_callback_params;

        if needs_wrapper {
            // Private raw DllImport
            output.push_str(&format!(
                "    [DllImport(\"{}\", EntryPoint = \"{}\", CallingConvention = CallingConvention.Cdecl)]\n",
                self.library_name, function.name()
            ));

            let raw_return_type =
                self.rust_type_to_csharp_type(function.return_type(), type_id_map);
            output.push_str(&format!(
                "    private static extern {raw_return_type} {function_name}Internal({});\n\n",
                raw_params.join(", ")
            ));

            // Public typed wrapper
            let public_return_type = match return_repr {
                FFIRepr::OwnedSlice => {
                    let element_type =
                        self.get_generic_element_csharp_type(function.return_type(), type_id_map);
                    format!("OwnedSliceHandle<{element_type}>")
                }
                FFIRepr::Owned => {
                    let marker_type = self.get_inner_type_name(function.return_type());
                    format!("OwnedHandle<{marker_type}>")
                }
                _ => self.rust_type_to_csharp_type(function.return_type(), type_id_map),
            };

            output.push_str(&format!(
                "    public static {public_return_type} {function_name}({})\n",
                public_params.join(", ")
            ));
            output.push_str("    {\n");

            // Generate registration code for slice callbacks
            for param in function.parameters() {
                if FFIRepr::from_type_info(param.ty()) == FFIRepr::SliceCallback {
                    let param_name = param.name().to_lowercase();
                    let element_type =
                        self.get_generic_element_csharp_type(param.ty(), type_id_map);
                    let registrar = format!("SliceCallbackRegistrar_{element_type}");
                    output.push_str(&format!(
                        "        var {param_name}_id = {registrar}.Instance.Register({param_name});\n"
                    ));
                    output.push_str(&format!(
                        "        var {param_name}_raw = new SliceCallbackRaw {{ Id = {param_name}_id, Func = Marshal.GetFunctionPointerForDelegate<{registrar}.CallbackDelegate>({registrar}.Callback) }};\n"
                    ));
                }
            }

            // Generate call arguments
            let call_args: Vec<String> = function
                .parameters()
                .iter()
                .map(|param| {
                    let param_name = param.name().to_lowercase();
                    match FFIRepr::from_type_info(param.ty()) {
                        FFIRepr::Owned => format!("{param_name}.Raw"),
                        FFIRepr::SliceCallback => format!("{param_name}_raw"),
                        _ => param_name,
                    }
                })
                .collect();

            match return_repr {
                FFIRepr::OwnedSlice => {
                    let element_type =
                        self.get_generic_element_csharp_type(function.return_type(), type_id_map);
                    output.push_str(&format!(
                        "        return new OwnedSliceHandle<{element_type}>({function_name}Internal({}));\n",
                        call_args.join(", ")
                    ));
                }
                FFIRepr::Owned => {
                    let marker_type = self.get_inner_type_name(function.return_type());
                    output.push_str(&format!(
                        "        return new OwnedHandle<{marker_type}>({function_name}Internal({}));\n",
                        call_args.join(", ")
                    ));
                }
                _ => {
                    let return_type =
                        self.rust_type_to_csharp_type(function.return_type(), type_id_map);
                    if return_type == "void" {
                        output.push_str(&format!(
                            "        {function_name}Internal({});\n",
                            call_args.join(", ")
                        ));
                    } else {
                        output.push_str(&format!(
                            "            return {function_name}Internal({});\n",
                            call_args.join(", ")
                        ));
                    }
                }
            }
            output.push_str("    }\n");
        } else {
            // Standard sync function binding (no special types)
            let return_type = self.rust_type_to_csharp_type(function.return_type(), type_id_map);

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
        // Check FFI representation from metadata first
        // Look up raw type name via raw_type_id
        match FFIRepr::from_type_info(rust_type) {
            FFIRepr::Owned | FFIRepr::OwnedSlice | FFIRepr::Slice | FFIRepr::SliceMut => {
                if let Some(raw_type_id) = rust_type.get_metadata(META_RAW_TYPE_ID) {
                    if let Some(raw_type) = type_id_map.get(raw_type_id) {
                        return raw_type.name().to_string();
                    }
                }
                // Fallback for backward compatibility
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
        let return_type = TypeInfo::new("u64", vec![], TypeKind::U64, vec![], &[]);
        let type_id_map = HashMap::new();
        let registrar = generator.generate_registrar_class(&return_type, &type_id_map);

        assert!(registrar.contains("class Registrar_ulong"));
        assert!(registrar.contains("Action<ulong>"));
        assert!(registrar.contains("CallbackDelegate(ulong id, ulong result)"));
    }

    #[test]
    fn test_sync_function_binding() {
        let generator = CSharpGenerator::default();
        let return_type = TypeInfo::new("u64", vec![], TypeKind::U64, vec![], &[]);
        let param_type = TypeInfo::new("u32", vec![], TypeKind::U32, vec![], &[]);
        let param = FunctionParameter::new("value", param_type);
        let function = FunctionInfo::new("test_func", vec![param], return_type, false);
        let type_id_map = HashMap::new();

        let binding = generator.generate_sync_function_binding(&function, &type_id_map);

        assert!(binding.contains("[DllImport(\"rust_lib.dll\""));
        assert!(binding.contains("public static extern ulong TestFunc(uint value)"));
    }

    #[test]
    fn test_async_function_binding() {
        let generator = CSharpGenerator::default();
        let return_type = TypeInfo::new("u64", vec![], TypeKind::U64, vec![], &[]);
        let param_type = TypeInfo::new("u32", vec![], TypeKind::U32, vec![], &[]);
        let param = FunctionParameter::new("value", param_type);
        let function = FunctionInfo::new("test_async_func", vec![param], return_type, true);
        let type_id_map = HashMap::new();

        let binding = generator.generate_async_function_binding(&function, &type_id_map);

        assert!(binding.contains("public static async Task<ulong> TestAsyncFunc(uint value)"));
        assert!(binding.contains("TaskCompletionSource<ulong>"));
        assert!(binding.contains("Registrar_ulong.Instance.Register"));
        assert!(binding.contains("private static extern void TestAsyncFuncInternal"));
    }
}
