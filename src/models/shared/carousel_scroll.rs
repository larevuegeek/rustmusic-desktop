use karbon_ui::ui::scroll::ScrollRequest;

/// Géométrie de la zone d'un carrousel, signalée par `onscroll` et `onscrollrange`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CarouselGeometry {
    pub offset: f32,
    pub max: f32,
    pub viewport: f32,
    pub extent: f32,
}

/// Un carrousel de la page ouverte : sa géométrie et le dernier glissement demandé.
#[derive(Debug, Clone, Copy, Default)]
pub struct CarouselScroll {
    pub geometry: CarouselGeometry,
    pub request: Option<ScrollRequest>,
    /// Le glissement d'une flèche est en cours : les flèches suivent sa cible, la vue n'est
    /// pas rebâtie pendant le mouvement.
    pub gliding: bool,
}
