/// En-tête d'une période : le grand nombre de la mosaïque et les puces d'années.
pub struct PeriodHead {
    pub big: String,
    pub decade: i64,
    pub year: Option<i64>,
    pub total: String,
    pub label: String,
    /// (année, nombre d'albums, nombre lisible).
    pub chips: Vec<(i64, usize, String)>,
}
