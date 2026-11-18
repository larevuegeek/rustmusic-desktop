/// Un album ou un artiste épinglé (`PinKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinKind {
    Album,
    Artist,
}
