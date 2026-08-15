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
    /// Remplacer la seule **pochette avant**, et laisser les autres images en
    /// place.
    ///
    /// # Pourquoi cette variante existe
    /// `Replace` décrit la liste finale, ce qui suppose de connaître les images
    /// du fichier. C'est vrai quand on en édite un seul, faux dès qu'on en
    /// traite cinquante : poser une pochette commune avec `Replace`
    /// supprimerait au passage les livrets et pochettes arrière de chacun,
    /// sans que personne ne l'ait demandé.
    SetCover(ImageSlot),
}

/// Ce qu'un conteneur sait dire d'une image déjà présente dans le fichier.
///
/// Sert d'intermédiaire entre les formats : l'ID3 et le FLAC stockent leurs
/// images très différemment, mais tous deux savent produire ces trois
/// informations, et c'est tout ce dont `resolve` a besoin.
#[derive(Debug, Clone)]
pub struct ExistingImage {
    /// Identifiant de contenu (cf. `image_prep::image_id`).
    pub id: String,
    pub picture_type: u8,
    pub description: String,
}

/// Code ID3 de la pochette avant, partagé par l'ID3 et le FLAC.
pub const PICTURE_TYPE_COVER_FRONT: u8 = 3;

impl ImagePlan {
    pub fn is_change(&self) -> bool {
        !matches!(self, ImagePlan::Keep)
    }

    /// Réduit le plan à une liste finale, connaissant les images du fichier.
    ///
    /// `None` pour `Keep`. Les encodeurs n'ont donc qu'un seul cas à traiter :
    /// une liste d'emplacements dans l'ordre voulu.
    pub fn resolve(&self, existing: &[ExistingImage]) -> Option<Vec<ImageSlot>> {
        match self {
            ImagePlan::Keep => None,
            ImagePlan::Replace(slots) => Some(slots.clone()),
            ImagePlan::SetCover(cover) => {
                // La nouvelle pochette en tête, puis tout ce qui n'est pas une
                // pochette avant — c'est ce qui préserve les images d'un
                // fichier dont l'appelant ignore le contenu.
                let mut slots = Vec::with_capacity(existing.len() + 1);
                slots.push(ImageSlot {
                    picture_type: PICTURE_TYPE_COVER_FRONT,
                    ..cover.clone()
                });
                slots.extend(
                    existing
                        .iter()
                        .filter(|image| image.picture_type != PICTURE_TYPE_COVER_FRONT)
                        .map(|image| ImageSlot {
                            source: ImageSource::Existing(image.id.clone()),
                            picture_type: image.picture_type,
                            description: image.description.clone(),
                        }),
                );
                Some(slots)
            }
        }
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

    fn new_slot(kind: u8) -> ImageSlot {
        ImageSlot {
            source: ImageSource::New {
                data: b"nouvelle-pochette".to_vec(),
                mime_type: "image/jpeg".into(),
                width: 1200,
                height: 1200,
            },
            picture_type: kind,
            description: String::new(),
        }
    }

    fn existing(id: &str, kind: u8) -> ExistingImage {
        ExistingImage {
            id: id.into(),
            picture_type: kind,
            description: String::new(),
        }
    }

    #[test]
    fn set_cover_keeps_the_other_images() {
        // Le cœur de la variante : en édition multiple on ignore ce que
        // contiennent les autres fichiers, il ne faut donc rien y supprimer.
        let plan = ImagePlan::SetCover(new_slot(0));
        let slots = plan
            .resolve(&[
                existing("ancienne-pochette", PICTURE_TYPE_COVER_FRONT),
                existing("livret", 5),
                existing("verso", 4),
            ])
            .unwrap();

        assert_eq!(slots.len(), 3, "une image a été perdue ou dupliquée");
        assert!(
            matches!(slots[0].source, ImageSource::New { .. }),
            "la nouvelle pochette doit venir en tête"
        );
        assert_eq!(
            slots[0].picture_type, PICTURE_TYPE_COVER_FRONT,
            "le type de la pochette doit être imposé, pas hérité de l'appelant"
        );
        assert_eq!(
            slots[1].source,
            ImageSource::Existing("livret".into()),
            "le livret a disparu"
        );
        assert_eq!(slots[2].source, ImageSource::Existing("verso".into()));
    }

    #[test]
    fn set_cover_on_a_file_without_images_just_adds_it() {
        let slots = ImagePlan::SetCover(new_slot(0)).resolve(&[]).unwrap();
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].picture_type, PICTURE_TYPE_COVER_FRONT);
    }

    #[test]
    fn keep_resolves_to_nothing_at_all() {
        // Les encodeurs s'appuient dessus pour ne pas toucher aux images.
        assert!(ImagePlan::Keep.resolve(&[existing("x", 3)]).is_none());
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
