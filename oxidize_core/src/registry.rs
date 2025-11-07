pub struct Registry {}

impl Registry {
    pub fn register_type<T: crate::WireType>(&mut self) {
        let type_info = T::get_type_info();
        // Store type_info in the registry
    }
}
