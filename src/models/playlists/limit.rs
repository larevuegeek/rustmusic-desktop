/// `Limit` : coupe, tri (clé de champ ou `random`), sens.
#[derive(Debug, Clone, PartialEq)]
pub struct Limit {
    pub count: Option<i64>,
    pub sort: Option<String>,
    pub desc: bool,
}
