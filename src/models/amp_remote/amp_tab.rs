/// Onglet de la télécommande.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AmpTab {
    #[default]
    Sources,
    Sound,
    Nav,
}
