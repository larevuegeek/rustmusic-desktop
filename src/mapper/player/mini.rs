//! Ce que les gabarits du mini et du micro-lecteur lisent : format et qualité du morceau,
//! état de la chaîne audio, titres de la file, paroles ; tailles du mode et état de départ.

use karbon_ui::intl::{self, t};
use karbon_ui::storage;
use karbon_ui::ui::prelude::Color;
use karbon_ui::window::WindowState;

use crate::app::music_app::MusicApp;
use crate::entity::audio::audio_file::AudioFormat;
use crate::entity::queue::queue_track::QueueTrack;
use crate::models::app::Msg;
use crate::models::player::*;

/// Hauteur de la partie repliée du mini-lecteur (barre 34, pochette 4 + 84, commandes
/// 12 + 46, onglets 14 + 40 + 12).
pub const COLLAPSED_H: i32 = 246;
/// Côté du micro-lecteur, retenu d'une fois sur l'autre.
pub const MICRO_MIN: i32 = 220;
pub const MICRO_MAX: i32 = 660;
pub(crate) const MICRO_SIZE_KEY: &str = "rustmusic:micro-size";

impl MiniMsg {
    /// Gestionnaire `onwheel` : un cran vers le haut monte le volume de 5, vers le bas le
    /// baisse ; volume verrouillé (DoP) : l'événement passe.
    pub fn wheel(locked: bool, hint: bool) -> impl Fn(karbon_ui::ui::runtime::WheelEvent) -> Option<Msg> + 'static {
        move |e| {
            (!locked && e.delta_y != 0.).then_some(Msg::Mini(MiniMsg::Wheel { up: e.delta_y < 0., hint }))
        }
    }
}

/// Une ligne de l'onglet File ; `index` est son rang dans la file entière.
fn queue_row(index: usize, track: &QueueTrack) -> MiniQueueRow {
    MiniQueueRow {
        index,
        queue_id: track.queue_id.clone(),
        title: track.title.clone(),
        artist: track.artist.clone().unwrap_or_default(),
        cover: track.cover.clone(),
        duration: track.duration.filter(|d| *d > 0.).map(karbon_ui::intl::clock),
    }
}

/// L'onglet File : `current` est le morceau en cours, `window` les titres à venir bâtis,
/// `next_label` l'en-tête « Ensuite · 3 titres ».
pub fn queue_view(
    queue: &crate::query::queue::queue_state_row::QueueStateRow,
    current: Option<&QueueTrack>,
    window: &crate::models::shared::ListWindow,
    next_label: String,
) -> MiniQueueView {
    use crate::mapper::player::queue::{upcoming, upcoming_total};
    MiniQueueView {
        now_playing: current.map(|track| queue_row(0, track)),
        next_label,
        can_clear: upcoming_total(queue) > 0 && current.is_some(),
        rows: upcoming(queue, window).into_iter().map(|(index, track)| queue_row(index, track)).collect(),
    }
}

impl Default for MiniState {
    fn default() -> Self {
        let micro_size = storage::local()
            .get(MICRO_SIZE_KEY)
            .and_then(|v| v.parse::<i32>().ok())
            .filter(|n| (MICRO_MIN..=MICRO_MAX).contains(n))
            .unwrap_or(MICRO_MIN);
        Self {
            active: false,
            micro: false,
            pinned: true,
            tab: None,
            volume_open: false,
            volume_hint: false,
            hint_turn: 0,
            saved: None,
            normal: None,
            window: WindowState::default(),
            micro_size,
            height: COLLAPSED_H,
            anim: None,
        }
    }
}

impl MiniState {
    /// L'onglet des paroles est déroulé : elles se chargent comme dans le volet.
    pub fn lyrics_open(&self) -> bool {
        self.active && !self.micro && self.tab == Some(MiniTab::Lyrics)
    }
}

impl MusicApp {
    fn mini_dsd(&self) -> bool {
        self.data.player.audio.as_ref().is_some_and(|a| matches!(a.audio_format, AudioFormat::DSF | AudioFormat::DFF))
    }

    /// « FLAC », « DSD64 ».
    pub fn mini_format(&self) -> Option<String> {
        let audio = self.data.player.audio.as_ref()?;
        Some(if self.mini_dsd() {
            crate::helper::audio::dsd_label(audio.sample_rate)
        } else {
            format!("{:?}", audio.audio_format).to_uppercase()
        })
    }

    /// « 16/44,1 », « 44,1 kHz », « 2,82 MHz ».
    pub fn mini_quality(&self) -> String {
        let Some(audio) = self.data.player.audio.as_ref().filter(|a| a.sample_rate > 0) else {
            return String::new();
        };
        if self.mini_dsd() {
            return format!("{} MHz", intl::decimal(f64::from(audio.sample_rate) / 1_000_000., 2));
        }
        let khz = intl::decimal(f64::from(audio.sample_rate) / 1000., 1);
        if audio.bits_per_sample > 0 {
            format!("{}/{khz}", audio.bits_per_sample)
        } else {
            format!("{khz} kHz")
        }
    }

    /// Bit-perfect, exclusif, DoP, rééchantillonné, DSD en
    /// PCM, partagé.
    pub fn mini_chain(&self) -> Option<Chain> {
        let info = self.data.player.pipeline.as_ref()?;
        let mode = crate::mapper::player::pipeline::chain_mode(info, self.data.volume);
        let dot = match mode {
            "bit-perfect" => 0x34d399,
            "exclusive" => 0x38bdf8,
            "dop" => 0xa78bfa,
            "resampled" | "dsd" => 0xfbbf24,
            _ => 0x8f9893,
        };
        Some(Chain {
            label: t(crate::mapper::settings::badge_keys(mode).0),
            dot: Color::hex(dot),
        })
    }

    /// L'onglet File, quand il est ouvert.
    pub fn mini_queue(&self) -> Option<MiniQueueView> {
        (self.views.mini.tab == Some(MiniTab::Queue)).then(|| {
            queue_view(&self.data.queue, self.data.current_track(), &self.views.queue_window, self.mini_next_label())
        })
    }

    /// Infobulle du volume : « Volume : 80 % », ou volume verrouillé en DSD natif.
    pub fn mini_volume_title(&self) -> String {
        if self.mini_dop() {
            t("player.volume_dop_locked")
        } else {
            format!("{} : {} %", t("common.volume"), self.data.volume)
        }
    }

    /// En DSD natif, le volume se règle sur le DAC.
    pub fn mini_dop(&self) -> bool {
        self.data
            .player
            .pipeline
            .as_ref()
            .and_then(|p| p.get("backend"))
            .and_then(|v| v.as_str())
            .is_some_and(|b| b.ends_with("DoP"))
    }

    /// Le morceau chargé est aimé.
    pub fn mini_liked(&self) -> bool {
        self.data
            .player
            .path
            .as_ref()
            .is_some_and(|p| self.data.liked.contains(p))
    }

    /// L'épisode d'un podcast n'a pas de « J'aime ».
    pub fn mini_episode(&self) -> bool {
        self.data
            .player
            .audio
            .as_ref()
            .and_then(|a| a.tags.media_type.as_deref())
            .is_some_and(|m| m == "podcast")
    }

    /// « Ensuite · 1 titre », « Ensuite · {n} titres ».
    pub fn mini_next_label(&self) -> String {
        let n = self.data.upcoming().count();
        if n == 1 {
            t("mini.next_one")
        } else {
            t("mini.next_count").replace("{n}", &n.to_string())
        }
    }

    /// Titres à venir, en cours compris (pastille de l'onglet File).
    pub fn mini_queue_count(&self) -> usize {
        self.data.upcoming().count() + usize::from(self.data.current_track().is_some())
    }

    /// Pied des paroles : synchronisées ou non, et leur source.
    pub fn mini_lyrics_footer(&self) -> String {
        let ly = &self.views.lyrics;
        let kind = if ly.lines.is_empty() { t("mini.lyrics_plain") } else { t("mini.lyrics_synced") };
        match ly.source.as_str() {
            "sidecar" | "lrclib" | "manual" | "server" => format!("{kind} · {}", t(&format!("mini.src_{}", ly.source))),
            _ => kind,
        }
    }
}
