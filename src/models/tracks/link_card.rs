use crate::models::app::Route;

/// Une carte « Album » ou « Artiste » : image, surtitre, nom, détail, lien, image ronde.
pub struct LinkCard {
    pub image: Option<String>,
    pub kicker: String,
    pub name: String,
    pub sub: Option<String>,
    pub route: Route,
    pub round: bool,
}
