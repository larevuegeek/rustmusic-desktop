/// Une section de la bibliothèque.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Section {
    Tracks,
    Artists,
    Albums,
    Genres,
    Years,
    Folders,
    Mixes,
}
