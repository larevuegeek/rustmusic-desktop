#[derive(Debug, Clone)]
pub struct RemoteCover {
    pub path: String,
    pub src: String,
    pub size: (u32, u32),
}
