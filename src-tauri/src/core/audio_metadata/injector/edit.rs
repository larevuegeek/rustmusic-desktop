//! Description d'une modification de tags, indépendante du format de fichier.
//!
//! # Pourquoi un type dédié plutôt que `AudioTags`
//! `AudioTags` décrit ce qu'un fichier **contient**. Ici on décrit ce qu'on
//! veut **changer**, et ces deux choses ne se confondent pas : avec
//! `Option<String>`, un `None` serait ambigu — « ne touche pas à ce champ » ou
//! « efface-le » ? La distinction compte : sur une édition multiple, laisser
//! intacts les titres de dix morceaux tout en corrigeant leur album est le cas
//! d'usage principal.
//!
//! D'où [`FieldEdit`], qui rend l'intention explicite à la lecture du code.

/// Ce qu'on veut faire d'un champ.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum FieldEdit<T> {
    /// Ne pas toucher à la valeur existante (défaut).
    #[default]
    Keep,
    /// Supprimer la valeur du fichier.
    Clear,
    /// Remplacer par cette valeur.
    Set(T),
}

impl<T> FieldEdit<T> {
    /// Vrai si ce champ demande une écriture.
    pub fn is_change(&self) -> bool {
        !matches!(self, FieldEdit::Keep)
    }

    /// Applique la modification à une valeur existante.
    ///
    /// C'est le seul endroit où la sémantique des trois variantes est
    /// interprétée : les encodeurs de format n'ont pas à la connaître.
    pub fn apply(self, current: Option<T>) -> Option<T> {
        match self {
            FieldEdit::Keep => current,
            FieldEdit::Clear => None,
            FieldEdit::Set(v) => Some(v),
        }
    }
}

/// Construit un `FieldEdit` depuis ce que le frontend envoie.
///
/// Convention côté interface : le champ absent du JSON signifie « ne touche
/// pas », une chaîne vide signifie « efface ». Sans cette règle, effacer un
/// titre depuis un formulaire serait impossible à exprimer.
impl FieldEdit<String> {
    pub fn from_optional(value: Option<String>) -> Self {
        match value {
            None => FieldEdit::Keep,
            Some(v) if v.trim().is_empty() => FieldEdit::Clear,
            Some(v) => FieldEdit::Set(v),
        }
    }
}

/// D'où viennent les octets d'une image de la liste finale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageSource {
    /// Image déjà dans le fichier, désignée par son identifiant de contenu
    /// (cf. `image_prep::image_id`). Ses octets ne transitent pas : on les
    /// recopie tels quels, sans réencodage — c'est ce qui permet de la
    /// promouvoir en pochette ou de la déplacer sans dégrader sa qualité.
    Existing(String),
    /// Nouvelles données, **déjà préparées** (redimensionnées et recompressées
    /// si nécessaire) par la couche appelante.
    ///
    /// Les dimensions accompagnent les octets parce que le bloc `PICTURE` du
    /// FLAC les stocke explicitement. Les redécoder ici pour les retrouver
    /// serait absurde : l'appelant vient de préparer l'image et les connaît.
    /// L'ID3, lui, les ignore.
    New {
        data: Vec<u8>,
        mime_type: String,
        width: u32,
        height: u32,
    },
}

/// Une image de la liste finale, avec le rôle qu'on lui donne.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageSlot {
    pub source: ImageSource,
    /// Type d'image ID3 : 3 = pochette avant, 4 = pochette arrière, 0 = autre…
    pub picture_type: u8,
    pub description: String,
}

/// Ce qu'on veut faire des images intégrées.
///
/// # Pourquoi une liste finale plutôt que cinq opérations
/// L'interface propose cinq gestes — ajouter, remplacer, supprimer, définir
/// comme pochette, réordonner. Les traduire en cinq commandes voudrait dire
/// cinq réécritures du fichier, cinq chemins d'erreur, et un état intermédiaire
/// observable si l'une échoue au milieu.
///
/// Décrire à la place **l'état visé** couvre les cinq par construction :
/// supprimer c'est omettre, réordonner c'est permuter, remplacer c'est
/// substituer la source d'un emplacement, définir la pochette c'est changer un
/// `picture_type`. Une seule écriture, atomique.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ImagePlan {
    /// Ne pas toucher aux images du fichier (défaut).
    #[default]
    Keep,
    /// Remplacer intégralement la liste par celle-ci, dans cet ordre.
    Replace(Vec<ImageSlot>),
}

impl ImagePlan {
    pub fn is_change(&self) -> bool {
        !matches!(self, ImagePlan::Keep)
    }
}

/// L'ensemble des modifications demandées sur un fichier.
///
/// Volontairement limité aux champs qu'un utilisateur corrige à la main. Les
/// champs techniques (encodeur, durée, ReplayGain…) ne sont pas éditables :
/// ils décrivent le fichier, ils ne s'inventent pas.
#[derive(Debug, Clone, Default)]
pub struct TagEdit {
    pub title: FieldEdit<String>,
    pub artist: FieldEdit<String>,
    pub album: FieldEdit<String>,
    pub album_artist: FieldEdit<String>,
    pub year: FieldEdit<String>,
    pub genre: FieldEdit<String>,
    pub comment: FieldEdit<String>,
    pub composer: FieldEdit<String>,
    pub track_number: FieldEdit<u16>,
    pub total_tracks: FieldEdit<u16>,
    pub disc_number: FieldEdit<u16>,
    pub total_discs: FieldEdit<u16>,
    pub images: ImagePlan,
}

impl TagEdit {
    /// Vrai si aucune modification n'est demandée — auquel cas on ne touche
    /// pas au fichier du tout. Réécrire un fichier « pour rien » est le
    /// meilleur moyen de le corrompre sans raison.
    pub fn is_empty(&self) -> bool {
        !self.title.is_change()
            && !self.artist.is_change()
            && !self.album.is_change()
            && !self.album_artist.is_change()
            && !self.year.is_change()
            && !self.genre.is_change()
            && !self.comment.is_change()
            && !self.composer.is_change()
            && !self.track_number.is_change()
            && !self.total_tracks.is_change()
            && !self.disc_number.is_change()
            && !self.total_discs.is_change()
            && !self.images.is_change()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keep_leaves_the_existing_value() {
        let e: FieldEdit<String> = FieldEdit::Keep;
        assert_eq!(e.apply(Some("Bowie".into())), Some("Bowie".into()));
    }

    #[test]
    fn clear_removes_the_value() {
        let e: FieldEdit<String> = FieldEdit::Clear;
        assert_eq!(e.apply(Some("Bowie".into())), None);
    }

    #[test]
    fn set_replaces_even_when_absent() {
        let e = FieldEdit::Set("Bowie".to_string());
        assert_eq!(e.apply(None), Some("Bowie".into()));
    }

    #[test]
    fn empty_string_from_the_ui_means_clear() {
        assert_eq!(FieldEdit::from_optional(Some("   ".into())), FieldEdit::Clear);
        assert_eq!(FieldEdit::from_optional(None), FieldEdit::Keep);
        assert_eq!(
            FieldEdit::from_optional(Some("Hunky Dory".into())),
            FieldEdit::Set("Hunky Dory".into())
        );
    }

    #[test]
    fn an_untouched_edit_changes_nothing() {
        assert!(TagEdit::default().is_empty());

        let mut edit = TagEdit::default();
        edit.album = FieldEdit::Set("Space Oddity".into());
        assert!(!edit.is_empty());
    }

    #[test]
    fn touching_only_the_images_still_counts_as_a_change() {
        // Sans ça, supprimer une pochette sans rien changer d'autre serait
        // considéré comme « rien à faire » et le fichier resterait intact.
        let mut edit = TagEdit::default();
        edit.images = ImagePlan::Replace(Vec::new());
        assert!(!edit.is_empty());
    }
}
