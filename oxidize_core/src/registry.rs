use derive_getters::Getters;

use crate::FunctionInfo;

#[derive(Getters)]
pub struct Registry {
    types: Vec<crate::TypeInfo>,
    functions: Vec<FunctionInfo>,
}

impl Registry {
    pub fn register_type<T: crate::WireType>(&mut self) -> &mut Self {
        let type_info = T::get_type_info();
        // Store type_info in the registry
        self.types.push(type_info);
        self
    }

    pub fn register_function<F: crate::WireFunction>(&mut self) -> &mut Self {
        let function_signature = F::get_function_info();
        // Store function_signature in the registry
        self.functions.push(function_signature);
        self
    }

    pub fn new() -> Self {
        Registry {
            types: Vec::new(),
            functions: Vec::new(),
        }
    }
}
