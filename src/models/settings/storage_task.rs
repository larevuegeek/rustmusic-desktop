#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StorageTask {
    Rescan,
    Portraits,
    Covers,
    Clean,
    Repair,
    ArtistCheck,
    OpenData,
}
