use derive_getters::Getters;
use derive_new::new;

pub mod impls;
pub mod registry;

pub trait WireFunction {
    fn get_function_signature() -> FunctionSignature;
}

pub struct FunctionSignature {
    pub name: &'static str,
    pub parameters: Vec<FunctionParameter>,
    pub return_type: TypeInfo,
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

#[derive(Debug, Clone)]
pub struct FieldInfo {
    pub name: &'static str,
    pub offset: usize,
    pub size: usize,
    pub ty: TypeInfo,
}

#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub name: &'static str,
    pub size: usize,
    pub fields: Vec<FieldInfo>,
    pub kind: TypeKind,
}

impl TypeInfo {
    pub fn size(&self) -> usize {
        self.fields.iter().map(|f| f.size).sum()
    }
}

#[derive(Debug, Clone)]
pub enum TypeKind {
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
    UserDefined,
}
