/// Explication sous un choix (`note` de la page Audio) ; `bold` remplace `{profile}`.
#[derive(Debug, Clone)]
pub struct Note {
    pub before: String,
    pub bold: Option<String>,
    pub after: String,
    pub tone: &'static str,
    pub hint: Option<String>,
}
