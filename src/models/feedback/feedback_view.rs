/// La page Feedback : les informations à joindre à un rapport de bug.
#[derive(Debug, Clone)]
pub struct FeedbackView {
    /// (libellé, valeur) : version, système, sortie audio, bibliothèque.
    pub diagnostics: Vec<(String, String)>,
    /// Les mêmes, en lignes de texte à copier.
    pub clipboard: String,
}
