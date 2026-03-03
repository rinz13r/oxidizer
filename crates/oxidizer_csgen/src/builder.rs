use crate::ir::*;
use crate::{
    CSharpGenerator, FFIRepr, FFI_SLICE_RAW_TYPE_ID, OWNED_RAW_TYPE_ID,
    OWNED_SLICE_RAW_TYPE_ID,
};
use oxidizer_core::{FunctionInfo, TypeInfo, TypeKind, registry::Registry};
use std::collections::HashMap;

impl CSharpGenerator {
    pub(crate) fn build_ir(&self, registry: &Registry) -> CSharpFile {
        let type_id_map = self.build_type_id_map(registry);
        let mut items = Vec::new();

        // Registrar classes (per unique async return type)
        let async_return_types = self.collect_async_return_types(registry.functions());
        for rt in &async_return_types {
            items.push(self.build_registrar_class(rt, &type_id_map));
        }

        // Slice callback registrars (per unique element type)
        let cb_types = self.collect_slice_callback_types(registry.functions());
        for et in &cb_types {
            items.push(self.build_slice_callback_registrar(et, &type_id_map));
        }

        // Infrastructure types (raw structs)
        items.extend(self.build_infrastructure_types(&type_id_map));

        // Wrapper classes
        items.push(self.build_owned_wrapper_class(&type_id_map));
        items.extend(self.build_slice_wrapper_classes(&type_id_map));

        // Slice callback struct if needed
        if !cb_types.is_empty() {
            items.push(self.build_slice_callback_struct());
        }

        // User types
        for type_info in registry.types() {
            if let Some(item) = self.build_user_type(type_info, &type_id_map) {
                items.push(item);
            }
        }

        // Bindings class with all function methods
        let mut methods = Vec::new();
        for function in registry.functions() {
            methods.extend(self.build_function(function, &type_id_map));
        }
        items.push(CSharpItem::StaticClass(CSharpStaticClass {
            visibility: Visibility::Public,
            name: self.bindings_class_name.clone(),
            methods,
        }));

        CSharpFile {
            usings: vec![
                "System".into(),
                "System.Collections.Generic".into(),
                "System.Runtime.InteropServices".into(),
                "System.Threading.Tasks".into(),
            ],
            namespace: self.namespace.clone(),
            items,
        }
    }

    // ------------------------------------------------------------------
    // Infrastructure types
    // ------------------------------------------------------------------

    fn build_infrastructure_types(
        &self,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> Vec<CSharpItem> {
        let mut items = Vec::new();
        for type_id in [
            OWNED_RAW_TYPE_ID,
            OWNED_SLICE_RAW_TYPE_ID,
            FFI_SLICE_RAW_TYPE_ID,
        ] {
            if let Some(type_info) = type_id_map.get(type_id) {
                items.push(self.build_infrastructure_struct(type_info, type_id_map));
            }
        }
        items
    }

    fn build_infrastructure_struct(
        &self,
        type_info: &TypeInfo,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> CSharpItem {
        let fields = type_info
            .fields()
            .iter()
            .map(|field| CSharpField {
                visibility: Visibility::Public,
                is_static: false,
                is_readonly: false,
                type_name: self.rust_type_to_csharp_type(field.ty(), type_id_map),
                name: self.to_pascal_case(field.name()),
                initializer: None,
            })
            .collect();

        CSharpItem::Struct(CSharpStruct {
            doc_lines: vec![format!(
                "/// <summary>FFI infrastructure type: {}</summary>",
                type_info.name()
            )],
            attributes: vec!["[StructLayout(LayoutKind.Sequential)]".into()],
            visibility: Visibility::Public,
            is_ref_struct: false,
            name: type_info.name().to_string(),
            constraints: None,
            fields,
            properties: vec![],
            methods: vec![],
            indexers: vec![],
        })
    }

    // ------------------------------------------------------------------
    // OwnedHandle<T> wrapper class
    // ------------------------------------------------------------------

    fn build_owned_wrapper_class(
        &self,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> CSharpItem {
        let raw_type_name = type_id_map
            .get(OWNED_RAW_TYPE_ID)
            .map(|t| t.name().to_string())
            .unwrap_or_else(|| "OwnedRawHandle".to_string());

        CSharpItem::Class(CSharpClass {
            doc_lines: vec![
                "/// <summary>".into(),
                "/// Type-safe wrapper for owned Rust objects.".into(),
                "/// Implements IDisposable to ensure proper cleanup of native resources.".into(),
                "/// </summary>".into(),
            ],
            visibility: Visibility::Public,
            modifiers: vec!["sealed".into()],
            name: "OwnedHandle<T>".into(),
            constraints: None,
            implements: vec!["IDisposable".into()],
            fields: vec![
                CSharpField {
                    visibility: Visibility::Private,
                    is_static: false,
                    is_readonly: false,
                    type_name: raw_type_name.clone(),
                    name: "_raw".into(),
                    initializer: None,
                },
                CSharpField {
                    visibility: Visibility::Private,
                    is_static: false,
                    is_readonly: false,
                    type_name: "bool".into(),
                    name: "_disposed".into(),
                    initializer: None,
                },
            ],
            properties: vec![CSharpProperty {
                visibility: Visibility::Internal,
                is_static: false,
                type_name: raw_type_name.clone(),
                name: "Raw".into(),
                body: PropertyBody::Expression("_raw".into()),
            }],
            constructors: vec![CSharpMethod {
                doc_lines: vec![],
                attributes: vec![],
                visibility: Visibility::Internal,
                modifiers: vec![],
                return_type: String::new(),
                name: "OwnedHandle".into(),
                parameters: vec![CSharpParam {
                    type_name: raw_type_name.clone(),
                    name: "raw".into(),
                }],
                body: MethodBody::Expression("_raw = raw".into()),
            }],
            finalizer: Some(vec!["Dispose();".into()]),
            methods: vec![CSharpMethod {
                doc_lines: vec![],
                attributes: vec![],
                visibility: Visibility::Public,
                modifiers: vec![],
                return_type: "void".into(),
                name: "Dispose".into(),
                parameters: vec![],
                body: MethodBody::Block(vec![
                    "if (_disposed) return;".into(),
                    "_disposed = true;".into(),
                    "GC.SuppressFinalize(this);".into(),
                    String::new(),
                    "if (_raw.Ptr != IntPtr.Zero)".into(),
                    "{".into(),
                    format!("    {}.DropOwned(_raw);", self.bindings_class_name),
                    "    _raw.Ptr = IntPtr.Zero;".into(),
                    "}".into(),
                ]),
            }],
            delegates: vec![],
            indexers: vec![],
        })
    }

    // ------------------------------------------------------------------
    // Slice wrapper classes
    // ------------------------------------------------------------------

    fn build_slice_wrapper_classes(
        &self,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> Vec<CSharpItem> {
        let ffi_slice_raw_name = type_id_map
            .get(FFI_SLICE_RAW_TYPE_ID)
            .map(|t| t.name().to_string())
            .unwrap_or_else(|| "FFISliceRaw".to_string());

        let owned_slice_raw_name = type_id_map
            .get(OWNED_SLICE_RAW_TYPE_ID)
            .map(|t| t.name().to_string())
            .unwrap_or_else(|| "OwnedSliceRawHandle".to_string());

        vec![
            self.build_readonly_slice_handle(&ffi_slice_raw_name),
            self.build_slice_handle(&ffi_slice_raw_name),
            self.build_owned_slice_handle(&owned_slice_raw_name),
        ]
    }

    fn build_readonly_slice_handle(&self, ffi_slice_raw_name: &str) -> CSharpItem {
        CSharpItem::Struct(CSharpStruct {
            doc_lines: vec![
                "/// <summary>".into(),
                "/// Read-only view into a borrowed Rust slice.".into(),
                "/// The underlying data is owned by Rust and must not be modified.".into(),
                "/// </summary>".into(),
            ],
            attributes: vec![],
            visibility: Visibility::Public,
            is_ref_struct: true,
            name: "ReadOnlySliceHandle<T>".into(),
            constraints: Some("where T : unmanaged".into()),
            fields: vec![CSharpField {
                visibility: Visibility::Private,
                is_static: false,
                is_readonly: true,
                type_name: ffi_slice_raw_name.to_string(),
                name: "_raw".into(),
                initializer: None,
            }],
            properties: vec![CSharpProperty {
                visibility: Visibility::Public,
                is_static: false,
                type_name: "int".into(),
                name: "Length".into(),
                body: PropertyBody::Expression("(int)_raw.Len".into()),
            }],
            methods: vec![
                CSharpMethod {
                    doc_lines: vec![],
                    attributes: vec![],
                    visibility: Visibility::Internal,
                    modifiers: vec![],
                    return_type: String::new(),
                    name: "ReadOnlySliceHandle".into(),
                    parameters: vec![CSharpParam {
                        type_name: ffi_slice_raw_name.to_string(),
                        name: "raw".into(),
                    }],
                    body: MethodBody::Expression("_raw = raw".into()),
                },
                CSharpMethod {
                    doc_lines: vec![],
                    attributes: vec![],
                    visibility: Visibility::Public,
                    modifiers: vec![MethodModifier::Unsafe],
                    return_type: "ReadOnlySpan<T>".into(),
                    name: "AsSpan".into(),
                    parameters: vec![],
                    body: MethodBody::Block(vec![
                        "if (_raw.Ptr == IntPtr.Zero || _raw.Len == 0)".into(),
                        "    return ReadOnlySpan<T>.Empty;".into(),
                        "return new ReadOnlySpan<T>((void*)_raw.Ptr, (int)_raw.Len);".into(),
                    ]),
                },
            ],
            indexers: vec![],
        })
    }

    fn build_slice_handle(&self, ffi_slice_raw_name: &str) -> CSharpItem {
        CSharpItem::Struct(CSharpStruct {
            doc_lines: vec![
                "/// <summary>".into(),
                "/// Mutable view into a borrowed Rust slice.".into(),
                "/// </summary>".into(),
            ],
            attributes: vec![],
            visibility: Visibility::Public,
            is_ref_struct: true,
            name: "SliceHandle<T>".into(),
            constraints: Some("where T : unmanaged".into()),
            fields: vec![CSharpField {
                visibility: Visibility::Private,
                is_static: false,
                is_readonly: true,
                type_name: ffi_slice_raw_name.to_string(),
                name: "_raw".into(),
                initializer: None,
            }],
            properties: vec![CSharpProperty {
                visibility: Visibility::Public,
                is_static: false,
                type_name: "int".into(),
                name: "Length".into(),
                body: PropertyBody::Expression("(int)_raw.Len".into()),
            }],
            methods: vec![
                CSharpMethod {
                    doc_lines: vec![],
                    attributes: vec![],
                    visibility: Visibility::Internal,
                    modifiers: vec![],
                    return_type: String::new(),
                    name: "SliceHandle".into(),
                    parameters: vec![CSharpParam {
                        type_name: ffi_slice_raw_name.to_string(),
                        name: "raw".into(),
                    }],
                    body: MethodBody::Expression("_raw = raw".into()),
                },
                CSharpMethod {
                    doc_lines: vec![],
                    attributes: vec![],
                    visibility: Visibility::Public,
                    modifiers: vec![MethodModifier::Unsafe],
                    return_type: "Span<T>".into(),
                    name: "AsSpan".into(),
                    parameters: vec![],
                    body: MethodBody::Block(vec![
                        "if (_raw.Ptr == IntPtr.Zero || _raw.Len == 0)".into(),
                        "    return Span<T>.Empty;".into(),
                        "return new Span<T>((void*)_raw.Ptr, (int)_raw.Len);".into(),
                    ]),
                },
            ],
            indexers: vec![],
        })
    }

    fn build_owned_slice_handle(&self, owned_slice_raw_name: &str) -> CSharpItem {
        CSharpItem::Class(CSharpClass {
            doc_lines: vec![
                "/// <summary>".into(),
                "/// Owned array transferred from Rust.".into(),
                "/// Implements IDisposable to ensure proper cleanup of native resources.".into(),
                "/// </summary>".into(),
            ],
            visibility: Visibility::Public,
            modifiers: vec!["sealed".into()],
            name: "OwnedSliceHandle<T>".into(),
            constraints: Some("where T : unmanaged".into()),
            implements: vec!["IDisposable".into()],
            fields: vec![
                CSharpField {
                    visibility: Visibility::Private,
                    is_static: false,
                    is_readonly: false,
                    type_name: owned_slice_raw_name.to_string(),
                    name: "_raw".into(),
                    initializer: None,
                },
                CSharpField {
                    visibility: Visibility::Private,
                    is_static: false,
                    is_readonly: false,
                    type_name: "bool".into(),
                    name: "_disposed".into(),
                    initializer: None,
                },
            ],
            properties: vec![
                CSharpProperty {
                    visibility: Visibility::Internal,
                    is_static: false,
                    type_name: owned_slice_raw_name.to_string(),
                    name: "Raw".into(),
                    body: PropertyBody::Expression("_raw".into()),
                },
                CSharpProperty {
                    visibility: Visibility::Public,
                    is_static: false,
                    type_name: "int".into(),
                    name: "Length".into(),
                    body: PropertyBody::Expression("(int)_raw.Len".into()),
                },
            ],
            constructors: vec![CSharpMethod {
                doc_lines: vec![],
                attributes: vec![],
                visibility: Visibility::Internal,
                modifiers: vec![],
                return_type: String::new(),
                name: "OwnedSliceHandle".into(),
                parameters: vec![CSharpParam {
                    type_name: owned_slice_raw_name.to_string(),
                    name: "raw".into(),
                }],
                body: MethodBody::Expression("_raw = raw".into()),
            }],
            finalizer: Some(vec!["Dispose();".into()]),
            methods: vec![
                CSharpMethod {
                    doc_lines: vec![],
                    attributes: vec![],
                    visibility: Visibility::Public,
                    modifiers: vec![MethodModifier::Unsafe],
                    return_type: "ReadOnlySpan<T>".into(),
                    name: "AsSpan".into(),
                    parameters: vec![],
                    body: MethodBody::Block(vec![
                        "if (_disposed) throw new ObjectDisposedException(nameof(OwnedSliceHandle<T>));".into(),
                        "if (_raw.Ptr == IntPtr.Zero || _raw.Len == 0)".into(),
                        "    return ReadOnlySpan<T>.Empty;".into(),
                        "return new ReadOnlySpan<T>((void*)_raw.Ptr, (int)_raw.Len);".into(),
                    ]),
                },
                CSharpMethod {
                    doc_lines: vec![],
                    attributes: vec![],
                    visibility: Visibility::Public,
                    modifiers: vec![],
                    return_type: "void".into(),
                    name: "Dispose".into(),
                    parameters: vec![],
                    body: MethodBody::Block(vec![
                        "if (_disposed) return;".into(),
                        "_disposed = true;".into(),
                        "GC.SuppressFinalize(this);".into(),
                        String::new(),
                        "if (_raw.Ptr != IntPtr.Zero)".into(),
                        "{".into(),
                        format!("    {}.DropOwnedSlice(_raw);", self.bindings_class_name),
                        "    _raw.Ptr = IntPtr.Zero;".into(),
                        "}".into(),
                    ]),
                },
            ],
            delegates: vec![],
            indexers: vec![CSharpIndexer {
                visibility: Visibility::Public,
                type_name: "T".into(),
                parameter: CSharpParam {
                    type_name: "int".into(),
                    name: "index".into(),
                },
                getter_body: vec![
                    "if (_disposed) throw new ObjectDisposedException(nameof(OwnedSliceHandle<T>));".into(),
                    "if (index < 0 || index >= (int)_raw.Len)".into(),
                    "    throw new IndexOutOfRangeException();".into(),
                    "unsafe { return ((T*)_raw.Ptr)[index]; }".into(),
                ],
            }],
        })
    }

    // ------------------------------------------------------------------
    // SliceCallbackRaw struct
    // ------------------------------------------------------------------

    fn build_slice_callback_struct(&self) -> CSharpItem {
        CSharpItem::Struct(CSharpStruct {
            doc_lines: vec![
                "/// <summary>Callback struct for scoped slice access.</summary>".into(),
            ],
            attributes: vec!["[StructLayout(LayoutKind.Sequential)]".into()],
            visibility: Visibility::Internal,
            is_ref_struct: false,
            name: "SliceCallbackRaw".into(),
            constraints: None,
            fields: vec![
                CSharpField {
                    visibility: Visibility::Public,
                    is_static: false,
                    is_readonly: false,
                    type_name: "ulong".into(),
                    name: "Id".into(),
                    initializer: None,
                },
                CSharpField {
                    visibility: Visibility::Public,
                    is_static: false,
                    is_readonly: false,
                    type_name: "IntPtr".into(),
                    name: "Func".into(),
                    initializer: None,
                },
            ],
            properties: vec![],
            methods: vec![],
            indexers: vec![],
        })
    }

    // ------------------------------------------------------------------
    // Slice callback registrar
    // ------------------------------------------------------------------

    fn build_slice_callback_registrar(
        &self,
        element_type: &TypeInfo,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> CSharpItem {
        let csharp_element_type = self.rust_type_to_csharp_type(element_type, type_id_map);
        let class_name = format!("SliceCallbackRegistrar_{csharp_element_type}");

        CSharpItem::Class(CSharpClass {
            doc_lines: vec![],
            visibility: Visibility::Default,
            modifiers: vec![],
            name: class_name.clone(),
            constraints: None,
            implements: vec![],
            fields: vec![
                CSharpField {
                    visibility: Visibility::Public,
                    is_static: true,
                    is_readonly: true,
                    type_name: class_name.clone(),
                    name: "Instance".into(),
                    initializer: Some("new()".into()),
                },
                CSharpField {
                    visibility: Visibility::Private,
                    is_static: false,
                    is_readonly: true,
                    type_name: format!(
                        "Dictionary<ulong, Action<ReadOnlySpan<{csharp_element_type}>>>"
                    ),
                    name: "registrations".into(),
                    initializer: Some("new()".into()),
                },
                CSharpField {
                    visibility: Visibility::Private,
                    is_static: false,
                    is_readonly: false,
                    type_name: "ulong".into(),
                    name: "id".into(),
                    initializer: Some("0".into()),
                },
                CSharpField {
                    visibility: Visibility::Private,
                    is_static: false,
                    is_readonly: true,
                    type_name: "object".into(),
                    name: "lockObj".into(),
                    initializer: Some("new()".into()),
                },
            ],
            properties: vec![],
            constructors: vec![CSharpMethod {
                doc_lines: vec![],
                attributes: vec![],
                visibility: Visibility::Private,
                modifiers: vec![],
                return_type: String::new(),
                name: class_name.clone(),
                parameters: vec![],
                body: MethodBody::Block(vec![]),
            }],
            methods: vec![
                CSharpMethod {
                    doc_lines: vec![],
                    attributes: vec![],
                    visibility: Visibility::Public,
                    modifiers: vec![],
                    return_type: "ulong".into(),
                    name: "Register".into(),
                    parameters: vec![CSharpParam {
                        type_name: format!(
                            "Action<ReadOnlySpan<{csharp_element_type}>>"
                        ),
                        name: "callback".into(),
                    }],
                    body: MethodBody::Block(vec![
                        "ulong currentId;".into(),
                        String::new(),
                        "lock (lockObj)".into(),
                        "{".into(),
                        "    currentId = id;".into(),
                        "    registrations[currentId] = callback;".into(),
                        "    id++;".into(),
                        "}".into(),
                        String::new(),
                        "return currentId;".into(),
                    ]),
                },
                CSharpMethod {
                    doc_lines: vec![],
                    attributes: vec![],
                    visibility: Visibility::Public,
                    modifiers: vec![MethodModifier::Static],
                    return_type: "void".into(),
                    name: "Callback".into(),
                    parameters: vec![
                        CSharpParam {
                            type_name: "ulong".into(),
                            name: "id".into(),
                        },
                        CSharpParam {
                            type_name: "FFISliceRaw".into(),
                            name: "slice".into(),
                        },
                    ],
                    body: MethodBody::Block(vec![
                        format!("Action<ReadOnlySpan<{csharp_element_type}>> callback = null;"),
                        "lock (Instance.lockObj)".into(),
                        "{".into(),
                        "    if (Instance.registrations.TryGetValue(id, out callback))".into(),
                        "    {".into(),
                        "        Instance.registrations.Remove(id);".into(),
                        "    }".into(),
                        "}".into(),
                        "if (callback != null)".into(),
                        "{".into(),
                        "    unsafe".into(),
                        "    {".into(),
                        format!(
                            "        var span = new ReadOnlySpan<{csharp_element_type}>((void*)slice.Ptr, (int)slice.Len);"
                        ),
                        "        callback(span);".into(),
                        "    }".into(),
                        "}".into(),
                    ]),
                },
            ],
            finalizer: None,
            delegates: vec![CSharpDelegate {
                visibility: Visibility::Public,
                return_type: "void".into(),
                name: "CallbackDelegate".into(),
                parameters: vec![
                    CSharpParam {
                        type_name: "ulong".into(),
                        name: "id".into(),
                    },
                    CSharpParam {
                        type_name: "FFISliceRaw".into(),
                        name: "slice".into(),
                    },
                ],
            }],
            indexers: vec![],
        })
    }

    // ------------------------------------------------------------------
    // Async registrar class
    // ------------------------------------------------------------------

    fn build_registrar_class(
        &self,
        return_type: &TypeInfo,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> CSharpItem {
        let class_name = self.get_registrar_class_name(return_type, type_id_map);
        let csharp_type = self.rust_type_to_csharp_type(return_type, type_id_map);

        CSharpItem::Class(CSharpClass {
            doc_lines: vec![],
            visibility: Visibility::Default,
            modifiers: vec![],
            name: class_name.clone(),
            constraints: None,
            implements: vec![],
            fields: vec![
                CSharpField {
                    visibility: Visibility::Public,
                    is_static: true,
                    is_readonly: true,
                    type_name: class_name.clone(),
                    name: "Instance".into(),
                    initializer: Some("new()".into()),
                },
                CSharpField {
                    visibility: Visibility::Private,
                    is_static: false,
                    is_readonly: true,
                    type_name: format!("Dictionary<ulong, Action<{csharp_type}>>"),
                    name: "registrations".into(),
                    initializer: Some("new()".into()),
                },
                CSharpField {
                    visibility: Visibility::Private,
                    is_static: false,
                    is_readonly: false,
                    type_name: "ulong".into(),
                    name: "id".into(),
                    initializer: Some("0".into()),
                },
                CSharpField {
                    visibility: Visibility::Private,
                    is_static: false,
                    is_readonly: true,
                    type_name: "object".into(),
                    name: "lockObj".into(),
                    initializer: Some("new()".into()),
                },
            ],
            properties: vec![],
            constructors: vec![CSharpMethod {
                doc_lines: vec![],
                attributes: vec![],
                visibility: Visibility::Private,
                modifiers: vec![],
                return_type: String::new(),
                name: class_name.clone(),
                parameters: vec![],
                body: MethodBody::Block(vec![]),
            }],
            methods: vec![
                CSharpMethod {
                    doc_lines: vec![],
                    attributes: vec![],
                    visibility: Visibility::Public,
                    modifiers: vec![],
                    return_type: "ulong".into(),
                    name: "Register".into(),
                    parameters: vec![CSharpParam {
                        type_name: format!("Action<{csharp_type}>"),
                        name: "callback".into(),
                    }],
                    body: MethodBody::Block(vec![
                        "ulong currentId;".into(),
                        String::new(),
                        "lock (lockObj)".into(),
                        "{".into(),
                        "    currentId = id;".into(),
                        "    registrations[currentId] = callback;".into(),
                        "    id++;".into(),
                        "}".into(),
                        String::new(),
                        "return currentId;".into(),
                    ]),
                },
                CSharpMethod {
                    doc_lines: vec![],
                    attributes: vec![],
                    visibility: Visibility::Public,
                    modifiers: vec![MethodModifier::Static],
                    return_type: "void".into(),
                    name: "Callback".into(),
                    parameters: vec![
                        CSharpParam {
                            type_name: "ulong".into(),
                            name: "id".into(),
                        },
                        CSharpParam {
                            type_name: csharp_type.clone(),
                            name: "result".into(),
                        },
                    ],
                    body: MethodBody::Block(vec![
                        format!("Action<{csharp_type}> callback = null;"),
                        "lock (Instance.lockObj)".into(),
                        "{".into(),
                        "    if (Instance.registrations.TryGetValue(id, out callback))".into(),
                        "    {".into(),
                        "        Instance.registrations.Remove(id);".into(),
                        "    }".into(),
                        "}".into(),
                        "callback?.Invoke(result);".into(),
                    ]),
                },
            ],
            finalizer: None,
            delegates: vec![CSharpDelegate {
                visibility: Visibility::Public,
                return_type: "void".into(),
                name: "CallbackDelegate".into(),
                parameters: vec![
                    CSharpParam {
                        type_name: "ulong".into(),
                        name: "id".into(),
                    },
                    CSharpParam {
                        type_name: csharp_type,
                        name: "result".into(),
                    },
                ],
            }],
            indexers: vec![],
        })
    }

    // ------------------------------------------------------------------
    // User types (marker structs, value structs)
    // ------------------------------------------------------------------

    fn build_user_type(
        &self,
        type_info: &TypeInfo,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> Option<CSharpItem> {
        match FFIRepr::from_type_info(type_info) {
            FFIRepr::Owned => Some(self.build_marker_struct(type_info)),
            FFIRepr::OwnedSlice | FFIRepr::Slice | FFIRepr::SliceMut | FFIRepr::SliceCallback => {
                None
            }
            FFIRepr::Direct => {
                if matches!(type_info.kind(), TypeKind::Struct)
                    && !type_info.fields().is_empty()
                    && type_info.get_metadata(crate::META_TYPE_ID).is_none()
                {
                    Some(self.build_user_struct(type_info, type_id_map))
                } else {
                    None
                }
            }
        }
    }

    fn build_marker_struct(&self, type_info: &TypeInfo) -> CSharpItem {
        let marker_name = self.get_inner_type_name(type_info);
        CSharpItem::Struct(CSharpStruct {
            doc_lines: vec![format!(
                "/// <summary>Marker struct for heap-allocated {marker_name} instances.</summary>"
            )],
            attributes: vec![],
            visibility: Visibility::Public,
            is_ref_struct: false,
            name: marker_name,
            constraints: None,
            fields: vec![],
            properties: vec![],
            methods: vec![],
            indexers: vec![],
        })
    }

    fn build_user_struct(
        &self,
        type_info: &TypeInfo,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> CSharpItem {
        let fields = type_info
            .fields()
            .iter()
            .map(|field| CSharpField {
                visibility: Visibility::Public,
                is_static: false,
                is_readonly: false,
                type_name: self.rust_type_to_csharp_type(field.ty(), type_id_map),
                name: self.to_pascal_case(field.name()),
                initializer: None,
            })
            .collect();

        CSharpItem::Struct(CSharpStruct {
            doc_lines: vec![],
            attributes: vec!["[StructLayout(LayoutKind.Sequential)]".into()],
            visibility: Visibility::Public,
            is_ref_struct: false,
            name: self.rust_type_to_csharp_name(type_info, type_id_map),
            constraints: None,
            fields,
            properties: vec![],
            methods: vec![],
            indexers: vec![],
        })
    }

    // ------------------------------------------------------------------
    // Function bindings
    // ------------------------------------------------------------------

    fn build_function(
        &self,
        function: &FunctionInfo,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> Vec<CSharpMethod> {
        if *function.is_async() {
            self.build_async_function(function, type_id_map)
        } else {
            self.build_sync_function(function, type_id_map)
        }
    }

    fn build_sync_function(
        &self,
        function: &FunctionInfo,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> Vec<CSharpMethod> {
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

        // Raw params (for DllImport)
        let raw_params: Vec<CSharpParam> = function
            .parameters()
            .iter()
            .map(|param| CSharpParam {
                type_name: self.rust_type_to_csharp_type(param.ty(), type_id_map),
                name: param.name().to_lowercase(),
            })
            .collect();

        // Public params (for wrapper)
        let public_params: Vec<CSharpParam> = function
            .parameters()
            .iter()
            .map(|param| {
                let param_name = param.name().to_lowercase();
                match FFIRepr::from_type_info(param.ty()) {
                    FFIRepr::Owned => {
                        let marker_type = self.get_inner_type_name(param.ty());
                        CSharpParam {
                            type_name: format!("OwnedHandle<{marker_type}>"),
                            name: param_name,
                        }
                    }
                    FFIRepr::SliceCallback => {
                        let element_type =
                            self.get_generic_element_csharp_type(param.ty(), type_id_map);
                        CSharpParam {
                            type_name: format!("Action<ReadOnlySpan<{element_type}>>"),
                            name: param_name,
                        }
                    }
                    _ => CSharpParam {
                        type_name: self.rust_type_to_csharp_type(param.ty(), type_id_map),
                        name: param_name,
                    },
                }
            })
            .collect();

        let needs_wrapper = matches!(return_repr, FFIRepr::Owned | FFIRepr::OwnedSlice)
            || has_owned_params
            || has_slice_callback_params;

        if needs_wrapper {
            let raw_return_type =
                self.rust_type_to_csharp_type(function.return_type(), type_id_map);

            // Private raw DllImport
            let dll_import = CSharpMethod {
                doc_lines: vec![],
                attributes: vec![format!(
                    "[DllImport(\"{}\", EntryPoint = \"{}\", CallingConvention = CallingConvention.Cdecl)]",
                    self.library_name, function.name()
                )],
                visibility: Visibility::Private,
                modifiers: vec![MethodModifier::Static, MethodModifier::Extern],
                return_type: raw_return_type.clone(),
                name: format!("{function_name}Internal"),
                parameters: raw_params,
                body: MethodBody::None,
            };

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
                _ => raw_return_type.clone(),
            };

            // Build wrapper body
            let mut body_lines = Vec::new();

            // Slice callback registration
            for param in function.parameters() {
                if FFIRepr::from_type_info(param.ty()) == FFIRepr::SliceCallback {
                    let param_name = param.name().to_lowercase();
                    let element_type =
                        self.get_generic_element_csharp_type(param.ty(), type_id_map);
                    let registrar = format!("SliceCallbackRegistrar_{element_type}");
                    body_lines.push(format!(
                        "var {param_name}_id = {registrar}.Instance.Register({param_name});"
                    ));
                    body_lines.push(format!(
                        "var {param_name}_raw = new SliceCallbackRaw {{ Id = {param_name}_id, Func = Marshal.GetFunctionPointerForDelegate<{registrar}.CallbackDelegate>({registrar}.Callback) }};"
                    ));
                }
            }

            // Call arguments
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
            let call_args_str = call_args.join(", ");

            match return_repr {
                FFIRepr::OwnedSlice => {
                    let element_type =
                        self.get_generic_element_csharp_type(function.return_type(), type_id_map);
                    body_lines.push(format!(
                        "return new OwnedSliceHandle<{element_type}>({function_name}Internal({call_args_str}));"
                    ));
                }
                FFIRepr::Owned => {
                    let marker_type = self.get_inner_type_name(function.return_type());
                    body_lines.push(format!(
                        "return new OwnedHandle<{marker_type}>({function_name}Internal({call_args_str}));"
                    ));
                }
                _ => {
                    if raw_return_type == "void" {
                        body_lines
                            .push(format!("{function_name}Internal({call_args_str});"));
                    } else {
                        body_lines.push(format!(
                            "return {function_name}Internal({call_args_str});"
                        ));
                    }
                }
            }

            let wrapper = CSharpMethod {
                doc_lines: vec![],
                attributes: vec![],
                visibility: Visibility::Public,
                modifiers: vec![MethodModifier::Static],
                return_type: public_return_type,
                name: function_name,
                parameters: public_params,
                body: MethodBody::Block(body_lines),
            };

            vec![dll_import, wrapper]
        } else {
            // Simple extern method
            let return_type = self.rust_type_to_csharp_type(function.return_type(), type_id_map);
            vec![CSharpMethod {
                doc_lines: vec![],
                attributes: vec![format!(
                    "[DllImport(\"{}\", EntryPoint = \"{}\", CallingConvention = CallingConvention.Cdecl)]",
                    self.library_name, function.name()
                )],
                visibility: Visibility::Public,
                modifiers: vec![MethodModifier::Static, MethodModifier::Extern],
                return_type,
                name: function_name,
                parameters: raw_params,
                body: MethodBody::None,
            }]
        }
    }

    fn build_async_function(
        &self,
        function: &FunctionInfo,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> Vec<CSharpMethod> {
        let function_name = self.to_pascal_case(function.name());
        let raw_return_type = self.rust_type_to_csharp_type(function.return_type(), type_id_map);
        let registrar_class = self.get_registrar_class_name(function.return_type(), type_id_map);
        let return_repr = FFIRepr::from_type_info(function.return_type());

        // Determine public return type
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

        // Public params
        let params: Vec<CSharpParam> = function
            .parameters()
            .iter()
            .map(|param| CSharpParam {
                type_name: self.rust_type_to_csharp_type(param.ty(), type_id_map),
                name: param.name().to_lowercase(),
            })
            .collect();

        // Build async wrapper body
        let mut body_lines = Vec::new();
        body_lines.push(format!(
            "var tcs = new TaskCompletionSource<{raw_return_type}>();"
        ));
        body_lines.push(String::new());
        body_lines.push(format!(
            "var id = {registrar_class}.Instance.Register("
        ));
        body_lines.push(format!("    ({raw_return_type} res) =>"));
        body_lines.push("    {".into());
        body_lines.push("        tcs.SetResult(res);".into());
        body_lines.push("    });".into());
        body_lines.push(String::new());

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
        body_lines.push(format!(
            "{}Internal({});",
            function_name,
            param_names.join(", ")
        ));
        body_lines.push(String::new());

        // Return statement
        match return_repr {
            FFIRepr::OwnedSlice => {
                let element_type =
                    self.get_generic_element_csharp_type(function.return_type(), type_id_map);
                body_lines.push(format!(
                    "return new OwnedSliceHandle<{element_type}>(await tcs.Task);"
                ));
            }
            FFIRepr::Owned => {
                let marker_type = self.get_inner_type_name(function.return_type());
                body_lines.push(format!(
                    "return new OwnedHandle<{marker_type}>(await tcs.Task);"
                ));
            }
            _ => {
                body_lines.push("return await tcs.Task;".into());
            }
        }

        let async_wrapper = CSharpMethod {
            doc_lines: vec![],
            attributes: vec![],
            visibility: Visibility::Public,
            modifiers: vec![MethodModifier::Static, MethodModifier::Async],
            return_type: format!("Task<{public_return_type}>"),
            name: function_name.clone(),
            parameters: params.clone(),
            body: MethodBody::Block(body_lines),
        };

        // DllImport method
        let internal_params: Vec<CSharpParam> = std::iter::once(CSharpParam {
            type_name: "ulong".into(),
            name: "id".into(),
        })
        .chain(params)
        .chain(std::iter::once(CSharpParam {
            type_name: format!("{registrar_class}.CallbackDelegate"),
            name: "cb".into(),
        }))
        .collect();

        let dll_import = CSharpMethod {
            doc_lines: vec![],
            attributes: vec![format!(
                "[DllImport(\"{}\", EntryPoint = \"{}\", CallingConvention = CallingConvention.Cdecl)]",
                self.library_name, function.name()
            )],
            visibility: Visibility::Private,
            modifiers: vec![MethodModifier::Static, MethodModifier::Extern],
            return_type: "void".into(),
            name: format!("{function_name}Internal"),
            parameters: internal_params,
            body: MethodBody::None,
        };

        vec![async_wrapper, dll_import]
    }
}
