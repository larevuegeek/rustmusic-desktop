/// D'où vient une carte « Ensuite », et donc ce qu'un clic lance.
#[derive(Debug, Clone, PartialEq)]
pub enum Launch {
    Queue(usize),
    Recent(String),
    Random(String),
}
