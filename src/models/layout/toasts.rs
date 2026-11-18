use crate::models::layout::*;

#[derive(Default)]
pub struct Toasts {
    pub list: Vec<Toast>,
    pub(crate) next: u64,
}
