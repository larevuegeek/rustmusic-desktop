import { writable, get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { platformFeatures, loadPlatformFeatures } from "#lib/stores/settings/platform.store";
import { enable as enableAutostart, disable as disableAutostart, isEnabled as isAutostartEnabled } from "@tauri-apps/plugin-autostart";
import {
  applyThemeMode,
  applyContrastMode,
  applyFavoritesMode,
  applyLightEffects,
  type ThemeMode,
  type ContrastMode,
} from "#lib/helper/theme/theme";

export type AppSettings = {
  language: string;
  auto_start: string;
  minimize_to_tray: string;
  show_notifications: string;
  scan_on_startup: string;
  audio_quality: string;
  hardware_acceleration: string;
  single_click_play: string;
  show_sleep_timer: string;          // 'true' | 'false' — bouton minuteur de veille dans le header
  show_whats_new_button: string;     // 'true' | 'false' — mégaphone des nouveautés dans le header
  show_theme_toggle: string;         // 'true' | 'false' — bascule clair / sombre dans le header
  system_media_controls: string;     // 'true' | 'false' — SMTC / MPRIS / Now Playing
  wasapi_exclusive: string;          // 'true' | 'false' — Windows uniquement, bit-perfect
  dsd_dop: string;                   // 'true' | 'false' — DSD natif (DoP) via WASAPI exclusive
  gapless: string;                   // 'true' | 'false' — enchaînement sans blanc entre pistes
  resume_playback: string;           // 'true' | 'false' — Lecture reprend à la position enregistrée
  play_on_start: string;             // 'true' | 'false' — Le dernier morceau repart au lancement
  listen_threshold: string;          // 'auto' (1 min ou la moitié) | '25' | '50' | '75' | '80' | '90' — % écouté pour compter
  prevent_sleep: string;             // 'true' | 'false' — pas de veille automatique pendant la lecture
  theme: string;                     // 'auto' | 'light' | 'dark'
  contrast: string;                  // 'normal' | 'high' — lisibilité renforcée
  accent_color: string;              // '#rrggbb' ; vide = vert d'origine
  window_controls_style: string;     // 'auto' | 'macos' | 'windows'
  window_controls_position: string;  // 'right' | 'left'
  mini_volume_layout: string;        // 'vertical' | 'horizontal' — bulle du volume du mini-lecteur
  shortcuts: string;                 // JSON des raccourcis modifiés ; vide = touches par défaut
  blurred_backgrounds: string;       // 'precomputed' | 'live' — fonds de pochette floutés
  visual_effects: string;            // 'auto' | 'full' | 'light' — flous et fondus allégés
  // 'true' | 'false' — interroger Deezer à l'ouverture d'une fiche artiste.
  // Par défaut actif : c'est le comportement historique, et le couper sans
  // le dire priverait les bibliothèques existantes de leurs portraits.
  auto_download_artist_images: string;
  // 'true' | 'false' — un réglage par emplacement : dans la barre ce sont des
  // destinations, sur l'accueil des indicateurs. Visibles par défaut.
  show_liked_in_playlists: string;
  show_recent_in_playlists: string;
  show_liked_in_home: string;
  show_recent_in_home: string;
  // 'true' | 'false' — boutons « Ouvrir » de la barre latérale. Masqués par
  // défaut : l'accueil propose déjà les deux actions.
  show_open_buttons: string;
  // 'true' | 'false' — le cœur des favoris et son entrée de menu.
  show_favorites: string;
  // 'sidebar' | 'top' | 'both' — où proposer les sections. 'both' d'origine.
  library_tabs_position: string;
  // Sections retenues, en JSON. Voir `#lib/config/libraryTabs`.
  library_tabs: string;
  // Colonnes de l'onglet Morceaux, en JSON : un tableau de clés, dans l'ordre
  // d'affichage. Voir `#lib/config/trackColumns`.
  track_columns: string;
  // Largeurs réglées à la main, en JSON : clé de colonne → pixels. Les colonnes
  // absentes gardent leur largeur d'origine.
  track_column_widths: string;
  // 'true' | 'false' — barre latérale repliée sur ses icônes.
  sidebar_collapsed: string;
  // 'true' | 'false' — sections d'épinglés de la barre latérale. Visibles par défaut.
  show_sidebar_playlists: string;
  network_players: string;          // 'true' | 'false' — lecteurs réseau (UPnP), désactivés par défaut
  airplay: string;                  // 'true' | 'false' — appareils AirPlay, désactivés par défaut
  amp_remote: string;               // 'true' | 'false' — télécommande d'ampli, désactivée par défaut
  amp_remote_open: string;          // 'true' | 'false' — panneau de la télécommande sorti
  amp_remote_mini: string;          // 'true' | 'false' — télécommande réduite à l'afficheur et au volume
  podcasts: string;                 // 'true' | 'false' — podcasts, désactivés par défaut
  podcast_play_mode: string;
  podcast_speed: string;            // vitesse des émissions sans vitesse propre : '0.75' … '2'
  podcast_refresh: string;          // vérification des nouveaux épisodes : 'never' | '1h' | '6h' | '24h'
  podcast_cleanup: string;          // épisodes écoutés effacés : 'never' | 'now' | '24h' | '7d' | '30d'        // 'stream' (pendant le téléchargement) | 'download' (après)
  podcast_country: string;          // pays de l'annuaire (« FR »…) ; vide = celui de l'ordinateur
  radios: string;                   // 'true' | 'false' — radios, désactivées par défaut
  radio_country: string;            // pays des radios populaires ; vide = celui de l'ordinateur
  // Ordre des sections de la barre latérale, en JSON (`playlists`, `online`, `pins`).
  sidebar_order: string;
  // Ordre de la section Playlists, en JSON : `playlist:<id>`, `liked`, `recent`.
  sidebar_playlists_order: string;
  // 'true' : la carte « Épinglez vos favoris » a été fermée (les sections restent).
  pins_hint_dismissed: string;
  // Source affichée : '0' = cet ordinateur, '<id>' = un serveur, 'all' = toutes.
  current_source: string;
  // Résultat en double (local + serveur) en « Toutes les sources » : version ouverte d'abord, 'local' | 'server'.
  duplicate_version: string;
  show_pinned_albums: string;
  show_pinned_artists: string;
  // 'true' | 'false' — pied de la barre réduit à une ligne, par sa languette.
  sidebar_footer_collapsed: string;
  // 'true' | 'false' — termes techniques soulignés, expliqués au clic.
  show_help_bubbles: string;
};

export const settingsDefaults: AppSettings = {
  language: 'fr',
  auto_start: 'false',
  minimize_to_tray: 'true',
  show_notifications: 'false',
  scan_on_startup: 'false',
  audio_quality: 'high',
  hardware_acceleration: 'true',
  blurred_backgrounds: 'precomputed',
  visual_effects: 'auto',
  single_click_play: 'false',
  show_sleep_timer: 'true',
  show_whats_new_button: 'true',
  show_theme_toggle: 'true',
  system_media_controls: 'true',
  wasapi_exclusive: 'false',
  dsd_dop: 'false',
  gapless: 'true',
  resume_playback: 'true',
  play_on_start: 'false',
  listen_threshold: 'auto',
  prevent_sleep: 'true',
  theme: 'dark',
  contrast: 'normal',
  accent_color: '',
  window_controls_style: 'auto',
  window_controls_position: 'right',
  mini_volume_layout: 'vertical',
  shortcuts: '',
  auto_download_artist_images: 'true',
  show_liked_in_playlists: 'true',
  show_recent_in_playlists: 'true',
  show_liked_in_home: 'true',
  show_recent_in_home: 'true',
  show_open_buttons: 'true',
  show_favorites: 'true',
  library_tabs_position: 'both',
  library_tabs: '["tracks","artists","albums","genres","folders"]',
  track_columns: '["artist","album","rating","duration"]',
  track_column_widths: '{}',
  sidebar_collapsed: 'false',
  show_sidebar_playlists: 'true',
  network_players: 'false',
  airplay: 'false',
  amp_remote: 'false',
  amp_remote_open: 'false',
  amp_remote_mini: 'false',
  podcasts: 'false',
  podcast_play_mode: 'stream',
  podcast_speed: '1',
  podcast_refresh: '6h',
  podcast_cleanup: '24h',
  podcast_country: '',
  radios: 'false',
  radio_country: '',
  sidebar_order: '["playlists","online","pins"]',
  sidebar_playlists_order: '[]',
  pins_hint_dismissed: 'false',
  current_source: '0',
  duplicate_version: 'local',
  show_pinned_albums: 'true',
  show_pinned_artists: 'true',
  sidebar_footer_collapsed: 'false',
  show_help_bubbles: 'true',
};

// Actions spéciales par clé — exécutées APRÈS la sauvegarde en BDD
// C'est ici qu'on câble les effets réels de chaque paramètre
const sideEffects: Partial<Record<keyof AppSettings, (value: string) => Promise<void>>> = {

  // Lancement au démarrage : on appelle le plugin Tauri autostart
  // Il écrit/supprime l'entrée dans le registre Windows (ou LaunchAgent macOS)
  auto_start: async (value: string) => {
    try {
      // Flatpak : le fichier d'autostart du plugin resterait dans le bac à sable ; le portail du système s'en charge.
      if (get(platformFeatures).flatpak) {
        await invoke('set_autostart_portal', { enabled: value === 'true' });
        return;
      }
      if (value === 'true') {
        await enableAutostart();
      } else {
        await disableAutostart();
      }
    } catch (e) {
      console.error('[settings] Erreur autostart:', e);
    }
  },

  // Thème : applique la classe `.dark` sur <html> et écoute la préférence
  // système si mode 'auto'. Appelé aussi au chargement initial.
  theme: async (value: string) => {
    applyThemeMode((value as ThemeMode) ?? 'auto');
  },

  // Contraste : pose `data-contrast` sur <html>, les règles CSS font le reste.
  contrast: async (value: string) => {
    applyContrastMode((value as ContrastMode) ?? 'normal');
  },

  show_favorites: async (value: string) => {
    applyFavoritesMode(value !== 'false');
  },

  // Effets visuels : `auto` allège sur rendu logiciel, GPU faible ou machine modeste.
  visual_effects: applyVisualEffects,

  // SMTC / MPRIS / Now Playing : on appelle le service qui parle au backend
  // souvlaki pour enregistrer/désenregistrer l'app auprès de l'OS.
  // Import dynamique pour éviter une dépendance circulaire au load.
  system_media_controls: async (_value: string) => {
    try {
      const { mediaControlsService } = await import('#lib/services/mediaControls/mediaControls.service');
      await mediaControlsService.sync();
    } catch (e) {
      console.error('[settings] Erreur sync SMTC:', e);
    }
  },

  // Exclusif : lu par le Rust au lancement d'un morceau, d'où la relance au même endroit.
  wasapi_exclusive: async (value: string) => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const { playerService } = await import('#lib/services/player/player.service');
      await playerService.applyOutputChange(
        () => invoke('set_wasapi_exclusive_preference', { enabled: value === 'true' }),
        { localOnly: true },
      );
    } catch (e) {
      console.error('[settings] Erreur sync WASAPI preference:', e);
    }
  },

  // Lecteurs réseau : le moteur refuse recherche et choix tant que c'est coupé.
  network_players: async (value: string) => {
    try {
      const { networkPlayerStore } = await import('#lib/stores/audio/networkPlayer.store');
      await networkPlayerStore.setEnabled('upnp', value === 'true');
    } catch (e) {
      console.error('[settings] Erreur lecteurs réseau:', e);
    }
  },

  // Télécommande : le moteur ne contacte aucun ampli tant que c'est coupé.
  amp_remote: async (value: string) => {
    try {
      const { ampRemoteStore } = await import('#lib/stores/audio/ampRemote.store');
      await ampRemoteStore.setEnabled(value === 'true');
    } catch (e) {
      console.error('[settings] Erreur télécommande:', e);
    }
  },

  // Podcasts : coupés, aucun flux n'est contacté.
  podcasts: async (value: string) => {
    try {
      await invoke('set_podcasts_enabled', { enabled: value === 'true' });
    } catch (e) {
      console.error('[settings] Erreur podcasts:', e);
    }
  },

  // Radios : coupées, l'annuaire n'est pas contacté.
  radios: async (value: string) => {
    try {
      await invoke('set_radios_enabled', { enabled: value === 'true' });
    } catch (e) {
      console.error('[settings] Erreur radios:', e);
    }
  },

  airplay: async (value: string) => {
    try {
      const { networkPlayerStore } = await import('#lib/stores/audio/networkPlayer.store');
      await networkPlayerStore.setEnabled('airplay', value === 'true');
    } catch (e) {
      console.error('[settings] Erreur AirPlay:', e);
    }
  },

  gapless: async (value: string) => {
    try {
      await invoke('set_gapless', { enabled: value !== 'false' });
    } catch (e) {
      console.warn('[settings] Erreur sync gapless:', e);
    }
  },

  // DSD natif (DoP) : pousse la préférence vers l'atomique global Rust, lu au
  // prochain morceau DSD. Effet réel seulement si le DAC accepte le rate
  // porteur (et, sur Windows uniquement, si WASAPI exclusive est actif) —
  // sinon fallback DSD2PCM silencieux.
  dsd_dop: async (value: string) => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const { playerService } = await import('#lib/services/player/player.service');
      const { player } = await import('#lib/stores/player/player.store');
      // Relance seulement si le morceau en cours est un DSD : rien ne change pour un PCM.
      const dsd = /\.(dsf|dff|dsd)$/i.test(get(player).pathFile ?? '');
      await playerService.applyOutputChange(
        () => invoke('set_dop_preference', { enabled: value === 'true' }),
        { replay: dsd, localOnly: true },
      );
    } catch (e) {
      console.error('[settings] Erreur sync DoP preference:', e);
    }
  },
};

export type LightEffectsAuto = { light: boolean; reason: string | null };

// Ce que décide `auto` : la machine ne change pas en cours de session, une
// seule question au backend suffit.
let effetsAuto: Promise<LightEffectsAuto> | null = null;
export function lightEffectsAuto(): Promise<LightEffectsAuto> {
  effetsAuto ??= invoke<LightEffectsAuto>('get_light_effects_auto')
    .catch(() => ({ light: false, reason: null }));
  return effetsAuto;
}

/** Effets allégés en ce moment, réglage `auto` résolu. */
export const effetsLegers = writable(false);

async function applyVisualEffects(value: string) {
  const legers = value === 'light' || (value !== 'full' && (await lightEffectsAuto()).light);
  applyLightEffects(legers);
  effetsLegers.set(legers);
}

const settingsWriter = writable<AppSettings>({ ...settingsDefaults });

const LANGUAGES = ['fr', 'en', 'de', 'es', 'it'];
/** Première langue du système que l'appli parle (« de-AT » → de), sinon l'anglais. */
function systemLanguage(): string {
  const prefs = typeof navigator === 'undefined' ? [] : [...(navigator.languages ?? []), navigator.language];
  return prefs.map((l) => l?.slice(0, 2).toLowerCase()).find((l) => LANGUAGES.includes(l)) ?? 'en';
}

export const settingsStore = {
  subscribe: settingsWriter.subscribe,

  // Chargement initial depuis la BDD
  // On synchronise aussi l'état réel de l'autostart avec ce que dit le registre
  init: async () => {
    try {
      const all = await invoke<Record<string, string>>('get_all_settings');

      settingsWriter.update(state => ({
        ...state,
        ...Object.fromEntries(
          Object.entries(all).filter(([key]) => key in settingsDefaults)
        ),
      }));

      // Thème et contraste appliqués **avant** la vérification de l'autostart.
      //
      // Celle-ci interroge le registre Windows ou le fichier d'autostart Linux,
      // et peut prendre plusieurs secondes. Elle se trouvait avant : le thème
      // attendait donc derrière elle, et l'interface restait dans la couleur du
      // démarrage tout ce temps — jusqu'à une dizaine de secondes sur certaines
      // machines. L'ordre seul règle ça ; rien d'autre ne change.
      // Installation neuve (aucun réglage d'interface) : langue du système. Mise à jour sans choix : le français déjà affiché.
      if (!('language' in all)) {
        const neuve = !Object.keys(all).some((key) => key in settingsDefaults);
        void settingsStore.set('language', neuve ? systemLanguage() : 'fr');
      }

      const finalTheme = get(settingsWriter).theme;
      applyThemeMode((finalTheme as ThemeMode) ?? 'auto');
      applyContrastMode((get(settingsWriter).contrast as ContrastMode) ?? 'normal');
      applyFavoritesMode(get(settingsWriter).show_favorites !== 'false');
      void applyVisualEffects(get(settingsWriter).visual_effects);

      // L'autostart en fond, sans retenir `init` : il interroge le registre et
      // peut prendre des secondes, pendant lesquelles l'interface attendrait
      // ses réglages. Personne ne le regarde au démarrage.
      // En Flatpak, le portail ne dit pas l'état : le réglage enregistré fait foi.
      loadPlatformFeatures()
        .then(({ flatpak }) => (flatpak ? undefined : isAutostartEnabled()))
        .then(reel => {
          if (reel !== undefined) settingsWriter.update(state => ({ ...state, auto_start: reel ? 'true' : 'false' }));
        })
        .catch(e => console.warn('[settings] Impossible de vérifier autostart:', e));
    } catch (e) {
      console.error('[settingsStore] Failed to load settings', e);
    }
  },

  // Sauvegarder un paramètre :
  // 1. Met à jour le store Svelte (UI réactive immédiatement)
  // 2. Persiste en BDD via la commande Tauri
  // 3. Exécute l'effet de bord si défini (ex: activer/désactiver autostart)
  set: async (key: keyof AppSettings, value: string) => {
    settingsWriter.update(state => ({ ...state, [key]: value }));

    try {
      await invoke('set_setting', { key, value });
    } catch (e) {
      console.error(`[settingsStore] Failed to save ${key}`, e);
    }

    // Exécuter l'effet de bord (autostart, notifications, etc.)
    const effect = sideEffects[key];
    if (effect) {
      await effect(value);
    }
  },

  get: (key: keyof AppSettings): string => {
    return get(settingsWriter)[key];
  },

  // Toggle un booléen : inverse la valeur et appelle set()
  toggle: async (key: keyof AppSettings) => {
    const current = get(settingsWriter)[key];
    const next = current === 'true' ? 'false' : 'true';
    await settingsStore.set(key, next);
  },
};
