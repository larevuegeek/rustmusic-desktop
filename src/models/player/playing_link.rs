/// Info du morceau en cours, dans la barre du lecteur, qui mène à sa fiche.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlayingLink {
    Track,
    Artist,
    Album,
    Year,
}
