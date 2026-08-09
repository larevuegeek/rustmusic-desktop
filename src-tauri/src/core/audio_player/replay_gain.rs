//! Replay Gain — normalisation du volume perçu entre morceaux et albums.
//!
//! # Principe
//! Les encodeurs (foobar2000, dBpoweramp, metaflac…) mesurent la sonie d'une
//! piste selon ReplayGain 1.0/2.0 (EBU R128) et écrivent le correctif à
//! appliquer dans les tags : `REPLAYGAIN_TRACK_GAIN` (« -6.54 dB »),
//! `REPLAYGAIN_ALBUM_GAIN`, plus les pics `_TRACK_PEAK` / `_ALBUM_PEAK`
//! (amplitude max, 1.0 = pleine échelle). On n'analyse rien nous-mêmes : on
//! lit ces tags et on applique un simple facteur multiplicatif.
//!
//! # Mode album vs piste
//! - **Album** : tous les morceaux d'un disque reçoivent le MÊME correctif,
//!   donc les écarts voulus par l'artiste (intro douce, final massif) sont
//!   préservés. C'est le mode recommandé pour l'écoute d'albums.
//! - **Piste** : chaque morceau est ramené au même niveau perçu. Adapté aux
//!   playlists mélangeant des sources hétérogènes.
//!
//! # Anti-écrêtage
//! Amplifier un morceau déjà proche de la pleine échelle le ferait saturer.
//! Quand le pic est connu, on plafonne le facteur à `1.0 / peak` : le gain
//! demandé est alors partiellement appliqué, mais jamais au prix d'une
//! distorsion.
//!
//! # Où le gain est appliqué (et où il ne l'est pas)
//! Le facteur est publié dans un atomic global lu par les boucles de sortie,
//! **exactement là où le volume logiciel est déjà appliqué** : CPAL partagé,
//! WASAPI exclusive PCM et DSD→PCM. Les chemins **DoP** (DSD natif, Windows /
//! macOS / Linux) écrivent leurs trames verbatim et n'appliquent ni volume ni
//! gain : ils restent bit-perfect par construction, sans traitement spécial
//! ici.

use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};

use symphonia::core::meta::{MetadataRevision, StandardTag};

/// Réglage utilisateur : quelle référence de gain appliquer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayGainMode {
    /// Aucun traitement (défaut) — le fichier est joué tel quel.
    Off,
    /// Gain par piste : chaque morceau au même niveau perçu.
    Track,
    /// Gain par album : cohérence à l'échelle du disque (recommandé).
    Album,
}

impl ReplayGainMode {
    pub fn parse_or_off(value: Option<&str>) -> Self {
        match value.map(str::to_ascii_lowercase).as_deref() {
            Some("track") => Self::Track,
            Some("album") => Self::Album,
            _ => Self::Off,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Track => "track",
            Self::Album => "album",
        }
    }

    fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Track,
            2 => Self::Album,
            _ => Self::Off,
        }
    }

    fn to_u8(self) -> u8 {
        match self {
            Self::Off => 0,
            Self::Track => 1,
            Self::Album => 2,
        }
    }
}

/// Valeurs Replay Gain lues dans les tags d'un fichier.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ReplayGainInfo {
    pub track_gain_db: Option<f32>,
    pub track_peak: Option<f32>,
    pub album_gain_db: Option<f32>,
    pub album_peak: Option<f32>,
}

impl ReplayGainInfo {
    /// Couple (gain dB, pic) à utiliser pour le mode demandé.
    ///
    /// On retombe sur l'autre référence quand celle demandée manque : mieux
    /// vaut un gain album sur une piste isolée que pas de normalisation du
    /// tout (cas courant des fichiers taggués par un seul outil).
    fn pick(&self, mode: ReplayGainMode) -> (Option<f32>, Option<f32>) {
        match mode {
            ReplayGainMode::Off => (None, None),
            ReplayGainMode::Album => match (self.album_gain_db, self.track_gain_db) {
                (Some(g), _) => (Some(g), self.album_peak.or(self.track_peak)),
                (None, Some(g)) => (Some(g), self.track_peak),
                _ => (None, None),
            },
            ReplayGainMode::Track => match (self.track_gain_db, self.album_gain_db) {
                (Some(g), _) => (Some(g), self.track_peak.or(self.album_peak)),
                (None, Some(g)) => (Some(g), self.album_peak),
                _ => (None, None),
            },
        }
    }
}

// ─── État global lu par les threads audio ────────────────────────────────
//
// Même approche que `output/preference.rs` : les boucles de sortie tournent
// hors runtime tokio et ne peuvent pas toucher la BDD. On publie donc les
// réglages dans des atomics, synchronisés au boot puis à chaque changement.

static MODE: AtomicU8 = AtomicU8::new(0);
/// Pré-ampli en centièmes de dB (évite un atomic flottant pour un réglage
/// que l'utilisateur exprime au dixième de dB près).
static PREAMP_CENTI_DB: AtomicU32 = AtomicU32::new(0);
/// Facteur linéaire effectif du morceau en cours, en bits de `f32`.
static CURRENT_FACTOR: AtomicU32 = AtomicU32::new(1.0f32.to_bits());

pub fn mode() -> ReplayGainMode {
    ReplayGainMode::from_u8(MODE.load(Ordering::Relaxed))
}

pub fn set_mode(m: ReplayGainMode) {
    MODE.store(m.to_u8(), Ordering::Relaxed);
    if m == ReplayGainMode::Off {
        reset_current_factor();
    }
}

pub fn preamp_db() -> f32 {
    PREAMP_CENTI_DB.load(Ordering::Relaxed) as i32 as f32 / 100.0
}

pub fn set_preamp_db(db: f32) {
    let clamped = db.clamp(-15.0, 15.0);
    PREAMP_CENTI_DB.store((clamped * 100.0).round() as i32 as u32, Ordering::Relaxed);
}

/// Facteur linéaire à appliquer aux échantillons du morceau en cours.
/// Vaut `1.0` quand le Replay Gain est inactif ou que le fichier n'a pas
/// de tags — le chemin audio reste alors strictement inchangé.
#[inline]
pub fn current_factor() -> f32 {
    f32::from_bits(CURRENT_FACTOR.load(Ordering::Relaxed))
}

pub fn reset_current_factor() {
    CURRENT_FACTOR.store(1.0f32.to_bits(), Ordering::Relaxed);
}

/// Calcule et publie le facteur pour le morceau qui démarre.
///
/// À appeler au lancement de chaque piste, **y compris** quand aucune info
/// n'a été trouvée : sans ça le facteur du morceau précédent resterait actif.
pub fn apply_track(info: ReplayGainInfo) {
    let m = mode();
    let preamp = preamp_db();

    let Some(outcome) = compute_factor(&info, m, preamp) else {
        reset_current_factor();
        if m != ReplayGainMode::Off {
            log::debug!("🔊 Replay Gain : aucun tag exploitable, lecture sans correction");
        }
        return;
    };

    CURRENT_FACTOR.store(outcome.factor.to_bits(), Ordering::Relaxed);
    log::debug!(
        "🔊 Replay Gain {:?} : {:+.2} dB (préampli {:+.1} dB) → ×{:.3}{}",
        m,
        outcome.gain_db,
        preamp,
        outcome.factor,
        if outcome.clipping_limited {
            " (limité anti-écrêtage)"
        } else {
            ""
        }
    );
}

/// Facteur linéaire qu'aurait cette piste avec les réglages courants, **sans**
/// toucher à l'état global.
///
/// Sert au préchargement : on calcule le gain de la piste suivante pendant
/// qu'elle est décodée, et il n'est publié qu'au moment de sa promotion —
/// donc exactement quand ses échantillons commencent à sortir.
pub fn factor_for(info: ReplayGainInfo) -> f32 {
    compute_factor(&info, mode(), preamp_db())
        .map(|o| o.factor)
        .unwrap_or(1.0)
}

/// Publie directement un facteur déjà calculé (cf. [`factor_for`]).
pub fn set_current_factor(factor: f32) {
    CURRENT_FACTOR.store(factor.to_bits(), Ordering::Relaxed);
}

/// Résultat du calcul de gain pour un morceau.
#[derive(Debug, Clone, Copy, PartialEq)]
struct GainOutcome {
    /// Facteur linéaire à appliquer aux échantillons.
    factor: f32,
    /// Gain du tag retenu (hors pré-ampli), pour le journal.
    gain_db: f32,
    /// `true` quand l'anti-écrêtage a rogné le gain demandé.
    clipping_limited: bool,
}

/// Cœur du calcul, sans état global — c'est ce que couvrent les tests.
/// Retourne `None` quand aucun gain n'est applicable (mode Off ou tags absents).
fn compute_factor(
    info: &ReplayGainInfo,
    mode: ReplayGainMode,
    preamp_db: f32,
) -> Option<GainOutcome> {
    let (gain_db, peak) = info.pick(mode);
    let gain_db = gain_db?;

    let mut factor = 10f32.powf((gain_db + preamp_db) / 20.0);

    // Anti-écrêtage : ne jamais pousser le pic au-delà de la pleine échelle.
    let mut clipping_limited = false;
    if let Some(peak) = peak {
        if peak > 0.0 && factor * peak > 1.0 {
            factor = 1.0 / peak;
            clipping_limited = true;
        }
    }

    // Garde-fou : un tag aberrant ne doit pas exploser le niveau.
    factor = factor.clamp(0.01, 4.0);

    Some(GainOutcome {
        factor,
        gain_db,
        clipping_limited,
    })
}

// ─── Parsing des tags ────────────────────────────────────────────────────

/// Parse un gain façon `« -6.54 dB »`, `« +3,2 dB »` ou `« -6.54 »`.
fn parse_gain_db(raw: &str) -> Option<f32> {
    let cleaned = raw
        .trim()
        .trim_end_matches(|c: char| c.is_ascii_alphabetic() || c.is_whitespace())
        .replace(',', ".");
    cleaned.trim().parse::<f32>().ok().filter(|v| v.is_finite())
}

/// Parse un pic (`« 0.988754 »`). Certains encodeurs écrivent le pic en dB
/// (valeur négative) : on l'ignore alors plutôt que de mal l'interpréter.
fn parse_peak(raw: &str) -> Option<f32> {
    let v = raw.trim().replace(',', ".").parse::<f32>().ok()?;
    (v.is_finite() && v > 0.0).then_some(v)
}

/// Extrait le Replay Gain d'une révision de métadonnées Symphonia
/// (FLAC, MP3, OGG, WAV, M4A…).
pub fn from_metadata(rev: &MetadataRevision) -> ReplayGainInfo {
    let mut info = ReplayGainInfo::default();

    let all = rev
        .media
        .tags
        .iter()
        .chain(rev.per_track.iter().flat_map(|pt| pt.metadata.tags.iter()));

    for tag in all {
        // Symphonia 0.6 type les clés ReplayGain quel que soit le conteneur
        // (TXXX ID3v2, Vorbis comment, MP4 `----`), donc pas besoin de
        // matcher les clés brutes format par format.
        match &tag.std {
            Some(StandardTag::ReplayGainTrackGain(v)) => info.track_gain_db = parse_gain_db(v),
            Some(StandardTag::ReplayGainTrackPeak(v)) => info.track_peak = parse_peak(v),
            Some(StandardTag::ReplayGainAlbumGain(v)) => info.album_gain_db = parse_gain_db(v),
            Some(StandardTag::ReplayGainAlbumPeak(v)) => info.album_peak = parse_peak(v),
            _ => {}
        }
    }

    info
}

/// Extrait le Replay Gain des paires clé/valeur produites par l'extracteur
/// maison (DSF/DFF : frames `TXXX` ID3v2 rangées dans `custom_tags`).
pub fn from_custom_tags(custom_tags: &[(String, String)]) -> ReplayGainInfo {
    let mut info = ReplayGainInfo::default();

    for (key, value) in custom_tags {
        match key.trim().to_ascii_uppercase().as_str() {
            "REPLAYGAIN_TRACK_GAIN" => info.track_gain_db = parse_gain_db(value),
            "REPLAYGAIN_TRACK_PEAK" => info.track_peak = parse_peak(value),
            "REPLAYGAIN_ALBUM_GAIN" => info.album_gain_db = parse_gain_db(value),
            "REPLAYGAIN_ALBUM_PEAK" => info.album_peak = parse_peak(value),
            _ => {}
        }
    }

    info
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_gain_formats() {
        assert_eq!(parse_gain_db("-6.54 dB"), Some(-6.54));
        assert_eq!(parse_gain_db("+3.20 dB"), Some(3.20));
        assert_eq!(parse_gain_db("  -1,5 dB "), Some(-1.5));
        assert_eq!(parse_gain_db("0"), Some(0.0));
        assert_eq!(parse_gain_db("n/a"), None);
    }

    #[test]
    fn rejects_peak_expressed_in_db() {
        assert_eq!(parse_peak("0.988754"), Some(0.988754));
        assert_eq!(parse_peak("-3.2"), None);
        assert_eq!(parse_peak(""), None);
    }

    #[test]
    fn album_mode_falls_back_to_track_gain() {
        let info = ReplayGainInfo {
            track_gain_db: Some(-4.0),
            track_peak: Some(0.9),
            ..Default::default()
        };
        assert_eq!(info.pick(ReplayGainMode::Album), (Some(-4.0), Some(0.9)));
    }

    #[test]
    fn off_mode_yields_nothing() {
        let info = ReplayGainInfo {
            album_gain_db: Some(-8.0),
            ..Default::default()
        };
        assert_eq!(info.pick(ReplayGainMode::Off), (None, None));
    }

    #[test]
    fn clipping_guard_limits_amplification() {
        // +12 dB demandés sur un morceau qui touche déjà 0.99 → on plafonne.
        let out = compute_factor(
            &ReplayGainInfo {
                track_gain_db: Some(12.0),
                track_peak: Some(0.99),
                ..Default::default()
            },
            ReplayGainMode::Track,
            0.0,
        )
        .expect("un gain doit être calculé");
        assert!(out.clipping_limited);
        assert!(
            (out.factor - 1.0 / 0.99).abs() < 1e-4,
            "facteur inattendu : {}",
            out.factor
        );
    }

    #[test]
    fn attenuation_is_applied_as_is() {
        // -6.02 dB ≈ moitié d'amplitude, aucun risque d'écrêtage.
        let out = compute_factor(
            &ReplayGainInfo {
                album_gain_db: Some(-6.02),
                album_peak: Some(0.95),
                ..Default::default()
            },
            ReplayGainMode::Album,
            0.0,
        )
        .expect("un gain doit être calculé");
        assert!(!out.clipping_limited);
        assert!((out.factor - 0.5).abs() < 1e-3, "facteur : {}", out.factor);
    }

    #[test]
    fn preamp_shifts_the_result() {
        let base = compute_factor(
            &ReplayGainInfo {
                track_gain_db: Some(-6.0),
                ..Default::default()
            },
            ReplayGainMode::Track,
            0.0,
        )
        .unwrap();
        let boosted = compute_factor(
            &ReplayGainInfo {
                track_gain_db: Some(-6.0),
                ..Default::default()
            },
            ReplayGainMode::Track,
            6.0,
        )
        .unwrap();
        assert!(boosted.factor > base.factor);
        assert!((boosted.factor - 1.0).abs() < 1e-3);
    }

    #[test]
    fn missing_tags_yield_no_gain() {
        assert!(compute_factor(&ReplayGainInfo::default(), ReplayGainMode::Album, 0.0).is_none());
    }

    #[test]
    fn off_mode_never_applies_gain() {
        let info = ReplayGainInfo {
            track_gain_db: Some(-8.0),
            album_gain_db: Some(-7.0),
            ..Default::default()
        };
        assert!(compute_factor(&info, ReplayGainMode::Off, 0.0).is_none());
    }
}
