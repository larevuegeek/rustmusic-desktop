/// D'où viennent les formats affichés.
#[derive(Debug, Clone, PartialEq)]
pub enum Banner {
    Probing,
    Probed,
    NoExclusive,
    Failed(String),
    CpalOnly,
    None,
}
