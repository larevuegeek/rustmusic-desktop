/// Colonne qu'on élargit à la souris : sa largeur au début du geste et celle du moment.
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnResize {
    pub key: String,
    pub start: f32,
    pub width: f32,
}
