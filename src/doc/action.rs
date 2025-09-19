#[derive(Debug)]
pub struct PropertyDoc {
    pub name: &'static str,
    pub description: &'static str,
    pub required: bool,
    pub ty: &'static str,
}

#[allow(unused)]
pub trait ActionDoc {
    fn id() -> &'static str;
    fn short_desc() -> &'static str;
    fn description() -> &'static str;
    fn example() -> &'static str;
}


// Define inventory entry
pub struct ActionDocEntry {
    pub id: &'static str,
    pub short_desc: &'static str,
    pub description: &'static str,
    pub example: &'static str,
    pub properties: &'static [PropertyDoc],
}
inventory::collect!(ActionDocEntry);

pub struct TypeDocEntry {
    pub name: &'static str,
    pub short_desc: &'static str,
    pub description: &'static str,
}

inventory::collect!(TypeDocEntry);

#[allow(unused)]
pub trait DocType {
    fn entry() -> TypeDocEntry;
}