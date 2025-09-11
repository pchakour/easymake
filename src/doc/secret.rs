pub trait SecretDoc {
    fn id() -> &'static str;
    fn short_desc() -> &'static str;
    fn description() -> &'static str;
    fn example() -> &'static str;
}


// Define inventory entry
pub struct SecretDocEntry {
    pub id: &'static str,
    pub short_desc: &'static str,
    pub description: &'static str,
    pub example: &'static str,
}
inventory::collect!(SecretDocEntry);
