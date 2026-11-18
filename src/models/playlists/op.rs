/// `OpInfo` : 0 valeur pour « est vide », 2 pour « entre ».
#[derive(Debug, Clone)]
pub struct Op {
    pub key: String,
    pub label: String,
    pub arity: u8,
}
