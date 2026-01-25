use derive_getters::Getters;
use derive_new::new;

pub mod impls;
pub mod registry;

pub trait WireFunction {
    fn get_function_info() -> FunctionInfo;
}

#[derive(new, Getters)]
pub struct FunctionInfo {
    #[getter(skip)]
    name: &'static str,
    parameters: Vec<FunctionParameter>,
    return_type: TypeInfo,
    is_async: bool,
}

impl FunctionInfo {
    pub fn name(&self) -> &'static str {
        self.name
    }
}

#[derive(new, Getters)]
pub struct FunctionParameter {
    #[getter(skip)]
    name: &'static str,
    ty: TypeInfo,
}

impl FunctionParameter {
    pub fn name(&self) -> &'static str {
        self.name
    }
}

pub trait WireType {
    fn get_type_info() -> TypeInfo;
}

#[derive(Debug, Clone, new, Getters)]
pub struct FieldInfo {
    #[getter(skip)]
    name: &'static str,
    ty: TypeInfo,
}

impl FieldInfo {
    pub fn name(&self) -> &'static str {
        self.name
    }
}

#[derive(Debug, Clone, new, Getters)]
pub struct TypeInfo {
    #[getter(skip)]
    name: &'static str,
    fields: Vec<FieldInfo>,
    kind: TypeKind,
    #[getter(skip)]
    is_heap_allocated: bool,
}

impl TypeInfo {
    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn is_heap_allocated(&self) -> bool {
        self.is_heap_allocated
    }
}

#[derive(Debug, Clone)]
pub enum TypeKind {
    // Primitive types
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
    Bool,
    Pointer,

    // Unit/void type
    Void,

    // User-defined type (value type, copied across FFI)
    UserDefined,

    // Slice types for Vec/slice transfer
    Slice { element_kind: Box<TypeKind> },
    OwnedSlice { element_kind: Box<TypeKind> },
}
