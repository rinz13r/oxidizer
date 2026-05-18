use crate::ir::*;
use crate::{
    FFI_SLICE_RAW_TYPE_ID, FFIRepr, OWNED_RAW_TYPE_ID, OWNED_SLICE_RAW_TYPE_ID, PythonGenerator,
};
use oxidizer_core::{FunctionInfo, TypeInfo, TypeKind, registry::Registry};
use std::collections::HashMap;

impl PythonGenerator {
    pub(crate) fn build_ir(&self, registry: &Registry) -> PythonFile {
        let type_id_map = self.build_type_id_map(registry);
        let mut items: Vec<PythonItem> = Vec::new();

        // 1. Infrastructure structs (OwnedRawHandle, OwnedSliceRawHandle, FFISliceRaw)
        items.extend(self.build_infrastructure_types(&type_id_map));

        // 2. SliceCallbackRaw struct (if needed)
        let cb_types = self.collect_slice_callback_types(registry.functions());
        if !cb_types.is_empty() {
            items.push(self.build_slice_callback_struct());
        }

        // 3. CFUNCTYPE declarations for async callbacks and slice callbacks
        let async_return_types = self.collect_async_return_types(registry.functions());
        for rt in &async_return_types {
            items.push(self.build_async_cfunctype(rt, &type_id_map));
        }
        for et in &cb_types {
            items.push(self.build_slice_callback_cfunctype(et, &type_id_map));
        }

        // 4. Wrapper classes (OwnedHandle, OwnedSliceHandle, SliceHandle)
        items.push(self.build_owned_handle_class());
        items.push(self.build_owned_slice_handle_class());
        items.push(self.build_slice_handle_class());

        // 5. Async registrar classes (one per unique return type)
        for rt in &async_return_types {
            items.push(self.build_async_registrar(rt, &type_id_map));
        }

        // 6. Slice callback registrar classes (one per unique element type)
        for et in &cb_types {
            items.push(self.build_slice_callback_registrar(et, &type_id_map));
        }

        // 7. User types
        for type_info in registry.types() {
            if let Some(item) = self.build_user_type(type_info, &type_id_map) {
                items.push(item);
            }
        }

        // 8. FFI declarations (_lib.func.argtypes / restype)
        for function in registry.functions() {
            items.push(self.build_ffi_declaration(function, &type_id_map));
        }

        // 9. Public wrapper functions
        for function in registry.functions() {
            items.push(self.build_wrapper_function(function, &type_id_map));
        }

        PythonFile {
            imports: vec![
                "from __future__ import annotations".into(),
                "import ctypes".into(),
                "import asyncio".into(),
                "import os".into(),
                "import threading".into(),
                "from typing import Callable".into(),
            ],
            module_docstring: self.module_docstring.clone(),
            module_statements: vec![format!(
                "_lib = ctypes.CDLL(os.path.join(os.path.dirname(__file__), \"{}\"))",
                self.library_name
            )],
            items,
        }
    }

    // ------------------------------------------------------------------
    // Infrastructure types
    // ------------------------------------------------------------------

    fn build_infrastructure_types(
        &self,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> Vec<PythonItem> {
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
    ) -> PythonItem {
        let fields = type_info
            .fields()
            .iter()
            .map(|field| PythonField {
                name: field.name().to_string(),
                ctypes_type: self.rust_type_to_python_ctypes(field.ty(), type_id_map),
            })
            .collect();

        PythonItem::StructClass(PythonStructClass {
            doc_lines: vec![format!("FFI infrastructure type: {}.", type_info.name())],
            name: type_info.name().to_string(),
            fields,
        })
    }

    // ------------------------------------------------------------------
    // SliceCallbackRaw struct
    // ------------------------------------------------------------------

    fn build_slice_callback_struct(&self) -> PythonItem {
        PythonItem::StructClass(PythonStructClass {
            doc_lines: vec!["Callback struct for scoped slice access.".into()],
            name: "SliceCallbackRaw".into(),
            fields: vec![
                PythonField {
                    name: "id".into(),
                    ctypes_type: "ctypes.c_uint64".into(),
                },
                PythonField {
                    name: "func".into(),
                    ctypes_type: "ctypes.c_void_p".into(),
                },
            ],
        })
    }

    // ------------------------------------------------------------------
    // CFUNCTYPE declarations
    // ------------------------------------------------------------------

    fn build_async_cfunctype(
        &self,
        return_type: &TypeInfo,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> PythonItem {
        let python_type = self.rust_type_to_python_ctypes(return_type, type_id_map);
        let name = self.get_callback_type_name(return_type, type_id_map);
        PythonItem::RawCode(vec![format!(
            "{name} = ctypes.CFUNCTYPE(None, ctypes.c_uint64, {python_type})"
        )])
    }

    fn build_slice_callback_cfunctype(
        &self,
        _element_type: &TypeInfo,
        _type_id_map: &HashMap<String, TypeInfo>,
    ) -> PythonItem {
        // Slice callbacks receive (id: u64, slice: FFISliceRaw)
        PythonItem::RawCode(vec![
            "_SliceCallbackCFUNC = ctypes.CFUNCTYPE(None, ctypes.c_uint64, FFISliceRaw)".into(),
        ])
    }

    // ------------------------------------------------------------------
    // OwnedHandle wrapper class
    // ------------------------------------------------------------------

    fn build_owned_handle_class(&self) -> PythonItem {
        PythonItem::Class(PythonClass {
            doc_lines: vec![
                "Type-safe wrapper for owned Rust objects.".into(),
                "Implements context manager protocol for resource cleanup.".into(),
            ],
            name: "OwnedHandle".into(),
            bases: vec![],
            class_statements: vec![],
            methods: vec![
                PythonMethod {
                    decorators: vec![],
                    name: "__init__".into(),
                    is_async: false,
                    parameters: vec![
                        PythonParam {
                            name: "self".into(),
                            annotation: None,
                        },
                        PythonParam {
                            name: "raw".into(),
                            annotation: Some("OwnedRawHandle".into()),
                        },
                    ],
                    return_annotation: None,
                    body: vec!["self._raw = raw".into(), "self._disposed = False".into()],
                },
                PythonMethod {
                    decorators: vec![],
                    name: "dispose".into(),
                    is_async: false,
                    parameters: vec![PythonParam {
                        name: "self".into(),
                        annotation: None,
                    }],
                    return_annotation: None,
                    body: vec![
                        "if self._disposed:".into(),
                        "    return".into(),
                        "self._disposed = True".into(),
                        "if self._raw.ptr:".into(),
                        "    _lib.drop_owned(self._raw)".into(),
                        "    self._raw.ptr = None".into(),
                    ],
                },
                PythonMethod {
                    decorators: vec![],
                    name: "__del__".into(),
                    is_async: false,
                    parameters: vec![PythonParam {
                        name: "self".into(),
                        annotation: None,
                    }],
                    return_annotation: None,
                    body: vec!["self.dispose()".into()],
                },
                PythonMethod {
                    decorators: vec![],
                    name: "__enter__".into(),
                    is_async: false,
                    parameters: vec![PythonParam {
                        name: "self".into(),
                        annotation: None,
                    }],
                    return_annotation: None,
                    body: vec!["return self".into()],
                },
                PythonMethod {
                    decorators: vec![],
                    name: "__exit__".into(),
                    is_async: false,
                    parameters: vec![
                        PythonParam {
                            name: "self".into(),
                            annotation: None,
                        },
                        PythonParam {
                            name: "*args".into(),
                            annotation: None,
                        },
                    ],
                    return_annotation: None,
                    body: vec!["self.dispose()".into()],
                },
            ],
        })
    }

    // ------------------------------------------------------------------
    // OwnedSliceHandle wrapper class
    // ------------------------------------------------------------------

    fn build_owned_slice_handle_class(&self) -> PythonItem {
        PythonItem::Class(PythonClass {
            doc_lines: vec![
                "Owned array transferred from Rust.".into(),
                "Implements context manager protocol for resource cleanup.".into(),
            ],
            name: "OwnedSliceHandle".into(),
            bases: vec![],
            class_statements: vec![],
            methods: vec![
                PythonMethod {
                    decorators: vec![],
                    name: "__init__".into(),
                    is_async: false,
                    parameters: vec![
                        PythonParam {
                            name: "self".into(),
                            annotation: None,
                        },
                        PythonParam {
                            name: "raw".into(),
                            annotation: Some("OwnedSliceRawHandle".into()),
                        },
                        PythonParam {
                            name: "element_type".into(),
                            annotation: None,
                        },
                    ],
                    return_annotation: None,
                    body: vec![
                        "self._raw = raw".into(),
                        "self._element_type = element_type".into(),
                        "self._disposed = False".into(),
                    ],
                },
                PythonMethod {
                    decorators: vec![],
                    name: "__len__".into(),
                    is_async: false,
                    parameters: vec![PythonParam {
                        name: "self".into(),
                        annotation: None,
                    }],
                    return_annotation: Some("int".into()),
                    body: vec!["return self._raw.len".into()],
                },
                PythonMethod {
                    decorators: vec![],
                    name: "__getitem__".into(),
                    is_async: false,
                    parameters: vec![
                        PythonParam {
                            name: "self".into(),
                            annotation: None,
                        },
                        PythonParam {
                            name: "index".into(),
                            annotation: Some("int".into()),
                        },
                    ],
                    return_annotation: None,
                    body: vec![
                        "if self._disposed:".into(),
                        "    raise RuntimeError(\"OwnedSliceHandle has been disposed\")".into(),
                        "if index < 0 or index >= self._raw.len:".into(),
                        "    raise IndexError(\"index out of range\")".into(),
                        "arr = ctypes.cast(self._raw.ptr, ctypes.POINTER(self._element_type))"
                            .into(),
                        "return arr[index]".into(),
                    ],
                },
                PythonMethod {
                    decorators: vec![],
                    name: "to_list".into(),
                    is_async: false,
                    parameters: vec![PythonParam {
                        name: "self".into(),
                        annotation: None,
                    }],
                    return_annotation: Some("list".into()),
                    body: vec!["return [self[i] for i in range(len(self))]".into()],
                },
                PythonMethod {
                    decorators: vec![],
                    name: "dispose".into(),
                    is_async: false,
                    parameters: vec![PythonParam {
                        name: "self".into(),
                        annotation: None,
                    }],
                    return_annotation: None,
                    body: vec![
                        "if self._disposed:".into(),
                        "    return".into(),
                        "self._disposed = True".into(),
                        "if self._raw.ptr:".into(),
                        "    _lib.drop_owned_slice(self._raw)".into(),
                        "    self._raw.ptr = None".into(),
                    ],
                },
                PythonMethod {
                    decorators: vec![],
                    name: "__del__".into(),
                    is_async: false,
                    parameters: vec![PythonParam {
                        name: "self".into(),
                        annotation: None,
                    }],
                    return_annotation: None,
                    body: vec!["self.dispose()".into()],
                },
                PythonMethod {
                    decorators: vec![],
                    name: "__enter__".into(),
                    is_async: false,
                    parameters: vec![PythonParam {
                        name: "self".into(),
                        annotation: None,
                    }],
                    return_annotation: None,
                    body: vec!["return self".into()],
                },
                PythonMethod {
                    decorators: vec![],
                    name: "__exit__".into(),
                    is_async: false,
                    parameters: vec![
                        PythonParam {
                            name: "self".into(),
                            annotation: None,
                        },
                        PythonParam {
                            name: "*args".into(),
                            annotation: None,
                        },
                    ],
                    return_annotation: None,
                    body: vec!["self.dispose()".into()],
                },
            ],
        })
    }

    // ------------------------------------------------------------------
    // SliceHandle (read-only borrowed)
    // ------------------------------------------------------------------

    fn build_slice_handle_class(&self) -> PythonItem {
        PythonItem::Class(PythonClass {
            doc_lines: vec!["Read-only view into a borrowed Rust slice.".into()],
            name: "SliceHandle".into(),
            bases: vec![],
            class_statements: vec![],
            methods: vec![
                PythonMethod {
                    decorators: vec![],
                    name: "__init__".into(),
                    is_async: false,
                    parameters: vec![
                        PythonParam {
                            name: "self".into(),
                            annotation: None,
                        },
                        PythonParam {
                            name: "raw".into(),
                            annotation: Some("FFISliceRaw".into()),
                        },
                        PythonParam {
                            name: "element_type".into(),
                            annotation: None,
                        },
                    ],
                    return_annotation: None,
                    body: vec![
                        "self._raw = raw".into(),
                        "self._element_type = element_type".into(),
                    ],
                },
                PythonMethod {
                    decorators: vec![],
                    name: "__len__".into(),
                    is_async: false,
                    parameters: vec![PythonParam {
                        name: "self".into(),
                        annotation: None,
                    }],
                    return_annotation: Some("int".into()),
                    body: vec!["return self._raw.len".into()],
                },
                PythonMethod {
                    decorators: vec![],
                    name: "__getitem__".into(),
                    is_async: false,
                    parameters: vec![
                        PythonParam {
                            name: "self".into(),
                            annotation: None,
                        },
                        PythonParam {
                            name: "index".into(),
                            annotation: Some("int".into()),
                        },
                    ],
                    return_annotation: None,
                    body: vec![
                        "if index < 0 or index >= self._raw.len:".into(),
                        "    raise IndexError(\"index out of range\")".into(),
                        "arr = ctypes.cast(self._raw.ptr, ctypes.POINTER(self._element_type))"
                            .into(),
                        "return arr[index]".into(),
                    ],
                },
                PythonMethod {
                    decorators: vec![],
                    name: "to_list".into(),
                    is_async: false,
                    parameters: vec![PythonParam {
                        name: "self".into(),
                        annotation: None,
                    }],
                    return_annotation: Some("list".into()),
                    body: vec!["return [self[i] for i in range(len(self))]".into()],
                },
            ],
        })
    }

    // ------------------------------------------------------------------
    // Async registrar class
    // ------------------------------------------------------------------

    fn build_async_registrar(
        &self,
        return_type: &TypeInfo,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> PythonItem {
        let class_name = self.get_registrar_class_name(return_type, type_id_map);
        let callback_type_name = self.get_callback_type_name(return_type, type_id_map);

        PythonItem::Class(PythonClass {
            doc_lines: vec![],
            name: class_name.clone(),
            bases: vec![],
            class_statements: vec!["_instance = None".into()],
            methods: vec![
                // instance() classmethod
                PythonMethod {
                    decorators: vec!["@classmethod".into()],
                    name: "instance".into(),
                    is_async: false,
                    parameters: vec![PythonParam {
                        name: "cls".into(),
                        annotation: None,
                    }],
                    return_annotation: None,
                    body: vec![
                        "if cls._instance is None:".into(),
                        "    cls._instance = cls()".into(),
                        "return cls._instance".into(),
                    ],
                },
                // __init__
                PythonMethod {
                    decorators: vec![],
                    name: "__init__".into(),
                    is_async: false,
                    parameters: vec![PythonParam {
                        name: "self".into(),
                        annotation: None,
                    }],
                    return_annotation: None,
                    body: vec![
                        "self._registrations = {}".into(),
                        "self._next_id = 0".into(),
                        "self._lock = threading.Lock()".into(),
                        format!("self._callback_ref = {callback_type_name}(self._callback)"),
                    ],
                },
                // register(loop) -> (id, future)
                PythonMethod {
                    decorators: vec![],
                    name: "register".into(),
                    is_async: false,
                    parameters: vec![
                        PythonParam {
                            name: "self".into(),
                            annotation: None,
                        },
                        PythonParam {
                            name: "loop".into(),
                            annotation: None,
                        },
                    ],
                    return_annotation: None,
                    body: vec![
                        "future = loop.create_future()".into(),
                        "with self._lock:".into(),
                        "    cb_id = self._next_id".into(),
                        "    self._next_id += 1".into(),
                        "    self._registrations[cb_id] = (loop, future)".into(),
                        "return cb_id, future".into(),
                    ],
                },
                // _callback (static, called from Rust)
                PythonMethod {
                    decorators: vec!["@staticmethod".into()],
                    name: "_callback".into(),
                    is_async: false,
                    parameters: vec![
                        PythonParam {
                            name: "cb_id".into(),
                            annotation: None,
                        },
                        PythonParam {
                            name: "result".into(),
                            annotation: None,
                        },
                    ],
                    return_annotation: None,
                    body: vec![
                        format!("inst = {class_name}.instance()"),
                        "with inst._lock:".into(),
                        "    loop, future = inst._registrations.pop(cb_id)".into(),
                        "loop.call_soon_threadsafe(future.set_result, result)".into(),
                    ],
                },
                // callback property (returns cached CFUNCTYPE instance)
                PythonMethod {
                    decorators: vec!["@property".into()],
                    name: "callback".into(),
                    is_async: false,
                    parameters: vec![PythonParam {
                        name: "self".into(),
                        annotation: None,
                    }],
                    return_annotation: None,
                    body: vec!["return self._callback_ref".into()],
                },
            ],
        })
    }

    // ------------------------------------------------------------------
    // Slice callback registrar class
    // ------------------------------------------------------------------

    fn build_slice_callback_registrar(
        &self,
        element_type: &TypeInfo,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> PythonItem {
        let csharp_element_type = self.rust_type_to_python_ctypes(element_type, type_id_map);
        let class_name = format!(
            "SliceCallbackRegistrar_{}",
            self.rust_type_to_python_name(element_type, type_id_map)
        );

        PythonItem::Class(PythonClass {
            doc_lines: vec![],
            name: class_name.clone(),
            bases: vec![],
            class_statements: vec!["_instance = None".into()],
            methods: vec![
                PythonMethod {
                    decorators: vec!["@classmethod".into()],
                    name: "instance".into(),
                    is_async: false,
                    parameters: vec![PythonParam {
                        name: "cls".into(),
                        annotation: None,
                    }],
                    return_annotation: None,
                    body: vec![
                        "if cls._instance is None:".into(),
                        "    cls._instance = cls()".into(),
                        "return cls._instance".into(),
                    ],
                },
                PythonMethod {
                    decorators: vec![],
                    name: "__init__".into(),
                    is_async: false,
                    parameters: vec![PythonParam {
                        name: "self".into(),
                        annotation: None,
                    }],
                    return_annotation: None,
                    body: vec![
                        "self._registrations = {}".into(),
                        "self._next_id = 0".into(),
                        "self._lock = threading.Lock()".into(),
                        "self._callback_ref = _SliceCallbackCFUNC(self._callback)".into(),
                    ],
                },
                PythonMethod {
                    decorators: vec![],
                    name: "register".into(),
                    is_async: false,
                    parameters: vec![
                        PythonParam {
                            name: "self".into(),
                            annotation: None,
                        },
                        PythonParam {
                            name: "callback".into(),
                            annotation: Some("Callable".into()),
                        },
                    ],
                    return_annotation: Some("int".into()),
                    body: vec![
                        "with self._lock:".into(),
                        "    cb_id = self._next_id".into(),
                        "    self._next_id += 1".into(),
                        "    self._registrations[cb_id] = callback".into(),
                        "return cb_id".into(),
                    ],
                },
                PythonMethod {
                    decorators: vec!["@staticmethod".into()],
                    name: "_callback".into(),
                    is_async: false,
                    parameters: vec![
                        PythonParam {
                            name: "cb_id".into(),
                            annotation: None,
                        },
                        PythonParam {
                            name: "slice_raw".into(),
                            annotation: None,
                        },
                    ],
                    return_annotation: None,
                    body: vec![
                        format!("inst = {class_name}.instance()"),
                        "with inst._lock:".into(),
                        "    callback = inst._registrations.pop(cb_id)".into(),
                        format!(
                            "arr = ctypes.cast(slice_raw.ptr, ctypes.POINTER({csharp_element_type}))"
                        ),
                        "data = [arr[i] for i in range(slice_raw.len)]".into(),
                        "callback(data)".into(),
                    ],
                },
                PythonMethod {
                    decorators: vec!["@property".into()],
                    name: "callback_func".into(),
                    is_async: false,
                    parameters: vec![PythonParam {
                        name: "self".into(),
                        annotation: None,
                    }],
                    return_annotation: None,
                    body: vec!["return self._callback_ref".into()],
                },
            ],
        })
    }

    // ------------------------------------------------------------------
    // User types
    // ------------------------------------------------------------------

    fn build_user_type(
        &self,
        type_info: &TypeInfo,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> Option<PythonItem> {
        match FFIRepr::from_type_info(type_info) {
            FFIRepr::Owned => Some(self.build_marker_class(type_info)),
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

    fn build_marker_class(&self, type_info: &TypeInfo) -> PythonItem {
        let marker_name = self.get_inner_type_name(type_info);
        PythonItem::Class(PythonClass {
            doc_lines: vec![format!(
                "Marker class for heap-allocated {marker_name} instances."
            )],
            name: marker_name,
            bases: vec![],
            class_statements: vec![],
            methods: vec![],
        })
    }

    fn build_user_struct(
        &self,
        type_info: &TypeInfo,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> PythonItem {
        let fields = type_info
            .fields()
            .iter()
            .map(|field| PythonField {
                name: field.name().to_string(),
                ctypes_type: self.rust_type_to_python_ctypes(field.ty(), type_id_map),
            })
            .collect();

        PythonItem::StructClass(PythonStructClass {
            doc_lines: vec![],
            name: self.rust_type_to_python_name(type_info, type_id_map),
            fields,
        })
    }

    // ------------------------------------------------------------------
    // FFI declarations
    // ------------------------------------------------------------------

    fn build_ffi_declaration(
        &self,
        function: &FunctionInfo,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> PythonItem {
        let fname = function.name();
        let mut lines = Vec::new();

        if *function.is_async() {
            // Async functions: (id: u64, ...params, callback: CFUNCTYPE)
            let mut argtypes = vec!["ctypes.c_uint64".to_string()];
            for param in function.parameters() {
                argtypes.push(self.rust_type_to_python_ctypes(param.ty(), type_id_map));
            }
            let callback_type = self.get_callback_type_name(function.return_type(), type_id_map);
            argtypes.push(callback_type);
            lines.push(format!("_lib.{fname}.argtypes = [{}]", argtypes.join(", ")));
            lines.push(format!("_lib.{fname}.restype = None"));
        } else {
            // Sync functions
            let argtypes: Vec<String> = function
                .parameters()
                .iter()
                .map(|p| self.rust_type_to_python_ctypes(p.ty(), type_id_map))
                .collect();
            if !argtypes.is_empty() {
                lines.push(format!("_lib.{fname}.argtypes = [{}]", argtypes.join(", ")));
            }
            let restype = self.rust_type_to_python_ctypes(function.return_type(), type_id_map);
            lines.push(format!("_lib.{fname}.restype = {restype}"));
        }

        PythonItem::RawCode(lines)
    }

    // ------------------------------------------------------------------
    // Wrapper functions
    // ------------------------------------------------------------------

    fn build_wrapper_function(
        &self,
        function: &FunctionInfo,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> PythonItem {
        if *function.is_async() {
            self.build_async_wrapper(function, type_id_map)
        } else {
            self.build_sync_wrapper(function, type_id_map)
        }
    }

    fn build_sync_wrapper(
        &self,
        function: &FunctionInfo,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> PythonItem {
        let fname = function.name();
        let return_repr = FFIRepr::from_type_info(function.return_type());
        let has_owned_params = function
            .parameters()
            .iter()
            .any(|p| FFIRepr::from_type_info(p.ty()) == FFIRepr::Owned);
        let has_slice_callback_params = function
            .parameters()
            .iter()
            .any(|p| FFIRepr::from_type_info(p.ty()) == FFIRepr::SliceCallback);

        // Build parameters
        let params: Vec<PythonParam> = function
            .parameters()
            .iter()
            .map(|param| {
                let annotation = self.python_public_type_annotation(param.ty(), type_id_map);
                PythonParam {
                    name: param.name().to_string(),
                    annotation,
                }
            })
            .collect();

        // Build return annotation
        let return_annotation =
            self.python_public_return_annotation(function.return_type(), type_id_map);

        let needs_wrapper = matches!(return_repr, FFIRepr::Owned | FFIRepr::OwnedSlice)
            || has_owned_params
            || has_slice_callback_params;

        let mut body = Vec::new();

        if needs_wrapper {
            // Slice callback registration
            for param in function.parameters() {
                if FFIRepr::from_type_info(param.ty()) == FFIRepr::SliceCallback {
                    let pname = param.name();
                    let element_type_name =
                        self.get_generic_element_python_name(param.ty(), type_id_map);
                    let registrar = format!("SliceCallbackRegistrar_{element_type_name}");
                    body.push(format!(
                        "{pname}_id = {registrar}.instance().register({pname})"
                    ));
                    body.push(format!(
                        "{pname}_raw = SliceCallbackRaw(id={pname}_id, func=ctypes.cast({registrar}.instance().callback_func, ctypes.c_void_p).value)"
                    ));
                }
            }

            // Build call arguments
            let call_args: Vec<String> = function
                .parameters()
                .iter()
                .map(|param| {
                    let pname = param.name().to_string();
                    match FFIRepr::from_type_info(param.ty()) {
                        FFIRepr::Owned => format!("{pname}._raw"),
                        FFIRepr::SliceCallback => format!("{pname}_raw"),
                        _ => pname,
                    }
                })
                .collect();
            let args_str = call_args.join(", ");

            match return_repr {
                FFIRepr::Owned => {
                    body.push(format!("return OwnedHandle(_lib.{fname}({args_str}))"));
                }
                FFIRepr::OwnedSlice => {
                    let element_ctypes_type =
                        self.get_generic_element_python_ctypes(function.return_type(), type_id_map);
                    body.push(format!(
                        "return OwnedSliceHandle(_lib.{fname}({args_str}), {element_ctypes_type})"
                    ));
                }
                _ => {
                    if matches!(function.return_type().kind(), TypeKind::Void) {
                        body.push(format!("_lib.{fname}({args_str})"));
                    } else {
                        body.push(format!("return _lib.{fname}({args_str})"));
                    }
                }
            }
        } else {
            // Simple passthrough
            let call_args: Vec<String> = function
                .parameters()
                .iter()
                .map(|p| p.name().to_string())
                .collect();
            let args_str = call_args.join(", ");

            if matches!(function.return_type().kind(), TypeKind::Void) {
                body.push(format!("_lib.{fname}({args_str})"));
            } else {
                body.push(format!("return _lib.{fname}({args_str})"));
            }
        }

        PythonItem::Function(PythonFunction {
            doc_lines: vec![],
            name: fname.to_string(),
            is_async: false,
            parameters: params,
            return_annotation,
            body,
        })
    }

    fn build_async_wrapper(
        &self,
        function: &FunctionInfo,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> PythonItem {
        let fname = function.name();
        let registrar_class = self.get_registrar_class_name(function.return_type(), type_id_map);
        let return_repr = FFIRepr::from_type_info(function.return_type());

        // Public params (excluding the async callback)
        let params: Vec<PythonParam> = function
            .parameters()
            .iter()
            .map(|param| {
                let annotation = self.python_public_type_annotation(param.ty(), type_id_map);
                PythonParam {
                    name: param.name().to_string(),
                    annotation,
                }
            })
            .collect();

        let return_annotation =
            self.python_public_return_annotation(function.return_type(), type_id_map);

        let mut body = Vec::new();
        body.push("loop = asyncio.get_running_loop()".into());
        body.push(format!("registrar = {registrar_class}.instance()"));
        body.push("cb_id, future = registrar.register(loop)".into());

        // Call FFI: _lib.func(cb_id, ...params, registrar.callback)
        let param_names: Vec<String> = function
            .parameters()
            .iter()
            .map(|p| match FFIRepr::from_type_info(p.ty()) {
                FFIRepr::Owned => format!("{}._raw", p.name()),
                _ => p.name().to_string(),
            })
            .collect();
        let all_args = std::iter::once("cb_id".to_string())
            .chain(param_names)
            .chain(std::iter::once("registrar.callback".to_string()))
            .collect::<Vec<_>>()
            .join(", ");
        body.push(format!("_lib.{fname}({all_args})"));

        // Await and wrap result
        match return_repr {
            FFIRepr::Owned => {
                body.push("return OwnedHandle(await future)".into());
            }
            FFIRepr::OwnedSlice => {
                let element_ctypes_type =
                    self.get_generic_element_python_ctypes(function.return_type(), type_id_map);
                body.push(format!(
                    "return OwnedSliceHandle(await future, {element_ctypes_type})"
                ));
            }
            _ => {
                body.push("return await future".into());
            }
        }

        PythonItem::Function(PythonFunction {
            doc_lines: vec![],
            name: fname.to_string(),
            is_async: true,
            parameters: params,
            return_annotation,
            body,
        })
    }

    // ------------------------------------------------------------------
    // Annotation helpers
    // ------------------------------------------------------------------

    fn python_public_type_annotation(
        &self,
        type_info: &TypeInfo,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> Option<String> {
        match FFIRepr::from_type_info(type_info) {
            FFIRepr::Owned => Some("OwnedHandle".into()),
            FFIRepr::OwnedSlice => Some("OwnedSliceHandle".into()),
            FFIRepr::Slice | FFIRepr::SliceMut => Some("SliceHandle".into()),
            FFIRepr::SliceCallback => Some("Callable".into()),
            FFIRepr::Direct => Some(self.rust_type_to_python_annotation(type_info, type_id_map)),
        }
    }

    fn python_public_return_annotation(
        &self,
        type_info: &TypeInfo,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> Option<String> {
        match FFIRepr::from_type_info(type_info) {
            FFIRepr::Owned => Some("OwnedHandle".into()),
            FFIRepr::OwnedSlice => Some("OwnedSliceHandle".into()),
            _ => {
                if matches!(type_info.kind(), TypeKind::Void) {
                    None
                } else {
                    Some(self.rust_type_to_python_annotation(type_info, type_id_map))
                }
            }
        }
    }

    /// Python type annotation for public API (e.g. `int`, `float`, struct name)
    fn rust_type_to_python_annotation(
        &self,
        type_info: &TypeInfo,
        _type_id_map: &HashMap<String, TypeInfo>,
    ) -> String {
        match type_info.kind() {
            TypeKind::U8
            | TypeKind::U16
            | TypeKind::U32
            | TypeKind::U64
            | TypeKind::I8
            | TypeKind::I16
            | TypeKind::I32
            | TypeKind::I64 => "int".into(),
            TypeKind::F32 | TypeKind::F64 => "float".into(),
            TypeKind::Bool => "bool".into(),
            TypeKind::Void => "None".into(),
            TypeKind::Pointer => "int".into(),
            TypeKind::Struct => type_info.name().to_string(),
        }
    }

    /// Get the element's Python ctypes type from generic_params
    fn get_generic_element_python_ctypes(
        &self,
        type_info: &TypeInfo,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> String {
        type_info
            .generic_params()
            .first()
            .map(|inner| self.rust_type_to_python_ctypes(inner, type_id_map))
            .unwrap_or_else(|| "ctypes.c_void_p".to_string())
    }

    /// Get the element's Python name from generic_params
    fn get_generic_element_python_name(
        &self,
        type_info: &TypeInfo,
        type_id_map: &HashMap<String, TypeInfo>,
    ) -> String {
        type_info
            .generic_params()
            .first()
            .map(|inner| self.rust_type_to_python_name(inner, type_id_map))
            .unwrap_or_else(|| "void_p".to_string())
    }
}
