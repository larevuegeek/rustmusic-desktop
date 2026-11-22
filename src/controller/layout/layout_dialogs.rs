use crate::models::layout::*;
use crate::app::music_app::MusicApp;
use karbon_ui::intl;
use karbon_ui::intl::t;
use crate::models::app::Route;
use crate::models::layout::ToastKind;
use crate::commands::library::library_command;
use crate::commands::playback::player_command;
use crate::entity::audio::audio_file::AudioFile;
use crate::entity::library::library::LibraryCreate;
use crate::entity::library::library_cache::LibraryCacheCreate;
use crate::entity::queue::queue_track::QueueTrack;
use karbon_ui::task::{TaskId, TaskResult};
use karbon_ui::window::{FileDialog, FileDialogResult, WindowCommand, WindowId};

/// Tâche des dialogues de fichiers : un seul ouvert à la fois.
pub const DIALOG_TASK: TaskId = TaskId(900);

use crate::helper::files::audio_formats::AUDIO_EXTENSIONS;

/// Images proposées pour l'avatar d'un profil (comme la 0.4).
pub(crate) const AVATAR_IMAGES: [&str; 6] = ["jpg", "jpeg", "png", "webp", "gif", "bmp"];


/// La piste de file d'un fichier lu (pochette intégrée en adresse `data:`).
pub(crate) fn track(path_part: &str, audio: &AudioFile) -> QueueTrack {
    QueueTrack {
        queue_id: String::new(),
        profile_id: 0,
        position: 0,
        path: path_part.to_string(),
        title: audio.tags.title.clone().unwrap_or_else(|| t("common.unknown_title")),
        artist: audio.tags.artist.clone(),
        duration: Some(f64::from(audio.duration)),
        cover: audio.tags.attached_images.first().map(|i| i.image_src.clone()).filter(|s| !s.is_empty()),
    }
}

pub(crate) fn cache(audio: &AudioFile, thumbnail_bytes: Option<String>) -> LibraryCacheCreate {
    let tags = &audio.tags;
    LibraryCacheCreate {
        path: audio.path.clone(),
        title: tags.title.clone(),
        artist: tags.artist.clone(),
        album: tags.album.clone(),
        album_artist: tags.album_artist.clone(),
        year: tags.year.clone(),
        genre: tags.genre.clone(),
        track_number: tags.track_number.map(i32::from),
        disc_number: tags.disc_number.map(|n| n as i32),
        duration: Some(f64::from(audio.duration)),
        bitrate: Some(audio.bitrate as i32),
        bits_per_sample: Some(audio.bits_per_sample as i32),
        sample_rate: Some(audio.sample_rate as i32),
        channels: Some(audio.channels as i32),
        audio_format: Some(format!("{:?}", audio.audio_format)),
        mime_type: None,
        file_size: Some(audio.file_size as i64),
        extra_tags: serde_json::to_string(tags).ok(),
        thumbnail_path: thumbnail_bytes,
    }
}

impl MusicApp {
    /// Ouvre le dialogue ; son choix sera traité selon `choice`. Un dialogue déjà ouvert
    /// garde la main.
    pub(crate) fn ask(&mut self, choice: Choice) {
        if self.views.dialog.is_some() {
            return;
        }
        let dialog = match choice {
            Choice::PlayFile => FileDialog::open_file()
                .title(t("home.open_file"))
                .filter(t("actions.dialog_audio_files"), &AUDIO_EXTENSIONS),
            Choice::ImportFiles { .. } | Choice::PageImport { folder: false } => FileDialog::open_files()
                .title(t("actions.dialog_open_files"))
                .filter(t("actions.dialog_audio_files"), &AUDIO_EXTENSIONS),
            Choice::PlayFolder | Choice::ImportFolder { .. } | Choice::PageImport { folder: true } => {
                FileDialog::pick_folder().title(t("actions.dialog_select_folder"))
            }
            Choice::ProfileAvatar => FileDialog::open_file()
                .title(t("profil.avatar_choose"))
                .filter(t("actions.dialog_images"), &AVATAR_IMAGES),
        };
        self.views.dialog = Some(choice);
        self.window_commands.push(WindowCommand::FileDialog(WindowId::MAIN, DIALOG_TASK, dialog));
    }

    /// Résultat du dialogue (tâche `DIALOG_TASK`).
    pub(crate) fn dialog_closed(&mut self, result: TaskResult) -> bool {
        let Some(choice) = self.views.dialog.take() else { return false };
        let paths: Vec<String> = match result.into_value::<FileDialogResult>() {
            Ok(r) => r.paths.into_iter().map(|p| p.to_string_lossy().into_owned()).collect(),
            Err(error) => {
                log::error!("Dialogue de fichiers : {error}");
                return false;
            }
        };
        // Annulé : l'écran reste tel quel.
        let Some(first) = paths.first().cloned() else { return false };
        match choice {
            Choice::PlayFile => self.play_file(first),
            Choice::PlayFolder => self.play_folder(first),
            Choice::ImportFiles { library, redirect } => self.importer(library, redirect, paths, false),
            Choice::ImportFolder { library, redirect } => self.importer(library, redirect, paths, true),
            Choice::PageImport { folder } => self.import_new(paths, folder),
            Choice::ProfileAvatar => self.profile_avatar_import(first),
        }
        true
    }

    /// La file devient ce morceau, lu tout de suite, puis il entre dans le
    /// cache de bibliothèque avec sa vignette.
    pub(crate) fn play_file(&mut self, path_part: String) {
        let app = self.backend.app.clone();
        let source = path_part.clone();
        self.backend.call(
            async move { player_command::open_file(app, source).await },
            move |me, audio| {
                if audio.path.is_empty() {
                    return false;
                }
                let track = track(&path_part, &audio);
                me.play_list(vec![track], &path_part);
                me.cache_files(vec![audio]);
                true
            },
        );
    }

    /// Une file neuve dans l'ordre du dossier, puis le cache.
    pub(crate) fn play_folder(&mut self, folder: String) {
        self.backend.call(async move { player_command::open_files(folder).await }, move |me, files| {
            if files.is_empty() {
                return false;
            }
            let tracks_list: Vec<QueueTrack> = files.iter().map(|f| track(&f.path, f)).collect();
            let first = tracks_list[0].path.clone();
            me.play_list(tracks_list, &first);
            let n = files.len();
            let key = if n == 1 { "notify.files_to_playlist_one" } else { "notify.files_to_playlist_n" };
            me.toast(ToastKind::Success, t("notify.playlist_updated"), Some(t(key).replace("{n}", &intl::number(n as i64))));
            me.cache_files(files);
            true
        });
    }

    /// `create_library_cache` de chaque fichier lu, avec sa vignette s'il porte une image.
    pub(crate) fn cache_files(&mut self, files: Vec<AudioFile>) {
        let app = self.backend.app.clone();
        self.backend.fire(async move {
            for audio in files {
                let thumbnail_bytes = if audio.tags.attached_images.is_empty() {
                    None
                } else {
                    library_command::save_thumbnail_from_file(app.clone(), audio.path.clone()).ok().flatten()
                };
                let _ = library_command::create_library_cache(app.state(), cache(&audio, thumbnail_bytes)).await;
            }
        });
    }

    /// Import, bibliothèque rechargée, bilan.
    pub(crate) fn importer(&mut self, library: i64, redirect: bool, paths: Vec<String>, folder: bool) {
        if redirect {
            self.open_library(library);
        }
        let app = self.backend.app.clone();
        self.backend.call(
            async move {
                if folder {
                    let first = paths.into_iter().next().unwrap_or_default();
                    library_command::add_directory(app.clone(), app.state(), library, first).await
                } else {
                    library_command::add_files(app.clone(), app.state(), library, paths).await
                }
            },
            move |me, result| {
                match result {
                    Ok(tracks_list) if tracks_list.is_empty() => return false,
                    Ok(tracks_list) => {
                        let n = tracks_list.len();
                        let key = if n == 1 { "notify.tracks_added_one" } else { "notify.tracks_added_n" };
                        me.toast(ToastKind::Success, t("notify.files_added"), Some(t(key).replace("{n}", &intl::number(n as i64))));
                    }
                    Err(e) => {
                        let key = if !folder {
                            "notify.add_files_failed"
                        } else if e == "deja_en_cours" {
                            "notify.import_busy"
                        } else {
                            "notify.add_folder_failed"
                        };
                        me.toast(ToastKind::Error, t("notify.error"), Some(t(key)));
                        return true;
                    }
                }
                if me.data.library_id == Some(library) {
                    me.load_library_data();
                    me.load_page();
                }
                me.load_profile_data();
                true
            },
        );
    }

    /// `ImportWelcome.importer` : la bibliothèque choisie, sinon une nouvelle au nom saisi
    /// (créée après le choix : annuler ne crée rien), puis l'import et la bibliothèque.
    pub(crate) fn import_new(&mut self, paths: Vec<String>, folder: bool) {
        if let Some(id) = self.data.library_id.filter(|_| self.import_library_name().is_some()) {
            self.importer(id, true, paths, folder);
            return;
        }
        let Some(profile) = self.data.profile_id() else {
            self.toast(ToastKind::Error, t("notify.error"), Some(t("import_page.error")));
            return;
        };
        let typed = self.views.import_name.trim().to_string();
        let name = if typed.is_empty() { t("import_page.name_default") } else { typed };
        let app = self.backend.app.clone();
        self.backend.call(
            async move { library_command::create_library(app.state(), LibraryCreate { profile_id: profile, name, description: None }).await },
            move |me, result| match result {
                Ok(library) => {
                    me.data.library_id = Some(library.id);
                    me.load_profile_data();
                    me.importer(library.id, true, paths, folder);
                    true
                }
                Err(e) => {
                    let key = if e.contains("UNIQUE") || e.contains("duplicate") { "import_page.name_taken" } else { "import_page.error" };
                    me.toast(ToastKind::Error, t("notify.error"), Some(t(key)));
                    true
                }
            },
        );
    }

    /// `goto('/library/{id}')` : son premier onglet.
    pub(crate) fn open_library(&mut self, library: i64) {
        if self.data.library_id != Some(library) {
            self.data.library_id = Some(library);
            self.load_library_data();
        }
        if let Some(section) = self.data.library_tabs().first().copied() {
            self.history.push(Route::Library(section));
        }
        self.load_page();
    }
}
