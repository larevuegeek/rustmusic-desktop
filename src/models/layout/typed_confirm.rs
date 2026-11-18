use crate::models::layout::*;

#[derive(Debug, Clone)]
pub struct TypedConfirm {
    pub kind: TypedKind,
    pub typed: String,
    pub busy: bool,
}
