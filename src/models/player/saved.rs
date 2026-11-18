/// Géométrie d'avant le mode mini, rétablie à la sortie.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Saved {
    pub(crate) geometry: (i32, i32, i32, i32),
    pub(crate) maximized: bool,
    pub(crate) fullscreen: bool,
}
