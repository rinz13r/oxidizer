pub mod registry;

pub struct StructLayout {}

pub trait WireType {
    fn get_type_info() -> TypeInfo;
}

impl WireType for u64 {
    fn get_type_info() -> TypeInfo {
        TypeInfo {
            name: "u64".to_string(),
            size: 8,
            fields: vec![],
        }
    }
}

impl StructLayout {
    pub fn get_type_info() -> TypeInfo {
        TypeInfo {
            name: "FFITy".to_string(),
            size: u64::get_type_info().size + u64::get_type_info().size,
            fields: vec![
                FieldInfo {
                    name: "x".to_string(),
                    offset: 0,
                    size: 8,
                    ty: u64::get_type_info(),
                },
                FieldInfo {
                    name: "y".to_string(),
                    offset: 8,
                    size: 8,
                    ty: u64::get_type_info(),
                },
            ],
        }
    }
}

struct FieldInfo {
    pub name: String,
    pub offset: u64,
    pub size: usize,
    pub ty: TypeInfo,
}

struct TypeInfo {
    pub name: String,
    pub size: u64,
    pub fields: Vec<FieldInfo>,
}

impl TypeInfo {
    pub fn size(&self) -> usize {
        self.fields.iter().map(|f| f.size).sum()
    }
}
