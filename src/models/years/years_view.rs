use crate::models::years::*;
use crate::models::app::Choice;

pub struct YearsView {
    pub decades: Vec<DecadeView>,
    pub index: Vec<(i64, String, f32, String)>,
    pub order_label: String,
    pub order_icon: &'static str,
    pub orders: Vec<Choice>,
}
