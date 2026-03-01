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

#[derive(Debug, Clone, Getters, new)]
pub struct TypeInfo {
    #[getter(skip)]
    name: &'static str,
    fields: Vec<FieldInfo>,
    kind: TypeKind,
    generic_params: Vec<TypeInfo>,
    #[getter(skip)]
    metadata: &'static [(&'static str, &'static str)],
}

impl TypeInfo {
    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn metadata(&self) -> &'static [(&'static str, &'static str)] {
        self.metadata
    }

    /// Get a metadata value by key
    pub fn get_metadata(&self, key: &str) -> Option<&'static str> {
        self.metadata
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| *v)
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

    // Struct type
    Struct,
}
