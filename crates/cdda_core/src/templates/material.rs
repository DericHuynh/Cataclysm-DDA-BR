/// A material definition — what things are made from (steel, flesh, bone, wood, etc.).
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialTemplate {
    pub name: String,
    pub description: String,
    pub bash_resist: u32,
    pub cut_resist: u32,
    pub stab_resist: u32,
    pub bullet_resist: u32,
    pub acid_resist: u32,
    pub fire_resist: u32,
    pub elec_resist: u32,
    pub density: u32,
}
