//! Ressources de l'interface : polices embarquées, images fixes (logo radio, « pas de
//! pochette ») et le chargeur des pochettes (`karbon_ui::resources::ImageLoader`). Les
//! templates y accèdent par `cx.resources` ; les icônes sont des SVG compilés
//! (`icons.rs`).
//!
//! Une pochette est désignée par son chemin, ou par `chemin#160` pour sa vignette
//! (`small_key`) : les vues passent ces clés à `cover` et à `preload`.

use std::sync::Arc;

use karbon_ui::application::Resources;
use karbon_ui::resources::{ImageDecoder, ImageLoadOptions, ImageLoader};
use karbon_ui::ui::{font::FontAsset, scene::Image, template::ImageSlot};

use crate::helper::library::covers::{self, SMALL_EDGE};

pub static FONTS: [FontAsset; 4] = [
    FontAsset {
        family: "Figtree Light",
        data: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/ui/assets/fonts/figtree.woff2")),
    },
    FontAsset {
        family: "Inter",
        data: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/ui/assets/fonts/inter.woff2")),
    },
    FontAsset {
        family: "JetBrains Mono",
        data: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/ui/assets/fonts/jetbrains-mono.woff2")),
    },
    // Afficheur de la télécommande d'ampli : police matricielle.
    FontAsset {
        family: "DotGothic16",
        data: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/ui/assets/fonts/dotgothic16.woff2")),
    },
];

/// Plus grand côté d'une pochette décodée : les cartes de l'accueil en font 260 px
/// logiques, à 150 % donc 390 px ; 448 garde une marge sans gonfler la mémoire.
const COVER_EDGE: u32 = 448;

/// La clé de la vignette d'une pochette : son chemin suivi de `#160`.
pub fn small_key(path: &str) -> String {
    format!("{path}#{SMALL_EDGE}")
}

/// Chemin et côté d'une clé (`chemin` ou `chemin#côté`).
pub(crate) fn split_key(key: &str) -> (&str, u32) {
    match key.rsplit_once('#') {
        Some((path, edge)) if !edge.is_empty() && edge.bytes().all(|b| b.is_ascii_digit()) => {
            (path, edge.parse().unwrap_or(COVER_EDGE))
        }
        _ => (key, COVER_EDGE),
    }
}

pub struct Assets {
    pub covers: ImageLoader,
    /// Décodeur des images reçues en mémoire (pochette de l'écran réseau de l'ampli).
    decoder: Option<Arc<dyn ImageDecoder>>,
    radio_logo: Arc<Image>,
    no_cd: Arc<Image>,
}

impl Assets {
    pub fn load(resources: &dyn Resources) -> karbon_ui::Result<Self> {
        Ok(Self {
            decoder: resources.image_decoder(),
            covers: ImageLoader::new(resources.image_decoder())
                .budget(128 * 1024 * 1024)
                .thumbnail_edge(COVER_EDGE)
                .resolve(covers::on_disk),
            radio_logo: resources.decode_image(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/ui/assets/radio_logo.png")))?,
            no_cd: resources.decode_image(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/ui/assets/no-cd.png")))?,
        })
    }

    /// Demande d'avance ces pochettes (chemins ou clés de vignettes) : celles de la vue
    /// sont connues dès que les données arrivent, avant le dessin.
    pub fn preload<'a>(&self, keys: impl IntoIterator<Item = &'a str>) {
        for key in keys {
            let (path, edge) = split_key(key);
            self.covers.preload_sized([path], edge);
        }
    }

    /// La pochette de ce chemin (ou clé), ou une image transparente le temps qu'elle
    /// arrive (repeinte alors sans reconstruire la vue).
    pub fn cover(&self, key: &str) -> ImageSlot {
        let (path, edge) = split_key(key);
        self.covers.load_sized(path, edge)
    }

    /// Comme `cover`, pour un chemin facultatif : « pas de pochette » sans chemin et le
    /// temps qu'elle arrive.
    pub fn cover_opt(&self, path: &Option<String>) -> ImageSlot {
        match path {
            Some(p) if !p.is_empty() => self.covers.load_or(p, COVER_EDGE, &self.no_cd),
            _ => ImageSlot::ready(self.no_cd.clone()),
        }
    }

    /// Identité de l'image en cache pour ce chemin (`cover_opt`) : elle change quand la
    /// pochette arrive ; entre dans la clé `memo={…}` d'une carte, pour qu'une carte
    /// reconstruite plus tard parte de la pochette arrivée.
    pub fn cover_id(&self, path: &Option<String>) -> usize {
        self.cached_id(path.as_deref(), COVER_EDGE)
    }

    /// Identité de la vignette en cache pour ce chemin (`cover_small`).
    pub fn cover_small_id(&self, path: &Option<String>) -> usize {
        self.cached_id(path.as_deref(), SMALL_EDGE)
    }

    /// Identité de l'image en cache pour cette clé (`cover`) : elle change quand l'image
    /// arrive.
    pub fn image_id(&self, key: &str) -> usize {
        let (path, edge) = split_key(key);
        self.cached_id(Some(path), edge)
    }

    fn cached_id(&self, path: Option<&str>, edge: u32) -> usize {
        match path.filter(|p| !p.is_empty()).and_then(|p| self.covers.peek_sized(p, edge)) {
            Some(image) => Arc::as_ptr(&image) as usize,
            None => Arc::as_ptr(&self.no_cd) as usize,
        }
    }

    /// La vignette (160 px) d'un chemin facultatif, transparente le temps qu'elle arrive
    /// (épingles de la barre latérale) : `cover` sur la clé de vignette.
    pub fn cover_thumb(&self, path: &Option<String>) -> ImageSlot {
        self.cover(&small_key(path.as_deref().unwrap_or_default()))
    }

    /// Une vignette (160 px) de ce chemin facultatif : la source dit `size="1x"`.
    pub fn cover_small(&self, path: &Option<String>) -> ImageSlot {
        match path {
            Some(p) if !p.is_empty() => self.covers.load_or(p, SMALL_EDGE, &self.no_cd),
            _ => ImageSlot::ready(self.no_cd.clone()),
        }
    }

    /// Comme `cover_small`, pour un chemin connu.
    pub fn cover_small_path(&self, path: &str) -> ImageSlot {
        if path.is_empty() {
            ImageSlot::ready(self.no_cd.clone())
        } else {
            self.covers.load_or(path, SMALL_EDGE, &self.no_cd)
        }
    }

    /// La pochette est connue (décodée) ; sinon la vue montre son repli.
    pub fn has_cover(&self, path: &Option<String>) -> bool {
        match path {
            Some(p) if !p.is_empty() => self.covers.get(p).is_some(),
            _ => false,
        }
    }

    /// Une image reçue en mémoire, décodée à la taille d'une pochette.
    pub fn decode(&self, bytes: &[u8]) -> Option<Arc<Image>> {
        let options = ImageLoadOptions { thumbnail_edge: COVER_EDGE, ..Default::default() };
        match self.decoder.as_ref()?.decode(bytes, options) {
            Ok(image) => Some(Arc::new(image)),
            Err(e) => {
                log::warn!("Image reçue illisible : {e}");
                None
            }
        }
    }

    pub fn radio_logo(&self) -> ImageSlot {
        ImageSlot::ready(self.radio_logo.clone())
    }
}
