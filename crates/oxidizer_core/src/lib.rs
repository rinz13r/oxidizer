use derive_getters::Getters;
use derive_new::new;
use std::future::Future;
use std::pin::Pin;

pub mod impls;
pub mod registry;

/// A runtime-agnostic trait for spawning async tasks from FFI functions.
///
/// Implement this for your chosen async runtime. Oxidizer itself does not
/// depend on any runtime crate — users provide the implementation.
pub trait Runtime {
    fn spawn(&self, fut: Pin<Box<dyn Future<Output = ()> + Send + 'static>>);
}

pub trait ReflectFunction {
    fn get_function_info() -> FunctionInfo;
}

#[derive(new, Getters)]
pub struct FunctionInfo {
    name: String,
    parameters: Vec<FunctionParameter>,
    return_type: TypeInfo,
    is_async: bool,
}

#[derive(new, Getters)]
pub struct FunctionParameter {
    name: String,
    ty: TypeInfo,
}

pub trait ReflectType {
    fn get_type_info() -> TypeInfo;
}

#[derive(Debug, Clone, new, Getters)]
pub struct FieldInfo {
    name: String,
    ty: TypeInfo,
}

#[derive(Debug, Clone, Getters, new)]
pub struct TypeInfo {
    name: String,
    fields: Vec<FieldInfo>,
    kind: TypeKind,
    generic_params: Vec<TypeInfo>,
    #[getter(skip)]
    metadata: &'static [(&'static str, &'static str)],
}

impl TypeInfo {
    pub fn metadata(&self) -> &[(&'static str, &'static str)] {
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
