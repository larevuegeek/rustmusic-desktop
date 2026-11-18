/// Résultat de la dernière vérification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LastCheck {
    #[default]
    Never,
    UpToDate,
    Available,
    Failed,
}
