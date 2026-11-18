use crate::models::settings::*;
use karbon_ui::ui::prelude::Color;
use crate::models::app::Choice;
use crate::models::app::Msg;

/// Le contrôle à droite d'une ligne, ou le contenu d'un bloc.
#[derive(Debug, Clone)]
pub enum Control {
    /// Interrupteur : la ligne entière agit.
    Toggle(Msg, bool),
    /// Liste déroulante : clé du réglage, valeur, options (valeur, libellé).
    Select(&'static str, String, Vec<(String, String)>),
    /// `MenuSelect` : ancre, libellé courant, options.
    Menu(&'static str, String, Vec<Choice>),
    /// `SegmentedControl` (`subtle` ou `plein`).
    Segmented(Vec<Choice>, bool),
    Pills(Vec<Pill>),
    Buttons(Vec<Ghost>),
    /// Contrôle propre à une page (curseur, champ, touches…), rendu par son gabarit.
    Custom(&'static str),
    None,
    // Blocs : toute la largeur de la carte.
    /// Grille de `ChoiceCard` : colonnes, cartes.
    Cards(u8, Vec<Card>),
    /// `AccentPicker` : pastilles, couleur libre choisie.
    Accent(Vec<Swatch>, Option<Color>),
    /// Titre, description, puis pastilles à la ligne.
    PillsBlock(Vec<Pill>),
    /// Icône, titre et description.
    Info(&'static str),
    /// Bloc propre à une page.
    Block(&'static str),
}
