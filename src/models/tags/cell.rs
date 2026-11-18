pub struct Cell {
    pub idx: usize,
    pub value: String,
    pub dirty: bool,
    pub missing: bool,
    pub title: String,
    pub editing: bool,
    pub width: f32,
    pub grow: f32,
    pub numeric: bool,
}
