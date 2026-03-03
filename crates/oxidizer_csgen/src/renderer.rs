use crate::ir::*;
use crate::IndentStyle;

pub(crate) fn render(file: &CSharpFile, indent_style: &IndentStyle) -> String {
    let base = if file.namespace.is_some() { 1 } else { 0 };
    let mut r = CSharpRenderer {
        output: String::new(),
        indent_unit: indent_style.unit(),
        base,
    };
    r.render_file(file);
    r.output
}

struct CSharpRenderer {
    output: String,
    indent_unit: &'static str,
    base: usize,
}

impl CSharpRenderer {
    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    fn indent(&self, level: usize) -> String {
        self.indent_unit.repeat(level)
    }

    fn line(&mut self, level: usize, text: &str) {
        self.output.push_str(&self.indent(level));
        self.output.push_str(text);
        self.output.push('\n');
    }

    fn blank(&mut self) {
        self.output.push('\n');
    }

    // ------------------------------------------------------------------
    // File
    // ------------------------------------------------------------------

    fn render_file(&mut self, file: &CSharpFile) {
        // Usings
        for u in &file.usings {
            self.line(0, &format!("using {u};"));
        }
        self.blank();

        // Namespace open
        if let Some(ref ns) = file.namespace {
            self.line(0, &format!("namespace {ns}"));
            self.line(0, "{");
        }

        // Items
        for (idx, item) in file.items.iter().enumerate() {
            self.render_item(item);
            // Blank line between items (but not after the last)
            if idx + 1 < file.items.len() {
                self.blank();
            }
        }

        // Namespace close
        if file.namespace.is_some() {
            self.line(0, "}");
        }
    }

    fn render_item(&mut self, item: &CSharpItem) {
        match item {
            CSharpItem::Struct(s) => self.render_struct(s),
            CSharpItem::Class(c) => self.render_class(c),
            CSharpItem::StaticClass(c) => self.render_static_class(c),
        }
    }

    // ------------------------------------------------------------------
    // Struct
    // ------------------------------------------------------------------

    fn render_struct(&mut self, s: &CSharpStruct) {
        let lv = self.base;

        // Doc comments
        for doc in &s.doc_lines {
            self.line(lv, doc);
        }

        // Attributes
        for attr in &s.attributes {
            self.line(lv, attr);
        }

        // Declaration
        let vis = vis_str(s.visibility);
        let ref_kw = if s.is_ref_struct {
            "readonly ref struct"
        } else {
            "struct"
        };
        let constraint = s
            .constraints
            .as_ref()
            .map(|c| format!(" {c}"))
            .unwrap_or_default();
        self.line(lv, &format!("{vis}{ref_kw} {}{constraint}", s.name));
        self.line(lv, "{");

        // Body
        let body = lv + 1;
        self.render_struct_body(
            body,
            &s.fields,
            &s.properties,
            &s.methods,
            &s.indexers,
            &[],  // no constructors
            &[],  // no delegates
        );

        self.line(lv, "}");
    }

    // ------------------------------------------------------------------
    // Class
    // ------------------------------------------------------------------

    fn render_class(&mut self, c: &CSharpClass) {
        let lv = self.base;

        for doc in &c.doc_lines {
            self.line(lv, doc);
        }

        let vis = vis_str(c.visibility);
        let mods = if c.modifiers.is_empty() {
            String::new()
        } else {
            format!("{} ", c.modifiers.join(" "))
        };
        let implements = if c.implements.is_empty() {
            String::new()
        } else {
            format!(" : {}", c.implements.join(", "))
        };
        let constraint = c
            .constraints
            .as_ref()
            .map(|c| format!(" {c}"))
            .unwrap_or_default();
        self.line(
            lv,
            &format!("{vis}{mods}class {}{implements}{constraint}", c.name),
        );
        self.line(lv, "{");

        let body = lv + 1;
        self.render_struct_body(
            body,
            &c.fields,
            &c.properties,
            &c.methods,
            &c.indexers,
            &c.constructors,
            &c.delegates,
        );

        // Finalizer (rendered between constructors and properties/methods sections)
        if let Some(ref finalizer_lines) = c.finalizer {
            self.blank();
            let bare_name = c.name.split('<').next().unwrap_or(&c.name);
            self.line(body, &format!("~{bare_name}()"));
            self.line(body, "{");
            for l in finalizer_lines {
                self.line(body + 1, l);
            }
            self.line(body, "}");
        }

        self.line(lv, "}");
    }

    // ------------------------------------------------------------------
    // Static class
    // ------------------------------------------------------------------

    fn render_static_class(&mut self, c: &CSharpStaticClass) {
        let lv = self.base;
        let vis = vis_str(c.visibility);
        self.line(lv, &format!("{vis}static class {}", c.name));
        self.line(lv, "{");

        for (idx, m) in c.methods.iter().enumerate() {
            self.render_method(m, lv + 1);
            if idx + 1 < c.methods.len() {
                self.blank();
            }
        }

        self.line(lv, "}");
    }

    // ------------------------------------------------------------------
    // Shared body renderer (fields, props, constructors, methods, etc.)
    // ------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    fn render_struct_body(
        &mut self,
        lv: usize,
        fields: &[CSharpField],
        properties: &[CSharpProperty],
        methods: &[CSharpMethod],
        indexers: &[CSharpIndexer],
        constructors: &[CSharpMethod],
        delegates: &[CSharpDelegate],
    ) {
        // We need to track whether we need a blank separator.
        // The original code uses blank lines in specific places; we replicate
        // that by separating sections and individual multi-line members.

        let mut need_sep = false;

        // Fields
        if !fields.is_empty() {
            for f in fields {
                self.render_field(f, lv);
            }
            need_sep = true;
        }

        // Delegates
        if !delegates.is_empty() {
            if need_sep {
                self.blank();
            }
            for d in delegates {
                self.render_delegate(d, lv);
            }
            need_sep = true;
        }

        // Constructors
        if !constructors.is_empty() {
            if need_sep {
                self.blank();
            }
            for (idx, c) in constructors.iter().enumerate() {
                self.render_method(c, lv);
                if idx + 1 < constructors.len() {
                    self.blank();
                }
            }
            need_sep = true;
        }

        // Properties
        if !properties.is_empty() {
            if need_sep {
                self.blank();
            }
            for (idx, p) in properties.iter().enumerate() {
                self.render_property(p, lv);
                if idx + 1 < properties.len() {
                    self.blank();
                }
            }
            need_sep = true;
        }

        // Methods
        if !methods.is_empty() {
            if need_sep {
                self.blank();
            }
            for (idx, m) in methods.iter().enumerate() {
                self.render_method(m, lv);
                if idx + 1 < methods.len() {
                    self.blank();
                }
            }
            need_sep = true;
        }

        // Indexers
        if !indexers.is_empty() {
            if need_sep {
                self.blank();
            }
            for (idx, ix) in indexers.iter().enumerate() {
                self.render_indexer(ix, lv);
                if idx + 1 < indexers.len() {
                    self.blank();
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Members
    // ------------------------------------------------------------------

    fn render_field(&mut self, f: &CSharpField, lv: usize) {
        let vis = vis_str(f.visibility);
        let st = if f.is_static { "static " } else { "" };
        let ro = if f.is_readonly { "readonly " } else { "" };
        let init = f
            .initializer
            .as_ref()
            .map(|v| format!(" = {v}"))
            .unwrap_or_default();
        self.line(lv, &format!("{vis}{st}{ro}{} {}{init};", f.type_name, f.name));
    }

    fn render_property(&mut self, p: &CSharpProperty, lv: usize) {
        let vis = vis_str(p.visibility);
        let st = if p.is_static { "static " } else { "" };
        match &p.body {
            PropertyBody::Expression(expr) => {
                self.line(lv, &format!("{vis}{st}{} {} => {expr};", p.type_name, p.name));
            }
            PropertyBody::GetterBody(lines) => {
                self.line(lv, &format!("{vis}{st}{} {}", p.type_name, p.name));
                self.line(lv, "{");
                self.line(lv + 1, "get");
                self.line(lv + 1, "{");
                for l in lines {
                    self.line(lv + 2, l);
                }
                self.line(lv + 1, "}");
                self.line(lv, "}");
            }
        }
    }

    fn render_method(&mut self, m: &CSharpMethod, lv: usize) {
        // Doc comments
        for doc in &m.doc_lines {
            self.line(lv, doc);
        }

        // Attributes
        for attr in &m.attributes {
            self.line(lv, attr);
        }

        // Signature
        let vis = vis_str(m.visibility);
        let mods = method_modifiers_str(&m.modifiers);
        let params = m
            .parameters
            .iter()
            .map(|p| format!("{} {}", p.type_name, p.name))
            .collect::<Vec<_>>()
            .join(", ");

        // For constructors the return_type is empty — omit it to avoid double spaces
        let ret_prefix = if m.return_type.is_empty() {
            String::new()
        } else {
            format!("{} ", m.return_type)
        };

        match &m.body {
            MethodBody::None => {
                self.line(lv, &format!("{vis}{mods}{ret_prefix}{}({params});", m.name));
            }
            MethodBody::Expression(expr) => {
                self.line(
                    lv,
                    &format!("{vis}{mods}{ret_prefix}{}({params}) => {expr};", m.name),
                );
            }
            MethodBody::Block(lines) => {
                self.line(lv, &format!("{vis}{mods}{ret_prefix}{}({params})", m.name));
                self.line(lv, "{");
                for l in lines {
                    self.line(lv + 1, l);
                }
                self.line(lv, "}");
            }
        }
    }

    fn render_delegate(&mut self, d: &CSharpDelegate, lv: usize) {
        let vis = vis_str(d.visibility);
        let params = d
            .parameters
            .iter()
            .map(|p| format!("{} {}", p.type_name, p.name))
            .collect::<Vec<_>>()
            .join(", ");
        self.line(
            lv,
            &format!("{vis}delegate {} {}({params});", d.return_type, d.name),
        );
    }

    fn render_indexer(&mut self, ix: &CSharpIndexer, lv: usize) {
        let vis = vis_str(ix.visibility);
        self.line(
            lv,
            &format!(
                "{vis}{} this[{} {}]",
                ix.type_name, ix.parameter.type_name, ix.parameter.name
            ),
        );
        self.line(lv, "{");
        self.line(lv + 1, "get");
        self.line(lv + 1, "{");
        for l in &ix.getter_body {
            self.line(lv + 2, l);
        }
        self.line(lv + 1, "}");
        self.line(lv, "}");
    }
}

// ---------------------------------------------------------------------------
// Formatting helpers
// ---------------------------------------------------------------------------

fn vis_str(v: Visibility) -> &'static str {
    match v {
        Visibility::Public => "public ",
        Visibility::Private => "private ",
        Visibility::Internal => "internal ",
        Visibility::Default => "",
    }
}

fn method_modifiers_str(mods: &[MethodModifier]) -> String {
    if mods.is_empty() {
        return String::new();
    }
    let mut s = String::new();
    for m in mods {
        s.push_str(match m {
            MethodModifier::Static => "static ",
            MethodModifier::Async => "async ",
            MethodModifier::Extern => "extern ",
            MethodModifier::Unsafe => "unsafe ",
        });
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_empty_file_no_namespace() {
        let file = CSharpFile {
            usings: vec!["System".into()],
            namespace: None,
            items: vec![],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert_eq!(out, "using System;\n\n");
    }

    #[test]
    fn test_render_simple_struct() {
        let file = CSharpFile {
            usings: vec![],
            namespace: None,
            items: vec![CSharpItem::Struct(CSharpStruct {
                doc_lines: vec![],
                attributes: vec!["[StructLayout(LayoutKind.Sequential)]".into()],
                visibility: Visibility::Public,
                is_ref_struct: false,
                name: "MyStruct".into(),
                constraints: None,
                fields: vec![CSharpField {
                    visibility: Visibility::Public,
                    is_static: false,
                    is_readonly: false,
                    type_name: "int".into(),
                    name: "Value".into(),
                    initializer: None,
                }],
                properties: vec![],
                methods: vec![],
                indexers: vec![],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("[StructLayout(LayoutKind.Sequential)]"));
        assert!(out.contains("public struct MyStruct"));
        assert!(out.contains("    public int Value;"));
    }

    #[test]
    fn test_render_extern_method() {
        let m = CSharpMethod {
            doc_lines: vec![],
            attributes: vec![
                "[DllImport(\"lib.dll\", EntryPoint = \"foo\", CallingConvention = CallingConvention.Cdecl)]".into(),
            ],
            visibility: Visibility::Public,
            modifiers: vec![MethodModifier::Static, MethodModifier::Extern],
            return_type: "void".into(),
            name: "Foo".into(),
            parameters: vec![CSharpParam {
                type_name: "int".into(),
                name: "x".into(),
            }],
            body: MethodBody::None,
        };
        let file = CSharpFile {
            usings: vec![],
            namespace: None,
            items: vec![CSharpItem::StaticClass(CSharpStaticClass {
                visibility: Visibility::Public,
                name: "Bindings".into(),
                methods: vec![m],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("public static extern void Foo(int x);"));
    }

    #[test]
    fn test_render_with_namespace_and_tabs() {
        let file = CSharpFile {
            usings: vec!["System".into()],
            namespace: Some("MyNs".into()),
            items: vec![CSharpItem::StaticClass(CSharpStaticClass {
                visibility: Visibility::Public,
                name: "Bindings".into(),
                methods: vec![],
            })],
        };
        let out = render(&file, &IndentStyle::Tabs);
        assert!(out.contains("namespace MyNs"));
        assert!(out.contains("\tpublic static class Bindings"));
    }

    // ------------------------------------------------------------------
    // Visibility
    // ------------------------------------------------------------------

    #[test]
    fn test_visibility_variants() {
        let make_struct = |vis| CSharpFile {
            usings: vec![],
            namespace: None,
            items: vec![CSharpItem::Struct(CSharpStruct {
                doc_lines: vec![],
                attributes: vec![],
                visibility: vis,
                is_ref_struct: false,
                name: "Foo".into(),
                constraints: None,
                fields: vec![],
                properties: vec![],
                methods: vec![],
                indexers: vec![],
            })],
        };
        let out = render(&make_struct(Visibility::Public), &IndentStyle::Spaces4);
        assert!(out.contains("public struct Foo"));

        let out = render(&make_struct(Visibility::Private), &IndentStyle::Spaces4);
        assert!(out.contains("private struct Foo"));

        let out = render(&make_struct(Visibility::Internal), &IndentStyle::Spaces4);
        assert!(out.contains("internal struct Foo"));

        let out = render(&make_struct(Visibility::Default), &IndentStyle::Spaces4);
        assert!(out.contains("struct Foo"));
        assert!(!out.contains("public struct Foo"));
    }

    // ------------------------------------------------------------------
    // Struct rendering
    // ------------------------------------------------------------------

    #[test]
    fn test_render_ref_struct_with_constraints() {
        let file = CSharpFile {
            usings: vec![],
            namespace: None,
            items: vec![CSharpItem::Struct(CSharpStruct {
                doc_lines: vec![],
                attributes: vec![],
                visibility: Visibility::Public,
                is_ref_struct: true,
                name: "SliceHandle<T>".into(),
                constraints: Some("where T : unmanaged".into()),
                fields: vec![],
                properties: vec![],
                methods: vec![],
                indexers: vec![],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("public readonly ref struct SliceHandle<T> where T : unmanaged"));
    }

    #[test]
    fn test_render_struct_doc_lines() {
        let file = CSharpFile {
            usings: vec![],
            namespace: None,
            items: vec![CSharpItem::Struct(CSharpStruct {
                doc_lines: vec![
                    "/// <summary>".into(),
                    "/// My documentation.".into(),
                    "/// </summary>".into(),
                ],
                attributes: vec![],
                visibility: Visibility::Public,
                is_ref_struct: false,
                name: "Foo".into(),
                constraints: None,
                fields: vec![],
                properties: vec![],
                methods: vec![],
                indexers: vec![],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("/// <summary>\n/// My documentation.\n/// </summary>\npublic struct Foo"));
    }

    // ------------------------------------------------------------------
    // Field rendering
    // ------------------------------------------------------------------

    #[test]
    fn test_render_static_readonly_field_with_initializer() {
        let file = CSharpFile {
            usings: vec![],
            namespace: None,
            items: vec![CSharpItem::Struct(CSharpStruct {
                doc_lines: vec![],
                attributes: vec![],
                visibility: Visibility::Public,
                is_ref_struct: false,
                name: "Foo".into(),
                constraints: None,
                fields: vec![CSharpField {
                    visibility: Visibility::Public,
                    is_static: true,
                    is_readonly: true,
                    type_name: "Foo".into(),
                    name: "Instance".into(),
                    initializer: Some("new()".into()),
                }],
                properties: vec![],
                methods: vec![],
                indexers: vec![],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("public static readonly Foo Instance = new();"));
    }

    #[test]
    fn test_render_private_field_no_modifiers() {
        let file = CSharpFile {
            usings: vec![],
            namespace: None,
            items: vec![CSharpItem::Struct(CSharpStruct {
                doc_lines: vec![],
                attributes: vec![],
                visibility: Visibility::Public,
                is_ref_struct: false,
                name: "Foo".into(),
                constraints: None,
                fields: vec![CSharpField {
                    visibility: Visibility::Private,
                    is_static: false,
                    is_readonly: false,
                    type_name: "bool".into(),
                    name: "_disposed".into(),
                    initializer: None,
                }],
                properties: vec![],
                methods: vec![],
                indexers: vec![],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("    private bool _disposed;"));
    }

    // ------------------------------------------------------------------
    // Property rendering
    // ------------------------------------------------------------------

    #[test]
    fn test_render_expression_property() {
        let file = CSharpFile {
            usings: vec![],
            namespace: None,
            items: vec![CSharpItem::Struct(CSharpStruct {
                doc_lines: vec![],
                attributes: vec![],
                visibility: Visibility::Public,
                is_ref_struct: false,
                name: "Foo".into(),
                constraints: None,
                fields: vec![],
                properties: vec![CSharpProperty {
                    visibility: Visibility::Public,
                    is_static: false,
                    type_name: "int".into(),
                    name: "Length".into(),
                    body: PropertyBody::Expression("(int)_raw.Len".into()),
                }],
                methods: vec![],
                indexers: vec![],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("    public int Length => (int)_raw.Len;"));
    }

    #[test]
    fn test_render_static_property() {
        let file = CSharpFile {
            usings: vec![],
            namespace: None,
            items: vec![CSharpItem::Struct(CSharpStruct {
                doc_lines: vec![],
                attributes: vec![],
                visibility: Visibility::Public,
                is_ref_struct: false,
                name: "Foo".into(),
                constraints: None,
                fields: vec![],
                properties: vec![CSharpProperty {
                    visibility: Visibility::Public,
                    is_static: true,
                    type_name: "int".into(),
                    name: "Count".into(),
                    body: PropertyBody::Expression("_count".into()),
                }],
                methods: vec![],
                indexers: vec![],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("    public static int Count => _count;"));
    }

    #[test]
    fn test_render_getter_body_property() {
        let file = CSharpFile {
            usings: vec![],
            namespace: None,
            items: vec![CSharpItem::Struct(CSharpStruct {
                doc_lines: vec![],
                attributes: vec![],
                visibility: Visibility::Public,
                is_ref_struct: false,
                name: "Foo".into(),
                constraints: None,
                fields: vec![],
                properties: vec![CSharpProperty {
                    visibility: Visibility::Public,
                    is_static: false,
                    type_name: "string".into(),
                    name: "Name".into(),
                    body: PropertyBody::GetterBody(vec![
                        "return _name;".into(),
                    ]),
                }],
                methods: vec![],
                indexers: vec![],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("    public string Name\n    {\n        get\n        {\n            return _name;\n        }\n    }"));
    }

    // ------------------------------------------------------------------
    // Method rendering
    // ------------------------------------------------------------------

    #[test]
    fn test_render_method_with_block_body() {
        let file = CSharpFile {
            usings: vec![],
            namespace: None,
            items: vec![CSharpItem::StaticClass(CSharpStaticClass {
                visibility: Visibility::Public,
                name: "Utils".into(),
                methods: vec![CSharpMethod {
                    doc_lines: vec![],
                    attributes: vec![],
                    visibility: Visibility::Public,
                    modifiers: vec![MethodModifier::Static],
                    return_type: "void".into(),
                    name: "DoStuff".into(),
                    parameters: vec![
                        CSharpParam { type_name: "int".into(), name: "a".into() },
                        CSharpParam { type_name: "string".into(), name: "b".into() },
                    ],
                    body: MethodBody::Block(vec![
                        "Console.WriteLine(a);".into(),
                        "Console.WriteLine(b);".into(),
                    ]),
                }],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("public static void DoStuff(int a, string b)"));
        assert!(out.contains("        Console.WriteLine(a);"));
        assert!(out.contains("        Console.WriteLine(b);"));
    }

    #[test]
    fn test_render_expression_bodied_method() {
        let file = CSharpFile {
            usings: vec![],
            namespace: None,
            items: vec![CSharpItem::StaticClass(CSharpStaticClass {
                visibility: Visibility::Public,
                name: "Utils".into(),
                methods: vec![CSharpMethod {
                    doc_lines: vec![],
                    attributes: vec![],
                    visibility: Visibility::Public,
                    modifiers: vec![MethodModifier::Static],
                    return_type: "int".into(),
                    name: "Add".into(),
                    parameters: vec![
                        CSharpParam { type_name: "int".into(), name: "a".into() },
                        CSharpParam { type_name: "int".into(), name: "b".into() },
                    ],
                    body: MethodBody::Expression("a + b".into()),
                }],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("public static int Add(int a, int b) => a + b;"));
    }

    #[test]
    fn test_render_method_doc_lines_and_attributes() {
        let file = CSharpFile {
            usings: vec![],
            namespace: None,
            items: vec![CSharpItem::StaticClass(CSharpStaticClass {
                visibility: Visibility::Public,
                name: "Bindings".into(),
                methods: vec![CSharpMethod {
                    doc_lines: vec![
                        "/// <summary>Does a thing.</summary>".into(),
                    ],
                    attributes: vec![
                        "[DllImport(\"lib.dll\")]".into(),
                    ],
                    visibility: Visibility::Public,
                    modifiers: vec![MethodModifier::Static, MethodModifier::Extern],
                    return_type: "void".into(),
                    name: "Foo".into(),
                    parameters: vec![],
                    body: MethodBody::None,
                }],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        // Doc line, then attribute, then method
        let idx_doc = out.find("/// <summary>Does a thing.</summary>").unwrap();
        let idx_attr = out.find("[DllImport(\"lib.dll\")]").unwrap();
        let idx_method = out.find("public static extern void Foo()").unwrap();
        assert!(idx_doc < idx_attr);
        assert!(idx_attr < idx_method);
    }

    #[test]
    fn test_render_constructor_empty_return_type() {
        let file = CSharpFile {
            usings: vec![],
            namespace: None,
            items: vec![CSharpItem::Class(CSharpClass {
                doc_lines: vec![],
                visibility: Visibility::Public,
                modifiers: vec![],
                name: "MyClass".into(),
                constraints: None,
                implements: vec![],
                fields: vec![],
                properties: vec![],
                constructors: vec![CSharpMethod {
                    doc_lines: vec![],
                    attributes: vec![],
                    visibility: Visibility::Public,
                    modifiers: vec![],
                    return_type: String::new(),
                    name: "MyClass".into(),
                    parameters: vec![CSharpParam {
                        type_name: "int".into(),
                        name: "value".into(),
                    }],
                    body: MethodBody::Expression("_value = value".into()),
                }],
                finalizer: None,
                methods: vec![],
                delegates: vec![],
                indexers: vec![],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        // No double space before name when return_type is empty
        assert!(out.contains("public MyClass(int value) => _value = value;"));
        assert!(!out.contains("public  MyClass"));
    }

    #[test]
    fn test_render_async_unsafe_modifiers() {
        let file = CSharpFile {
            usings: vec![],
            namespace: None,
            items: vec![CSharpItem::StaticClass(CSharpStaticClass {
                visibility: Visibility::Public,
                name: "Test".into(),
                methods: vec![CSharpMethod {
                    doc_lines: vec![],
                    attributes: vec![],
                    visibility: Visibility::Public,
                    modifiers: vec![MethodModifier::Static, MethodModifier::Async],
                    return_type: "Task<int>".into(),
                    name: "RunAsync".into(),
                    parameters: vec![],
                    body: MethodBody::Block(vec!["return await Task.FromResult(42);".into()]),
                }],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("public static async Task<int> RunAsync()"));
    }

    // ------------------------------------------------------------------
    // Delegate rendering
    // ------------------------------------------------------------------

    #[test]
    fn test_render_delegate() {
        let file = CSharpFile {
            usings: vec![],
            namespace: None,
            items: vec![CSharpItem::Class(CSharpClass {
                doc_lines: vec![],
                visibility: Visibility::Public,
                modifiers: vec![],
                name: "MyClass".into(),
                constraints: None,
                implements: vec![],
                fields: vec![],
                properties: vec![],
                constructors: vec![],
                finalizer: None,
                methods: vec![],
                delegates: vec![CSharpDelegate {
                    visibility: Visibility::Public,
                    return_type: "void".into(),
                    name: "CallbackDelegate".into(),
                    parameters: vec![
                        CSharpParam { type_name: "ulong".into(), name: "id".into() },
                        CSharpParam { type_name: "ulong".into(), name: "result".into() },
                    ],
                }],
                indexers: vec![],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("public delegate void CallbackDelegate(ulong id, ulong result);"));
    }

    // ------------------------------------------------------------------
    // Indexer rendering
    // ------------------------------------------------------------------

    #[test]
    fn test_render_indexer() {
        let file = CSharpFile {
            usings: vec![],
            namespace: None,
            items: vec![CSharpItem::Class(CSharpClass {
                doc_lines: vec![],
                visibility: Visibility::Public,
                modifiers: vec![],
                name: "MyCollection".into(),
                constraints: None,
                implements: vec![],
                fields: vec![],
                properties: vec![],
                constructors: vec![],
                finalizer: None,
                methods: vec![],
                delegates: vec![],
                indexers: vec![CSharpIndexer {
                    visibility: Visibility::Public,
                    type_name: "T".into(),
                    parameter: CSharpParam {
                        type_name: "int".into(),
                        name: "index".into(),
                    },
                    getter_body: vec![
                        "if (index < 0) throw new IndexOutOfRangeException();".into(),
                        "return _items[index];".into(),
                    ],
                }],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("public T this[int index]"));
        assert!(out.contains("get"));
        assert!(out.contains("if (index < 0) throw new IndexOutOfRangeException();"));
        assert!(out.contains("return _items[index];"));
    }

    // ------------------------------------------------------------------
    // Class rendering
    // ------------------------------------------------------------------

    #[test]
    fn test_render_class_with_modifiers_and_implements() {
        let file = CSharpFile {
            usings: vec![],
            namespace: None,
            items: vec![CSharpItem::Class(CSharpClass {
                doc_lines: vec!["/// <summary>My class</summary>".into()],
                visibility: Visibility::Public,
                modifiers: vec!["sealed".into()],
                name: "OwnedHandle<T>".into(),
                constraints: None,
                implements: vec!["IDisposable".into()],
                fields: vec![],
                properties: vec![],
                constructors: vec![],
                finalizer: None,
                methods: vec![],
                delegates: vec![],
                indexers: vec![],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("public sealed class OwnedHandle<T> : IDisposable"));
    }

    #[test]
    fn test_render_class_multiple_implements() {
        let file = CSharpFile {
            usings: vec![],
            namespace: None,
            items: vec![CSharpItem::Class(CSharpClass {
                doc_lines: vec![],
                visibility: Visibility::Public,
                modifiers: vec![],
                name: "MyClass".into(),
                constraints: None,
                implements: vec!["IDisposable".into(), "IEquatable<MyClass>".into()],
                fields: vec![],
                properties: vec![],
                constructors: vec![],
                finalizer: None,
                methods: vec![],
                delegates: vec![],
                indexers: vec![],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("public class MyClass : IDisposable, IEquatable<MyClass>"));
    }

    #[test]
    fn test_render_class_with_constraints() {
        let file = CSharpFile {
            usings: vec![],
            namespace: None,
            items: vec![CSharpItem::Class(CSharpClass {
                doc_lines: vec![],
                visibility: Visibility::Public,
                modifiers: vec![],
                name: "Container<T>".into(),
                constraints: Some("where T : struct".into()),
                implements: vec![],
                fields: vec![],
                properties: vec![],
                constructors: vec![],
                finalizer: None,
                methods: vec![],
                delegates: vec![],
                indexers: vec![],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("public class Container<T> where T : struct"));
    }

    // ------------------------------------------------------------------
    // File-level: multiple usings, items, and separator logic
    // ------------------------------------------------------------------

    #[test]
    fn test_render_multiple_usings() {
        let file = CSharpFile {
            usings: vec![
                "System".into(),
                "System.Collections.Generic".into(),
                "System.Runtime.InteropServices".into(),
            ],
            namespace: None,
            items: vec![],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("using System;\nusing System.Collections.Generic;\nusing System.Runtime.InteropServices;\n"));
    }

    #[test]
    fn test_render_blank_line_between_items() {
        let file = CSharpFile {
            usings: vec![],
            namespace: None,
            items: vec![
                CSharpItem::Struct(CSharpStruct {
                    doc_lines: vec![],
                    attributes: vec![],
                    visibility: Visibility::Public,
                    is_ref_struct: false,
                    name: "A".into(),
                    constraints: None,
                    fields: vec![],
                    properties: vec![],
                    methods: vec![],
                    indexers: vec![],
                }),
                CSharpItem::Struct(CSharpStruct {
                    doc_lines: vec![],
                    attributes: vec![],
                    visibility: Visibility::Public,
                    is_ref_struct: false,
                    name: "B".into(),
                    constraints: None,
                    fields: vec![],
                    properties: vec![],
                    methods: vec![],
                    indexers: vec![],
                }),
            ],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        // There should be a blank line between the two structs
        assert!(out.contains("}\n\npublic struct B"));
    }

    #[test]
    fn test_render_no_trailing_blank_after_last_item() {
        let file = CSharpFile {
            usings: vec![],
            namespace: None,
            items: vec![CSharpItem::Struct(CSharpStruct {
                doc_lines: vec![],
                attributes: vec![],
                visibility: Visibility::Public,
                is_ref_struct: false,
                name: "A".into(),
                constraints: None,
                fields: vec![],
                properties: vec![],
                methods: vec![],
                indexers: vec![],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        // Should end with "}\n" (no extra blank line)
        assert!(out.ends_with("}\n"));
    }

    // ------------------------------------------------------------------
    // Namespace wrapping
    // ------------------------------------------------------------------

    #[test]
    fn test_render_namespace_wrapping_indentation() {
        let file = CSharpFile {
            usings: vec![],
            namespace: Some("MyApp.Interop".into()),
            items: vec![CSharpItem::Struct(CSharpStruct {
                doc_lines: vec![],
                attributes: vec![],
                visibility: Visibility::Public,
                is_ref_struct: false,
                name: "Foo".into(),
                constraints: None,
                fields: vec![CSharpField {
                    visibility: Visibility::Public,
                    is_static: false,
                    is_readonly: false,
                    type_name: "int".into(),
                    name: "X".into(),
                    initializer: None,
                }],
                properties: vec![],
                methods: vec![],
                indexers: vec![],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        // With namespace, struct should be indented one level
        assert!(out.contains("    public struct Foo"));
        assert!(out.contains("        public int X;"));
        // Namespace open and close at top level
        assert!(out.contains("namespace MyApp.Interop\n{"));
        assert!(out.ends_with("}\n"));
    }

    // ------------------------------------------------------------------
    // Indent style: Spaces2
    // ------------------------------------------------------------------

    #[test]
    fn test_render_spaces2_indentation() {
        let file = CSharpFile {
            usings: vec![],
            namespace: Some("NS".into()),
            items: vec![CSharpItem::Struct(CSharpStruct {
                doc_lines: vec![],
                attributes: vec![],
                visibility: Visibility::Public,
                is_ref_struct: false,
                name: "Foo".into(),
                constraints: None,
                fields: vec![CSharpField {
                    visibility: Visibility::Public,
                    is_static: false,
                    is_readonly: false,
                    type_name: "int".into(),
                    name: "X".into(),
                    initializer: None,
                }],
                properties: vec![],
                methods: vec![],
                indexers: vec![],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces2);
        assert!(out.contains("  public struct Foo"));
        assert!(out.contains("    public int X;"));
    }

    // ------------------------------------------------------------------
    // Body section ordering & separator logic
    // ------------------------------------------------------------------

    #[test]
    fn test_render_body_section_separators() {
        // Struct with fields, properties, and methods should have blank lines between sections
        let file = CSharpFile {
            usings: vec![],
            namespace: None,
            items: vec![CSharpItem::Struct(CSharpStruct {
                doc_lines: vec![],
                attributes: vec![],
                visibility: Visibility::Public,
                is_ref_struct: false,
                name: "Foo".into(),
                constraints: None,
                fields: vec![CSharpField {
                    visibility: Visibility::Public,
                    is_static: false,
                    is_readonly: false,
                    type_name: "int".into(),
                    name: "X".into(),
                    initializer: None,
                }],
                properties: vec![CSharpProperty {
                    visibility: Visibility::Public,
                    is_static: false,
                    type_name: "int".into(),
                    name: "Y".into(),
                    body: PropertyBody::Expression("X * 2".into()),
                }],
                methods: vec![CSharpMethod {
                    doc_lines: vec![],
                    attributes: vec![],
                    visibility: Visibility::Public,
                    modifiers: vec![],
                    return_type: "void".into(),
                    name: "Reset".into(),
                    parameters: vec![],
                    body: MethodBody::Block(vec!["X = 0;".into()]),
                }],
                indexers: vec![],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        // Blank line between fields and properties
        let idx_field = out.find("public int X;").unwrap();
        let idx_prop = out.find("public int Y =>").unwrap();
        let between = &out[idx_field..idx_prop];
        assert!(between.contains("\n\n"), "Expected blank line between fields and properties");

        // Blank line between properties and methods
        let idx_method = out.find("public void Reset()").unwrap();
        let between2 = &out[idx_prop..idx_method];
        assert!(between2.contains("\n\n"), "Expected blank line between properties and methods");
    }

    #[test]
    fn test_render_class_body_ordering() {
        // Class with fields, delegates, constructors, properties, methods, indexers
        let file = CSharpFile {
            usings: vec![],
            namespace: None,
            items: vec![CSharpItem::Class(CSharpClass {
                doc_lines: vec![],
                visibility: Visibility::Public,
                modifiers: vec![],
                name: "Full".into(),
                constraints: None,
                implements: vec![],
                fields: vec![CSharpField {
                    visibility: Visibility::Private,
                    is_static: false,
                    is_readonly: false,
                    type_name: "int".into(),
                    name: "_val".into(),
                    initializer: None,
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
                    attributes: vec![],
                    visibility: Visibility::Public,
                    modifiers: vec![],
                    return_type: "void".into(),
                    name: "Clear".into(),
                    parameters: vec![],
                    body: MethodBody::Block(vec!["_val = 0;".into()]),
                }],
                delegates: vec![CSharpDelegate {
                    visibility: Visibility::Public,
                    return_type: "void".into(),
                    name: "OnChange".into(),
                    parameters: vec![CSharpParam {
                        type_name: "int".into(),
                        name: "newVal".into(),
                    }],
                }],
                indexers: vec![CSharpIndexer {
                    visibility: Visibility::Public,
                    type_name: "int".into(),
                    parameter: CSharpParam {
                        type_name: "int".into(),
                        name: "i".into(),
                    },
                    getter_body: vec!["return _val + i;".into()],
                }],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);

        // Verify ordering: fields -> delegates -> constructors -> properties -> methods -> indexers
        let idx_field = out.find("private int _val;").unwrap();
        let idx_delegate = out.find("public delegate void OnChange(int newVal);").unwrap();
        let idx_ctor = out.find("public Full(int v)").unwrap();
        let idx_prop = out.find("public int Val =>").unwrap();
        let idx_method = out.find("public void Clear()").unwrap();
        let idx_indexer = out.find("public int this[int i]").unwrap();

        assert!(idx_field < idx_delegate, "fields before delegates");
        assert!(idx_delegate < idx_ctor, "delegates before constructors");
        assert!(idx_ctor < idx_prop, "constructors before properties");
        assert!(idx_prop < idx_method, "properties before methods");
        assert!(idx_method < idx_indexer, "methods before indexers");
    }

    // ------------------------------------------------------------------
    // Static class
    // ------------------------------------------------------------------

    #[test]
    fn test_render_static_class_multiple_methods_separated() {
        let file = CSharpFile {
            usings: vec![],
            namespace: None,
            items: vec![CSharpItem::StaticClass(CSharpStaticClass {
                visibility: Visibility::Internal,
                name: "Helpers".into(),
                methods: vec![
                    CSharpMethod {
                        doc_lines: vec![],
                        attributes: vec![],
                        visibility: Visibility::Public,
                        modifiers: vec![MethodModifier::Static],
                        return_type: "int".into(),
                        name: "First".into(),
                        parameters: vec![],
                        body: MethodBody::Expression("1".into()),
                    },
                    CSharpMethod {
                        doc_lines: vec![],
                        attributes: vec![],
                        visibility: Visibility::Public,
                        modifiers: vec![MethodModifier::Static],
                        return_type: "int".into(),
                        name: "Second".into(),
                        parameters: vec![],
                        body: MethodBody::Expression("2".into()),
                    },
                ],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("internal static class Helpers"));
        // Blank line between methods
        let idx_first = out.find("First()").unwrap();
        let idx_second = out.find("Second()").unwrap();
        let between = &out[idx_first..idx_second];
        assert!(between.contains("\n\n"), "Expected blank line between static class methods");
    }

    // ------------------------------------------------------------------
    // Method modifier combinations
    // ------------------------------------------------------------------

    #[test]
    fn test_render_unsafe_method() {
        let file = CSharpFile {
            usings: vec![],
            namespace: None,
            items: vec![CSharpItem::StaticClass(CSharpStaticClass {
                visibility: Visibility::Public,
                name: "Unsafe".into(),
                methods: vec![CSharpMethod {
                    doc_lines: vec![],
                    attributes: vec![],
                    visibility: Visibility::Public,
                    modifiers: vec![MethodModifier::Unsafe],
                    return_type: "void*".into(),
                    name: "GetPtr".into(),
                    parameters: vec![],
                    body: MethodBody::Block(vec!["return null;".into()]),
                }],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("public unsafe void* GetPtr()"));
    }

    // ------------------------------------------------------------------
    // Edge cases
    // ------------------------------------------------------------------

    #[test]
    fn test_render_empty_file_with_namespace() {
        let file = CSharpFile {
            usings: vec![],
            namespace: Some("Empty".into()),
            items: vec![],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert_eq!(out, "\nnamespace Empty\n{\n}\n");
    }

    #[test]
    fn test_render_no_usings_no_namespace() {
        let file = CSharpFile {
            usings: vec![],
            namespace: None,
            items: vec![],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert_eq!(out, "\n");
    }

    #[test]
    fn test_render_method_no_params() {
        let file = CSharpFile {
            usings: vec![],
            namespace: None,
            items: vec![CSharpItem::StaticClass(CSharpStaticClass {
                visibility: Visibility::Public,
                name: "S".into(),
                methods: vec![CSharpMethod {
                    doc_lines: vec![],
                    attributes: vec![],
                    visibility: Visibility::Public,
                    modifiers: vec![MethodModifier::Static, MethodModifier::Extern],
                    return_type: "void".into(),
                    name: "NoParams".into(),
                    parameters: vec![],
                    body: MethodBody::None,
                }],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("public static extern void NoParams();"));
    }

    #[test]
    fn test_render_method_multiple_params() {
        let file = CSharpFile {
            usings: vec![],
            namespace: None,
            items: vec![CSharpItem::StaticClass(CSharpStaticClass {
                visibility: Visibility::Public,
                name: "S".into(),
                methods: vec![CSharpMethod {
                    doc_lines: vec![],
                    attributes: vec![],
                    visibility: Visibility::Public,
                    modifiers: vec![MethodModifier::Static, MethodModifier::Extern],
                    return_type: "int".into(),
                    name: "Multi".into(),
                    parameters: vec![
                        CSharpParam { type_name: "int".into(), name: "a".into() },
                        CSharpParam { type_name: "float".into(), name: "b".into() },
                        CSharpParam { type_name: "IntPtr".into(), name: "c".into() },
                    ],
                    body: MethodBody::None,
                }],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("public static extern int Multi(int a, float b, IntPtr c);"));
    }
}
