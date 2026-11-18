/// Une ligne d'appariement : rang local, titre local, confiance (0 sûr, 1 douteux, 2 rejeté),
/// piste distante, durée, exclue.
#[derive(Debug, Clone)]
pub struct Association {
    pub local: usize,
    pub title: String,
    pub confidence: u8,
    pub remote: String,
    pub duration: String,
    pub excluded: bool,
}
