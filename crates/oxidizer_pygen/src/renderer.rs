use crate::IndentStyle;
use crate::ir::*;

pub(crate) fn render(file: &PythonFile, indent_style: &IndentStyle) -> String {
    let mut r = PythonRenderer {
        output: String::new(),
        indent_unit: indent_style.unit(),
    };
    r.render_file(file);
    r.output
}

struct PythonRenderer {
    output: String,
    indent_unit: &'static str,
}

impl PythonRenderer {
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

    fn render_file(&mut self, file: &PythonFile) {
        // Module docstring
        if let Some(ref doc) = file.module_docstring {
            self.line(0, &format!("\"\"\"{}\"\"\"", doc));
        }

        // Imports
        for imp in &file.imports {
            self.line(0, imp);
        }
        if !file.imports.is_empty() {
            self.blank();
        }

        // Module-level statements
        for stmt in &file.module_statements {
            self.line(0, stmt);
        }
        if !file.module_statements.is_empty() {
            self.blank();
        }

        // Items
        for (idx, item) in file.items.iter().enumerate() {
            self.render_item(item);
            // Two blank lines between top-level items (Python convention)
            if idx + 1 < file.items.len() {
                self.blank();
                self.blank();
            }
        }
    }

    fn render_item(&mut self, item: &PythonItem) {
        match item {
            PythonItem::StructClass(s) => self.render_struct_class(s),
            PythonItem::Class(c) => self.render_class(c),
            PythonItem::Function(f) => self.render_function(f),
            PythonItem::RawCode(lines) => self.render_raw_code(lines),
            PythonItem::Comment(text) => self.line(0, text),
        }
    }

    // ------------------------------------------------------------------
    // Struct class (ctypes.Structure)
    // ------------------------------------------------------------------

    fn render_struct_class(&mut self, s: &PythonStructClass) {
        self.line(0, &format!("class {}(ctypes.Structure):", s.name));

        // Docstring
        if !s.doc_lines.is_empty() {
            self.line(1, "\"\"\"");
            for doc in &s.doc_lines {
                self.line(1, doc);
            }
            self.line(1, "\"\"\"");
        }

        // _fields_
        if s.fields.is_empty() {
            self.line(1, "pass");
        } else {
            self.line(1, "_fields_ = [");
            for (idx, field) in s.fields.iter().enumerate() {
                let comma = if idx + 1 < s.fields.len() { "," } else { "," };
                self.line(
                    2,
                    &format!("(\"{}\", {}){}", field.name, field.ctypes_type, comma),
                );
            }
            self.line(1, "]");
        }
    }

    // ------------------------------------------------------------------
    // Class
    // ------------------------------------------------------------------

    fn render_class(&mut self, c: &PythonClass) {
        // class declaration
        if c.bases.is_empty() {
            self.line(0, &format!("class {}:", c.name));
        } else {
            self.line(0, &format!("class {}({}):", c.name, c.bases.join(", ")));
        }

        // Docstring
        if !c.doc_lines.is_empty() {
            self.line(1, "\"\"\"");
            for doc in &c.doc_lines {
                self.line(1, doc);
            }
            self.line(1, "\"\"\"");
        }

        let has_body = !c.class_statements.is_empty() || !c.methods.is_empty();
        if !has_body && c.doc_lines.is_empty() {
            self.line(1, "pass");
            return;
        }

        // Class-level statements
        for stmt in &c.class_statements {
            self.line(1, stmt);
        }

        // Methods
        for (idx, method) in c.methods.iter().enumerate() {
            if idx > 0 || !c.class_statements.is_empty() {
                self.blank();
            }
            self.render_method(method, 1);
        }
    }

    // ------------------------------------------------------------------
    // Function
    // ------------------------------------------------------------------

    fn render_function(&mut self, f: &PythonFunction) {
        let kw = if f.is_async { "async def" } else { "def" };
        let params = render_params(&f.parameters);
        let ret = f
            .return_annotation
            .as_ref()
            .map(|r| format!(" -> {r}"))
            .unwrap_or_default();

        self.line(
            0,
            &format!("{kw} {}{ret}:", params_with_name(&f.name, &params)),
        );

        // Docstring
        if !f.doc_lines.is_empty() {
            self.line(1, "\"\"\"");
            for doc in &f.doc_lines {
                self.line(1, doc);
            }
            self.line(1, "\"\"\"");
        }

        // Body
        if f.body.is_empty() {
            self.line(1, "pass");
        } else {
            for line in &f.body {
                self.line(1, line);
            }
        }
    }

    // ------------------------------------------------------------------
    // Method
    // ------------------------------------------------------------------

    fn render_method(&mut self, m: &PythonMethod, base_level: usize) {
        // Decorators
        for dec in &m.decorators {
            self.line(base_level, dec);
        }

        let kw = if m.is_async { "async def" } else { "def" };
        let params = render_params(&m.parameters);
        let ret = m
            .return_annotation
            .as_ref()
            .map(|r| format!(" -> {r}"))
            .unwrap_or_default();

        self.line(
            base_level,
            &format!("{kw} {}{ret}:", params_with_name(&m.name, &params)),
        );

        // Body
        let body_level = base_level + 1;
        if m.body.is_empty() {
            self.line(body_level, "pass");
        } else {
            for line in &m.body {
                self.line(body_level, line);
            }
        }
    }

    // ------------------------------------------------------------------
    // Raw code
    // ------------------------------------------------------------------

    fn render_raw_code(&mut self, lines: &[String]) {
        for line in lines {
            self.line(0, line);
        }
    }
}

// ---------------------------------------------------------------------------
// Formatting helpers
// ---------------------------------------------------------------------------

fn render_params(params: &[PythonParam]) -> String {
    params
        .iter()
        .map(|p| match &p.annotation {
            Some(ann) => format!("{}: {}", p.name, ann),
            None => p.name.clone(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn params_with_name(name: &str, params: &str) -> String {
    format!("{name}({params})")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // Empty / minimal files
    // ------------------------------------------------------------------

    #[test]
    fn test_render_empty_file() {
        let file = PythonFile {
            imports: vec![],
            module_docstring: None,
            module_statements: vec![],
            items: vec![],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert_eq!(out, "");
    }

    #[test]
    fn test_render_file_with_docstring_and_imports() {
        let file = PythonFile {
            imports: vec!["import ctypes".into(), "import asyncio".into()],
            module_docstring: Some("Auto-generated bindings.".into()),
            module_statements: vec![],
            items: vec![],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.starts_with("\"\"\"Auto-generated bindings.\"\"\"\n"));
        assert!(out.contains("import ctypes\nimport asyncio\n"));
    }

    #[test]
    fn test_render_module_statements() {
        let file = PythonFile {
            imports: vec![],
            module_docstring: None,
            module_statements: vec!["_lib = ctypes.CDLL(\"libfoo.so\")".into()],
            items: vec![],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("_lib = ctypes.CDLL(\"libfoo.so\")\n"));
    }

    // ------------------------------------------------------------------
    // Struct class
    // ------------------------------------------------------------------

    #[test]
    fn test_render_struct_class_with_fields() {
        let file = PythonFile {
            imports: vec![],
            module_docstring: None,
            module_statements: vec![],
            items: vec![PythonItem::StructClass(PythonStructClass {
                doc_lines: vec![],
                name: "OwnedRawHandle".into(),
                fields: vec![
                    PythonField {
                        name: "ptr".into(),
                        ctypes_type: "ctypes.c_void_p".into(),
                    },
                    PythonField {
                        name: "drop_fn".into(),
                        ctypes_type: "ctypes.c_void_p".into(),
                    },
                ],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("class OwnedRawHandle(ctypes.Structure):"));
        assert!(out.contains("    _fields_ = ["));
        assert!(out.contains("        (\"ptr\", ctypes.c_void_p),"));
        assert!(out.contains("        (\"drop_fn\", ctypes.c_void_p),"));
        assert!(out.contains("    ]"));
    }

    #[test]
    fn test_render_struct_class_empty_fields() {
        let file = PythonFile {
            imports: vec![],
            module_docstring: None,
            module_statements: vec![],
            items: vec![PythonItem::StructClass(PythonStructClass {
                doc_lines: vec![],
                name: "Empty".into(),
                fields: vec![],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("class Empty(ctypes.Structure):"));
        assert!(out.contains("    pass"));
    }

    #[test]
    fn test_render_struct_class_with_docstring() {
        let file = PythonFile {
            imports: vec![],
            module_docstring: None,
            module_statements: vec![],
            items: vec![PythonItem::StructClass(PythonStructClass {
                doc_lines: vec!["FFI infrastructure type.".into()],
                name: "FFISliceRaw".into(),
                fields: vec![PythonField {
                    name: "ptr".into(),
                    ctypes_type: "ctypes.c_void_p".into(),
                }],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("    \"\"\""));
        assert!(out.contains("    FFI infrastructure type."));
    }

    // ------------------------------------------------------------------
    // Class
    // ------------------------------------------------------------------

    #[test]
    fn test_render_class_with_methods() {
        let file = PythonFile {
            imports: vec![],
            module_docstring: None,
            module_statements: vec![],
            items: vec![PythonItem::Class(PythonClass {
                doc_lines: vec![],
                name: "OwnedHandle".into(),
                bases: vec![],
                class_statements: vec![],
                methods: vec![PythonMethod {
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
                }],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("class OwnedHandle:"));
        assert!(out.contains("    def __init__(self, raw: OwnedRawHandle):"));
        assert!(out.contains("        self._raw = raw"));
        assert!(out.contains("        self._disposed = False"));
    }

    #[test]
    fn test_render_class_with_bases() {
        let file = PythonFile {
            imports: vec![],
            module_docstring: None,
            module_statements: vec![],
            items: vec![PythonItem::Class(PythonClass {
                doc_lines: vec![],
                name: "MyClass".into(),
                bases: vec!["Base1".into(), "Base2".into()],
                class_statements: vec![],
                methods: vec![],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("class MyClass(Base1, Base2):"));
        assert!(out.contains("    pass"));
    }

    #[test]
    fn test_render_class_with_class_statements() {
        let file = PythonFile {
            imports: vec![],
            module_docstring: None,
            module_statements: vec![],
            items: vec![PythonItem::Class(PythonClass {
                doc_lines: vec![],
                name: "Registrar".into(),
                bases: vec![],
                class_statements: vec!["_instance = None".into()],
                methods: vec![PythonMethod {
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
                }],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("    _instance = None"));
        assert!(out.contains("    @classmethod"));
        assert!(out.contains("    def instance(cls):"));
    }

    #[test]
    fn test_render_class_with_decorators() {
        let file = PythonFile {
            imports: vec![],
            module_docstring: None,
            module_statements: vec![],
            items: vec![PythonItem::Class(PythonClass {
                doc_lines: vec![],
                name: "Foo".into(),
                bases: vec![],
                class_statements: vec![],
                methods: vec![PythonMethod {
                    decorators: vec!["@staticmethod".into()],
                    name: "bar".into(),
                    is_async: false,
                    parameters: vec![],
                    return_annotation: Some("int".into()),
                    body: vec!["return 42".into()],
                }],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("    @staticmethod"));
        assert!(out.contains("    def bar() -> int:"));
        assert!(out.contains("        return 42"));
    }

    // ------------------------------------------------------------------
    // Function
    // ------------------------------------------------------------------

    #[test]
    fn test_render_sync_function() {
        let file = PythonFile {
            imports: vec![],
            module_docstring: None,
            module_statements: vec![],
            items: vec![PythonItem::Function(PythonFunction {
                doc_lines: vec![],
                name: "add".into(),
                is_async: false,
                parameters: vec![
                    PythonParam {
                        name: "x".into(),
                        annotation: Some("int".into()),
                    },
                    PythonParam {
                        name: "y".into(),
                        annotation: Some("int".into()),
                    },
                ],
                return_annotation: Some("int".into()),
                body: vec!["return _lib.add(x, y)".into()],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("def add(x: int, y: int) -> int:"));
        assert!(out.contains("    return _lib.add(x, y)"));
    }

    #[test]
    fn test_render_async_function() {
        let file = PythonFile {
            imports: vec![],
            module_docstring: None,
            module_statements: vec![],
            items: vec![PythonItem::Function(PythonFunction {
                doc_lines: vec![],
                name: "do_work".into(),
                is_async: true,
                parameters: vec![PythonParam {
                    name: "val".into(),
                    annotation: Some("int".into()),
                }],
                return_annotation: Some("float".into()),
                body: vec![
                    "loop = asyncio.get_running_loop()".into(),
                    "return await future".into(),
                ],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("async def do_work(val: int) -> float:"));
        assert!(out.contains("    loop = asyncio.get_running_loop()"));
    }

    #[test]
    fn test_render_function_no_params_no_return() {
        let file = PythonFile {
            imports: vec![],
            module_docstring: None,
            module_statements: vec![],
            items: vec![PythonItem::Function(PythonFunction {
                doc_lines: vec![],
                name: "noop".into(),
                is_async: false,
                parameters: vec![],
                return_annotation: None,
                body: vec!["pass".into()],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("def noop():"));
        assert!(out.contains("    pass"));
    }

    #[test]
    fn test_render_function_with_docstring() {
        let file = PythonFile {
            imports: vec![],
            module_docstring: None,
            module_statements: vec![],
            items: vec![PythonItem::Function(PythonFunction {
                doc_lines: vec!["Add two numbers.".into()],
                name: "add".into(),
                is_async: false,
                parameters: vec![],
                return_annotation: None,
                body: vec!["pass".into()],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("def add():"));
        assert!(out.contains("    \"\"\""));
        assert!(out.contains("    Add two numbers."));
    }

    // ------------------------------------------------------------------
    // Raw code & comments
    // ------------------------------------------------------------------

    #[test]
    fn test_render_raw_code() {
        let file = PythonFile {
            imports: vec![],
            module_docstring: None,
            module_statements: vec![],
            items: vec![PythonItem::RawCode(vec![
                "_lib.add.argtypes = [ctypes.c_uint64, ctypes.c_uint64]".into(),
                "_lib.add.restype = ctypes.c_uint64".into(),
            ])],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("_lib.add.argtypes = [ctypes.c_uint64, ctypes.c_uint64]"));
        assert!(out.contains("_lib.add.restype = ctypes.c_uint64"));
    }

    #[test]
    fn test_render_comment() {
        let file = PythonFile {
            imports: vec![],
            module_docstring: None,
            module_statements: vec![],
            items: vec![PythonItem::Comment("# --- User types ---".into())],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("# --- User types ---"));
    }

    // ------------------------------------------------------------------
    // Blank line separation between items
    // ------------------------------------------------------------------

    #[test]
    fn test_render_blank_lines_between_items() {
        let file = PythonFile {
            imports: vec![],
            module_docstring: None,
            module_statements: vec![],
            items: vec![
                PythonItem::Function(PythonFunction {
                    doc_lines: vec![],
                    name: "a".into(),
                    is_async: false,
                    parameters: vec![],
                    return_annotation: None,
                    body: vec!["pass".into()],
                }),
                PythonItem::Function(PythonFunction {
                    doc_lines: vec![],
                    name: "b".into(),
                    is_async: false,
                    parameters: vec![],
                    return_annotation: None,
                    body: vec!["pass".into()],
                }),
            ],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        // Two blank lines between top-level items (Python convention)
        assert!(out.contains("    pass\n\n\ndef b():"));
    }

    // ------------------------------------------------------------------
    // Indent styles
    // ------------------------------------------------------------------

    #[test]
    fn test_render_spaces2_indentation() {
        let file = PythonFile {
            imports: vec![],
            module_docstring: None,
            module_statements: vec![],
            items: vec![PythonItem::Function(PythonFunction {
                doc_lines: vec![],
                name: "foo".into(),
                is_async: false,
                parameters: vec![],
                return_annotation: None,
                body: vec!["pass".into()],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces2);
        assert!(out.contains("def foo():\n  pass"));
    }

    #[test]
    fn test_render_tabs_indentation() {
        let file = PythonFile {
            imports: vec![],
            module_docstring: None,
            module_statements: vec![],
            items: vec![PythonItem::Function(PythonFunction {
                doc_lines: vec![],
                name: "foo".into(),
                is_async: false,
                parameters: vec![],
                return_annotation: None,
                body: vec!["pass".into()],
            })],
        };
        let out = render(&file, &IndentStyle::Tabs);
        assert!(out.contains("def foo():\n\tpass"));
    }

    // ------------------------------------------------------------------
    // Method body: empty
    // ------------------------------------------------------------------

    #[test]
    fn test_render_method_empty_body() {
        let file = PythonFile {
            imports: vec![],
            module_docstring: None,
            module_statements: vec![],
            items: vec![PythonItem::Class(PythonClass {
                doc_lines: vec![],
                name: "Foo".into(),
                bases: vec![],
                class_statements: vec![],
                methods: vec![PythonMethod {
                    decorators: vec![],
                    name: "noop".into(),
                    is_async: false,
                    parameters: vec![PythonParam {
                        name: "self".into(),
                        annotation: None,
                    }],
                    return_annotation: None,
                    body: vec![],
                }],
            })],
        };
        let out = render(&file, &IndentStyle::Spaces4);
        assert!(out.contains("    def noop(self):"));
        assert!(out.contains("        pass"));
    }
}
