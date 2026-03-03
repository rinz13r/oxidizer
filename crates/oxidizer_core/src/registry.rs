use derive_getters::Getters;

use crate::FunctionInfo;

#[derive(Getters)]
pub struct Registry {
    types: Vec<crate::TypeInfo>,
    functions: Vec<FunctionInfo>,
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

impl Registry {
    pub fn register_type<T: crate::ReflectType>(&mut self) -> &mut Self {
        let type_info = T::get_type_info();
        // Store type_info in the registry
        self.types.push(type_info);
        self
    }

    pub fn register_function<F: crate::ReflectFunction>(&mut self) -> &mut Self {
        let function_signature = F::get_function_info();
        // Store function_signature in the registry
        self.functions.push(function_signature);
        self
    }

    pub fn register_type_info(&mut self, type_info: crate::TypeInfo) -> &mut Self {
        self.types.push(type_info);
        self
    }

    pub fn register_function_info(&mut self, function_info: FunctionInfo) -> &mut Self {
        self.functions.push(function_info);
        self
    }

    pub fn new() -> Self {
        Registry {
            types: Vec::new(),
            functions: Vec::new(),
        }
    }
}
