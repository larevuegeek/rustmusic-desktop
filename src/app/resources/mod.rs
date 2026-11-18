//! Le thème embarqué dans l'exécutable : polices et images, icônes, traductions, feuille
//! de style.

pub(crate) mod assets;
pub(crate) mod icons;
pub(crate) mod translations;

karbon_ui::ui::prelude::css_file!(pub mod css, "/ui/assets/css/app.css");
