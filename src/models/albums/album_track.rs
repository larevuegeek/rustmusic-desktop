/// Un titre de la vue grille.
#[derive(Debug, Clone)]
pub struct AlbumTrack {
    /// Rang dans la liste triée (messages de ligne).
    pub i: usize,
    pub number: String,
    pub numbered: bool,
    pub cover: Option<String>,
    pub title: String,
    pub invite: Option<String>,
    pub duration: String,
    pub liked: bool,
    pub in_progress: bool,
    /// Jaquette affichée (bouton « Jaquettes »).
    pub artwork: bool,
    /// Le morceau de la fiche ouverte (`mark`).
    pub mark: bool,
}
