#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProfileMode {
    Select,
    Create,
    Edit(i64),
}
