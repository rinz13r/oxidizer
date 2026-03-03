/// Intermediate representation for a generated Python file.
///
/// The IR models the structure of the output without any knowledge of the
/// source registry or Rust types.  It is built by [`super::builder`] and
/// rendered to text by [`super::renderer`].

// ---------------------------------------------------------------------------
// Top-level
// ---------------------------------------------------------------------------

pub struct PythonFile {
    /// Import lines, e.g. `"import ctypes"`, `"from typing import Callable"`
    pub imports: Vec<String>,
    /// Optional module docstring
    pub module_docstring: Option<String>,
    /// Module-level statements, e.g. `_lib = ctypes.CDLL(...)`
    pub module_statements: Vec<String>,
    /// Top-level items (classes, functions, raw code, comments)
    pub items: Vec<PythonItem>,
}

pub enum PythonItem {
    /// A `ctypes.Structure` subclass with `_fields_`
    StructClass(PythonStructClass),
    /// A regular Python class
    Class(PythonClass),
    /// A module-level function
    Function(PythonFunction),
    /// Raw code lines (e.g. `_lib.func.argtypes = [...]`)
    RawCode(Vec<String>),
    /// A comment block
    Comment(String),
}

// ---------------------------------------------------------------------------
// Struct class (ctypes.Structure)
// ---------------------------------------------------------------------------

pub struct PythonStructClass {
    pub doc_lines: Vec<String>,
    pub name: String,
    pub fields: Vec<PythonField>,
}

// ---------------------------------------------------------------------------
// Class
// ---------------------------------------------------------------------------

pub struct PythonClass {
    pub doc_lines: Vec<String>,
    pub name: String,
    /// Base classes, e.g. `["ctypes.Structure"]`
    pub bases: Vec<String>,
    /// Class-level statements (raw lines inside the class body)
    pub class_statements: Vec<String>,
    pub methods: Vec<PythonMethod>,
}

// ---------------------------------------------------------------------------
// Function
// ---------------------------------------------------------------------------

pub struct PythonFunction {
    pub doc_lines: Vec<String>,
    pub name: String,
    pub is_async: bool,
    pub parameters: Vec<PythonParam>,
    pub return_annotation: Option<String>,
    pub body: Vec<String>,
}

// ---------------------------------------------------------------------------
// Method
// ---------------------------------------------------------------------------

pub struct PythonMethod {
    pub decorators: Vec<String>,
    pub name: String,
    pub is_async: bool,
    pub parameters: Vec<PythonParam>,
    pub return_annotation: Option<String>,
    pub body: Vec<String>,
}

// ---------------------------------------------------------------------------
// Field & Param
// ---------------------------------------------------------------------------

/// A field in a ctypes.Structure `_fields_` list: `("name", ctypes_type)`
pub struct PythonField {
    pub name: String,
    pub ctypes_type: String,
}

/// A function/method parameter with optional type annotation
#[derive(Clone)]
pub struct PythonParam {
    pub name: String,
    pub annotation: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_param_clone() {
        let p = PythonParam {
            name: "x".into(),
            annotation: Some("int".into()),
        };
        let p2 = p.clone();
        assert_eq!(p2.name, "x");
        assert_eq!(p2.annotation.as_deref(), Some("int"));
    }

    #[test]
    fn test_python_item_variants() {
        let s = PythonItem::StructClass(PythonStructClass {
            doc_lines: vec![],
            name: "MyStruct".into(),
            fields: vec![],
        });
        assert!(matches!(s, PythonItem::StructClass(_)));

        let c = PythonItem::Class(PythonClass {
            doc_lines: vec![],
            name: "MyClass".into(),
            bases: vec![],
            class_statements: vec![],
            methods: vec![],
        });
        assert!(matches!(c, PythonItem::Class(_)));

        let f = PythonItem::Function(PythonFunction {
            doc_lines: vec![],
            name: "my_func".into(),
            is_async: false,
            parameters: vec![],
            return_annotation: None,
            body: vec!["pass".into()],
        });
        assert!(matches!(f, PythonItem::Function(_)));

        let r = PythonItem::RawCode(vec!["x = 1".into()]);
        assert!(matches!(r, PythonItem::RawCode(_)));

        let cmt = PythonItem::Comment("# hello".into());
        assert!(matches!(cmt, PythonItem::Comment(_)));
    }

    #[test]
    fn test_file_construction() {
        let file = PythonFile {
            imports: vec!["import ctypes".into()],
            module_docstring: Some("Auto-generated bindings.".into()),
            module_statements: vec!["_lib = ctypes.CDLL(\"libfoo.so\")".into()],
            items: vec![PythonItem::StructClass(PythonStructClass {
                doc_lines: vec![],
                name: "FFITy".into(),
                fields: vec![
                    PythonField {
                        name: "x".into(),
                        ctypes_type: "ctypes.c_uint64".into(),
                    },
                    PythonField {
                        name: "y".into(),
                        ctypes_type: "ctypes.c_uint64".into(),
                    },
                ],
            })],
        };

        assert_eq!(file.imports.len(), 1);
        assert!(file.module_docstring.is_some());
        assert_eq!(file.module_statements.len(), 1);
        assert_eq!(file.items.len(), 1);

        if let PythonItem::StructClass(s) = &file.items[0] {
            assert_eq!(s.name, "FFITy");
            assert_eq!(s.fields.len(), 2);
            assert_eq!(s.fields[0].name, "x");
            assert_eq!(s.fields[1].ctypes_type, "ctypes.c_uint64");
        } else {
            panic!("Expected StructClass");
        }
    }

    #[test]
    fn test_async_function() {
        let f = PythonFunction {
            doc_lines: vec!["Do something async.".into()],
            name: "my_async".into(),
            is_async: true,
            parameters: vec![
                PythonParam {
                    name: "x".into(),
                    annotation: Some("int".into()),
                },
            ],
            return_annotation: Some("float".into()),
            body: vec!["return await something()".into()],
        };
        assert!(f.is_async);
        assert_eq!(f.parameters.len(), 1);
        assert_eq!(f.return_annotation.as_deref(), Some("float"));
    }

    #[test]
    fn test_class_with_methods() {
        let c = PythonClass {
            doc_lines: vec!["A wrapper class.".into()],
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
                    body: vec!["self._raw = raw".into()],
                },
                PythonMethod {
                    decorators: vec!["@staticmethod".into()],
                    name: "create".into(),
                    is_async: false,
                    parameters: vec![],
                    return_annotation: Some("OwnedHandle".into()),
                    body: vec!["pass".into()],
                },
            ],
        };
        assert_eq!(c.name, "OwnedHandle");
        assert_eq!(c.methods.len(), 2);
        assert_eq!(c.methods[1].decorators, vec!["@staticmethod"]);
    }
}
