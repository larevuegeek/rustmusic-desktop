use crate::core::amp_remote::model::Amp;

#[derive(Default)]
pub struct RemoteState {
    pub found: Vec<Amp>,
    pub searching: bool,
    pub searched: bool,
    /// L'ampli piloté et sa connexion.
    pub pilot: Option<(Amp, bool)>,
}
