//! Icônes Iconify compilées par `icons_dir!` (voir le composant `Icon`) : un fichier
//! `assets/icons/<set>/<name>.svg` est l'icône `"set:name"`. Pour en ajouter une, déposer
//! son SVG (https://icon-sets.iconify.design, ou `karbon-icons add set:name`) ; `build.rs`
//! fait suivre les deux dossiers à Cargo.
//!
//! `assets/icons-thick/` : les mêmes dessins avec un trait ajouté
//! (`stroke="currentColor" stroke-width="0.55" stroke-linejoin="round"` sur la racine),
//! pour les tuiles de navigation, de source et de bibliothèque.
use karbon_ui::icons_dir;

icons_dir!(pub mod plain, "/ui/assets/icons");
icons_dir!(pub mod thick, "/ui/assets/icons-thick");
