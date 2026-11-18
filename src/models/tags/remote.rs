/// Pochette téléchargée et préparée.
#[derive(Debug, Clone)]
pub struct Remote {
    pub path: String,
    pub src: String,
    pub mime: String,
    pub size: String,
    pub bytes: usize,
    pub recompressed: bool,
}
