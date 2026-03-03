/// Intermediate representation for a generated C# file.
///
/// The IR models the structure of the output without any knowledge of the
/// source registry or Rust types.  It is built by [`super::builder`] and
/// rendered to text by [`super::renderer`].

// ---------------------------------------------------------------------------
// Top-level
// ---------------------------------------------------------------------------

pub struct CSharpFile {
    pub usings: Vec<String>,
    pub namespace: Option<String>,
    pub items: Vec<CSharpItem>,
}

pub enum CSharpItem {
    Struct(CSharpStruct),
    Class(CSharpClass),
    StaticClass(CSharpStaticClass),
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

pub struct CSharpStruct {
    pub doc_lines: Vec<String>,
    /// e.g. `[StructLayout(LayoutKind.Sequential)]`
    pub attributes: Vec<String>,
    pub visibility: Visibility,
    /// `true` → `readonly ref struct`, `false` → `struct`
    pub is_ref_struct: bool,
    /// Includes generic params, e.g. `"ReadOnlySliceHandle<T>"`
    pub name: String,
    /// e.g. `"where T : unmanaged"`
    pub constraints: Option<String>,
    pub fields: Vec<CSharpField>,
    pub properties: Vec<CSharpProperty>,
    pub constructors: Vec<CSharpMethod>,
    pub methods: Vec<CSharpMethod>,
    pub indexers: Vec<CSharpIndexer>,
}

// ---------------------------------------------------------------------------
// Classes
// ---------------------------------------------------------------------------

pub struct CSharpClass {
    pub doc_lines: Vec<String>,
    pub visibility: Visibility,
    /// e.g. `["sealed"]`
    pub modifiers: Vec<String>,
    pub name: String,
    pub constraints: Option<String>,
    /// e.g. `["IDisposable"]`
    pub implements: Vec<String>,
    pub fields: Vec<CSharpField>,
    pub properties: Vec<CSharpProperty>,
    pub constructors: Vec<CSharpMethod>,
    /// Optional finalizer body lines. Rendered as `~ClassName() { ... }`
    pub finalizer: Option<Vec<String>>,
    pub methods: Vec<CSharpMethod>,
    pub delegates: Vec<CSharpDelegate>,
    pub indexers: Vec<CSharpIndexer>,
}

// ---------------------------------------------------------------------------
// Static class (the Bindings class)
// ---------------------------------------------------------------------------

pub struct CSharpStaticClass {
    pub visibility: Visibility,
    pub name: String,
    pub methods: Vec<CSharpMethod>,
}

// ---------------------------------------------------------------------------
// Members
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
pub enum Visibility {
    Public,
    Private,
    Internal,
    Default,
}

pub struct CSharpField {
    pub visibility: Visibility,
    pub is_static: bool,
    pub is_readonly: bool,
    pub type_name: String,
    pub name: String,
    pub initializer: Option<String>,
}

pub struct CSharpProperty {
    pub visibility: Visibility,
    pub is_static: bool,
    pub type_name: String,
    pub name: String,
    pub body: PropertyBody,
}

pub enum PropertyBody {
    /// `=> expr;`
    Expression(String),
    /// `{ get { lines } }`
    GetterBody(Vec<String>),
}

pub struct CSharpMethod {
    pub doc_lines: Vec<String>,
    /// e.g. `["[DllImport(...)]"]`
    pub attributes: Vec<String>,
    pub visibility: Visibility,
    pub modifiers: Vec<MethodModifier>,
    pub return_type: String,
    pub name: String,
    pub parameters: Vec<CSharpParam>,
    /// `None` → extern (no body / expression-bodied), `Some` → `{ lines }`
    pub body: MethodBody,
}

pub enum MethodBody {
    /// No body — used for `extern` methods: `;`
    None,
    /// Expression-bodied: `=> expr;`
    Expression(String),
    /// Block body: `{ lines }`
    Block(Vec<String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MethodModifier {
    Static,
    Async,
    Extern,
    Unsafe,
}

#[derive(Clone)]
pub struct CSharpParam {
    pub type_name: String,
    pub name: String,
}

pub struct CSharpDelegate {
    pub visibility: Visibility,
    pub return_type: String,
    pub name: String,
    pub parameters: Vec<CSharpParam>,
}

pub struct CSharpIndexer {
    pub visibility: Visibility,
    pub type_name: String,
    pub parameter: CSharpParam,
    pub getter_body: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // Derived trait tests
    // ------------------------------------------------------------------

    #[test]
    fn test_visibility_is_copy() {
        let v = Visibility::Public;
        let v2 = v; // Copy
        assert!(matches!(v, Visibility::Public));
        assert!(matches!(v2, Visibility::Public));
    }

    #[test]
    fn test_method_modifier_eq() {
        assert_eq!(MethodModifier::Static, MethodModifier::Static);
        assert_ne!(MethodModifier::Static, MethodModifier::Async);
        assert_ne!(MethodModifier::Extern, MethodModifier::Unsafe);
        assert_eq!(MethodModifier::Unsafe, MethodModifier::Unsafe);
    }

    #[test]
    fn test_csharp_param_clone() {
        let p = CSharpParam {
            type_name: "int".into(),
            name: "x".into(),
        };
        let p2 = p.clone();
        assert_eq!(p2.type_name, "int");
        assert_eq!(p2.name, "x");
    }

    // ------------------------------------------------------------------
    // Enum variant construction
    // ------------------------------------------------------------------

    #[test]
    fn test_csharp_item_variants() {
        let s = CSharpItem::Struct(CSharpStruct {
            doc_lines: vec![],
            attributes: vec![],
            visibility: Visibility::Public,
            is_ref_struct: false,
            name: "S".into(),
            constraints: None,
            fields: vec![],
            properties: vec![],
            constructors: vec![],
            methods: vec![],
            indexers: vec![],
        });
        assert!(matches!(s, CSharpItem::Struct(_)));

        let c = CSharpItem::Class(CSharpClass {
            doc_lines: vec![],
            visibility: Visibility::Public,
            modifiers: vec![],
            name: "C".into(),
            constraints: None,
            implements: vec![],
            fields: vec![],
            properties: vec![],
            constructors: vec![],
            finalizer: None,
            methods: vec![],
            delegates: vec![],
            indexers: vec![],
        });
        assert!(matches!(c, CSharpItem::Class(_)));

        let sc = CSharpItem::StaticClass(CSharpStaticClass {
            visibility: Visibility::Public,
            name: "SC".into(),
            methods: vec![],
        });
        assert!(matches!(sc, CSharpItem::StaticClass(_)));
    }

    #[test]
    fn test_method_body_variants() {
        let none = MethodBody::None;
        assert!(matches!(none, MethodBody::None));

        let expr = MethodBody::Expression("42".into());
        assert!(matches!(expr, MethodBody::Expression(_)));

        let block = MethodBody::Block(vec!["return;".into()]);
        assert!(matches!(block, MethodBody::Block(_)));
        if let MethodBody::Block(lines) = block {
            assert_eq!(lines.len(), 1);
            assert_eq!(lines[0], "return;");
        }
    }

    #[test]
    fn test_property_body_variants() {
        let expr = PropertyBody::Expression("_x".into());
        assert!(matches!(expr, PropertyBody::Expression(_)));

        let getter = PropertyBody::GetterBody(vec!["return _x;".into()]);
        assert!(matches!(getter, PropertyBody::GetterBody(_)));
        if let PropertyBody::GetterBody(lines) = getter {
            assert_eq!(lines, vec!["return _x;"]);
        }
    }

    // ------------------------------------------------------------------
    // Full file construction
    // ------------------------------------------------------------------

    #[test]
    fn test_file_construction_with_all_member_types() {
        let file = CSharpFile {
            usings: vec!["System".into(), "System.Runtime.InteropServices".into()],
            namespace: Some("Test.NS".into()),
            items: vec![CSharpItem::Class(CSharpClass {
                doc_lines: vec!["/// doc".into()],
                visibility: Visibility::Public,
                modifiers: vec!["sealed".into()],
                name: "Full".into(),
                constraints: Some("where T : new()".into()),
                implements: vec!["IDisposable".into()],
                fields: vec![CSharpField {
                    visibility: Visibility::Private,
                    is_static: false,
                    is_readonly: true,
                    type_name: "int".into(),
                    name: "_val".into(),
                    initializer: Some("0".into()),
                }],
                properties: vec![CSharpProperty {
                    visibility: Visibility::Public,
                    is_static: false,
                    type_name: "int".into(),
                    name: "Val".into(),
                    body: PropertyBody::Expression("_val".into()),
                }],
                constructors: vec![CSharpMethod {
                    doc_lines: vec![],
                    attributes: vec![],
                    visibility: Visibility::Public,
                    modifiers: vec![],
                    return_type: String::new(),
                    name: "Full".into(),
                    parameters: vec![CSharpParam {
                        type_name: "int".into(),
                        name: "v".into(),
                    }],
                    body: MethodBody::Expression("_val = v".into()),
                }],
                finalizer: None,
                methods: vec![CSharpMethod {
                    doc_lines: vec![],
                    attributes: vec!["[Obsolete]".into()],
                    visibility: Visibility::Public,
                    modifiers: vec![MethodModifier::Static],
                    return_type: "void".into(),
                    name: "Noop".into(),
                    parameters: vec![],
                    body: MethodBody::Block(vec![]),
                }],
                delegates: vec![CSharpDelegate {
                    visibility: Visibility::Public,
                    return_type: "void".into(),
                    name: "Callback".into(),
                    parameters: vec![CSharpParam {
                        type_name: "int".into(),
                        name: "result".into(),
                    }],
                }],
                indexers: vec![CSharpIndexer {
                    visibility: Visibility::Public,
                    type_name: "int".into(),
                    parameter: CSharpParam {
                        type_name: "int".into(),
                        name: "i".into(),
                    },
                    getter_body: vec!["return _val;".into()],
                }],
            })],
        };

        assert_eq!(file.usings.len(), 2);
        assert_eq!(file.namespace.as_deref(), Some("Test.NS"));
        assert_eq!(file.items.len(), 1);

        if let CSharpItem::Class(c) = &file.items[0] {
            assert_eq!(c.name, "Full");
            assert_eq!(c.modifiers, vec!["sealed"]);
            assert_eq!(c.implements, vec!["IDisposable"]);
            assert_eq!(c.constraints.as_deref(), Some("where T : new()"));
            assert_eq!(c.fields.len(), 1);
            assert_eq!(c.properties.len(), 1);
            assert_eq!(c.constructors.len(), 1);
            assert_eq!(c.methods.len(), 1);
            assert_eq!(c.delegates.len(), 1);
            assert_eq!(c.indexers.len(), 1);

            assert!(c.fields[0].is_readonly);
            assert!(!c.fields[0].is_static);
            assert_eq!(c.fields[0].initializer.as_deref(), Some("0"));

            assert_eq!(c.methods[0].attributes, vec!["[Obsolete]"]);
            assert_eq!(c.methods[0].modifiers, vec![MethodModifier::Static]);
            assert!(matches!(c.methods[0].body, MethodBody::Block(ref v) if v.is_empty()));
        } else {
            panic!("Expected Class");
        }
    }

    #[test]
    fn test_struct_with_multiple_fields() {
        let s = CSharpStruct {
            doc_lines: vec![],
            attributes: vec!["[StructLayout(LayoutKind.Sequential)]".into()],
            visibility: Visibility::Public,
            is_ref_struct: false,
            name: "Point".into(),
            constraints: None,
            fields: vec![
                CSharpField {
                    visibility: Visibility::Public,
                    is_static: false,
                    is_readonly: false,
                    type_name: "float".into(),
                    name: "X".into(),
                    initializer: None,
                },
                CSharpField {
                    visibility: Visibility::Public,
                    is_static: false,
                    is_readonly: false,
                    type_name: "float".into(),
                    name: "Y".into(),
                    initializer: None,
                },
                CSharpField {
                    visibility: Visibility::Public,
                    is_static: false,
                    is_readonly: false,
                    type_name: "float".into(),
                    name: "Z".into(),
                    initializer: None,
                },
            ],
            properties: vec![],
            constructors: vec![],
            methods: vec![],
            indexers: vec![],
        };

        assert_eq!(s.fields.len(), 3);
        assert_eq!(s.fields[0].name, "X");
        assert_eq!(s.fields[1].name, "Y");
        assert_eq!(s.fields[2].name, "Z");
        assert!(s.fields.iter().all(|f| f.type_name == "float"));
    }

    #[test]
    fn test_method_modifier_all_variants() {
        let mods = vec![
            MethodModifier::Static,
            MethodModifier::Async,
            MethodModifier::Extern,
            MethodModifier::Unsafe,
        ];
        assert_eq!(mods.len(), 4);
        // Each variant is distinct
        for i in 0..mods.len() {
            for j in (i + 1)..mods.len() {
                assert_ne!(mods[i], mods[j]);
            }
        }
    }

    #[test]
    fn test_visibility_all_variants() {
        let variants = [
            Visibility::Public,
            Visibility::Private,
            Visibility::Internal,
            Visibility::Default,
        ];
        assert!(matches!(variants[0], Visibility::Public));
        assert!(matches!(variants[1], Visibility::Private));
        assert!(matches!(variants[2], Visibility::Internal));
        assert!(matches!(variants[3], Visibility::Default));
    }

    #[test]
    fn test_static_class_construction() {
        let sc = CSharpStaticClass {
            visibility: Visibility::Public,
            name: "Bindings".into(),
            methods: vec![
                CSharpMethod {
                    doc_lines: vec![],
                    attributes: vec!["[DllImport(\"lib\")]".into()],
                    visibility: Visibility::Public,
                    modifiers: vec![MethodModifier::Static, MethodModifier::Extern],
                    return_type: "void".into(),
                    name: "Init".into(),
                    parameters: vec![],
                    body: MethodBody::None,
                },
            ],
        };
        assert_eq!(sc.name, "Bindings");
        assert_eq!(sc.methods.len(), 1);
        assert_eq!(sc.methods[0].parameters.len(), 0);
        assert!(matches!(sc.methods[0].body, MethodBody::None));
    }
}
