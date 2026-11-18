#[derive(Debug, Clone, PartialEq)]
pub enum SmartMsg {
    /// Ouvre la fenêtre : playlist à modifier, mix.
    Open(Option<i64>, bool),
    Name(String),
    Color(usize),
    Icon(usize),
    Preset(usize),
    /// Liant d'un groupe (chemin, « toutes »).
    Match(Vec<usize>, bool),
    Field(Vec<usize>, u64),
    Op(Vec<usize>, u64),
    Value(Vec<usize>, String),
    Low(Vec<usize>, String),
    High(Vec<usize>, String),
    Remove(Vec<usize>),
    AddRule(Vec<usize>),
    AddGroup(Vec<usize>),
    Size(i64),
    LimitCount(String),
    /// 0 : ordre naturel, 1 : au hasard, puis les champs.
    LimitSort(u64),
    LimitDesc,
    Submit,
}
