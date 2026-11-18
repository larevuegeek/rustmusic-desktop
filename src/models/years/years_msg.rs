#[derive(Debug, Clone, PartialEq)]
pub enum YearsMsg {
    Order(bool),
    Year(i64, Option<i64>),
    Jump(Option<i64>),
    Slide(i64, bool),
    Mix(i64, Option<i64>),
    /// La page de la période (décennie, année choisie).
    Open(i64, Option<i64>),
}
