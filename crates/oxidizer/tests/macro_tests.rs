use oxidizer::{ReflectType, ffi_function, ffi_type};

// --- ffi_type tests ---

#[ffi_type]
pub struct RegularStruct {
    pub x: u32,
    pub y: u64,
}

#[ffi_type(marker)]
pub struct MarkerStruct {
    pub x: u32,
    pub y: u64,
}

#[derive(Debug, Clone)]
#[ffi_type]
pub struct StructWithDerives {
    pub value: u32,
}

#[test]
fn marker_type_has_empty_fields() {
    let info = MarkerStruct::get_type_info();
    assert_eq!(info.name(), "MarkerStruct");
    assert!(info.fields().is_empty(), "marker types should have empty fields");
}

#[test]
fn marker_type_has_owned_metadata() {
    let info = MarkerStruct::get_type_info();
    assert_eq!(info.metadata(), &[("ffi_repr", "owned")]);
}

#[test]
fn regular_type_has_populated_fields() {
    let info = RegularStruct::get_type_info();
    assert_eq!(info.name(), "RegularStruct");
    assert_eq!(info.fields().len(), 2);
    assert_eq!(info.fields()[0].name(), "x");
    assert_eq!(info.fields()[1].name(), "y");
}

#[test]
fn regular_type_has_no_metadata() {
    let info = RegularStruct::get_type_info();
    assert!(info.metadata().is_empty());
}

#[test]
fn user_attributes_preserved() {
    // If #[derive(Debug, Clone)] was dropped, these would fail to compile.
    let s = StructWithDerives { value: 42 };
    let cloned = s.clone();
    let debug_str = format!("{:?}", cloned);
    assert!(debug_str.contains("42"));
}

// --- ffi_function visibility tests ---

mod inner {
    use oxidizer::ffi_function;

    #[ffi_function]
    pub fn visible_fn(x: u32) -> u32 {
        x + 1
    }
}

#[test]
fn pub_function_struct_accessible_from_parent() {
    // If the generated struct didn't propagate `pub`, this wouldn't compile.
    use oxidizer::ReflectFunction;
    let info = inner::visible_fn::get_function_info();
    assert_eq!(info.name(), "visible_fn");
}

// --- ffi_function parameter extraction tests ---

#[ffi_function]
fn multi_param(a: u32, b: u64, c: i32) -> u32 {
    a + c as u32 + b as u32
}

#[test]
fn function_params_in_sync() {
    use oxidizer::ReflectFunction;
    let info = multi_param::get_function_info();
    let params = info.parameters();
    assert_eq!(params.len(), 3);
    assert_eq!(params[0].name(), "a");
    assert_eq!(params[1].name(), "b");
    assert_eq!(params[2].name(), "c");
}
