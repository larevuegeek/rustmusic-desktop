#[derive(Debug, Clone, PartialEq)]
pub enum WhatsNewMsg {
    Open,
    /// Au lancement, une fois les bibliothèques chargées.
    OpenIfDue,
    Go(usize),
    Next,
    Try,
    /// Case « Afficher au démarrage » de la fenêtre (même réglage que dans Général).
    AtStartup(bool),
}
