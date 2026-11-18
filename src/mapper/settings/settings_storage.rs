use crate::models::settings::*;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::app::Msg;
use crate::models::settings::Control;
use crate::models::settings::Ghost;
use crate::models::settings::OptionGroup;
use crate::models::settings::OptionRow;
use crate::models::settings::SettingsMsg;

impl MusicApp {
    pub(crate) fn settings_storage(&self) -> Vec<OptionGroup> {
        let st = &self.views.settings.storage;
        let task = |task: StorageTask| Msg::Settings(SettingsMsg::Storage(task));
        let button = |key: &str, title: &str, desc: &str, words: &str, g: Ghost| self.keep(OptionRow::new(key, t(title), t(desc), Control::Buttons(vec![g])), words);
        let fetch_label = |busy: bool| if busy { t("common.fetching") } else { t("common.fetch") };
        let mut groups = vec![
            self.option_group(
                "libraries",
                Some(t("settings.storage_libraries_group")),
                Some(t("settings.storage_libraries_group_hint")),
                Some("settings.storage_libraries_group"),
                vec![
                    button("rescan", "settings.rescan_library", "settings.rescan_library_desc", "scan", Ghost::new("material-symbols:sync-rounded", t("settings.rescan_btn"), task(StorageTask::Rescan)).busy(st.rescanning)),
                    button(
                        "portraits",
                        "settings.storage_artist_images",
                        "settings.storage_artist_images_desc",
                        "deezer portrait",
                        Ghost::new("material-symbols:download-rounded", fetch_label(st.portraits), task(StorageTask::Portraits)).busy(st.portraits),
                    ),
                    button(
                        "covers",
                        "settings.album_covers",
                        "settings.album_covers_desc",
                        "deezer pochette",
                        Ghost::new("material-symbols:download-rounded", fetch_label(st.covers), task(StorageTask::Covers)).busy(st.covers),
                    ),
                    button(
                        "clean",
                        "settings.clean_images",
                        "settings.clean_images_desc",
                        "cache images pochettes portraits espace",
                        Ghost::new("material-symbols:cleaning-services-outline-rounded", t("settings.clean_images_btn"), task(StorageTask::Clean)).busy(st.cleaning),
                    ),
                    button(
                        "repair",
                        "settings.repair_links",
                        "settings.repair_links_desc",
                        "feat artistes",
                        Ghost::new("material-symbols:link-rounded", t("settings.repair_links_btn"), task(StorageTask::Repair)).busy(st.repairing),
                    ),
                ],
            ),
            self.option_group(
                "artist_split",
                Some(t("artist_split.title")),
                Some(t("artist_split.group_hint")),
                Some("settings.artist_split_group"),
                vec![
                    button(
                        "artist_check",
                        "artist_split.check_now",
                        "artist_split.check_now_desc",
                        "artistes multiples virgule musicbrainz feat séparer",
                        Ghost::new("material-symbols:group-search-rounded", t("artist_split.check_btn"), task(StorageTask::ArtistCheck)),
                    ),
                    self.toggle_row("artist_check_auto", t("artist_split.auto"), t("artist_split.auto_desc"), "artistes musicbrainz scan automatique"),
                    button("artist_review", "artist_split.review", "artist_split.review_desc", "artistes annuler corrections", Ghost::new("lucide:undo-2", t("artist_split.review_btn"), Msg::ArtistCheck(crate::models::artists::ArtistCheckMsg::Report))),
                ],
            ),
        ];
        groups.push(self.settings_servers());
        groups.push(self.option_group(
            "backup",
            Some(t("settings.backup_group")),
            Some(t("settings.backup_group_hint")),
            Some("settings.backup_group"),
            vec![
                button("export", "settings.export_title", "settings.export_desc", "json sauvegarde", Ghost::new("material-symbols:download-rounded", t("settings.export_btn"), Msg::Noop)),
                button("import", "settings.import_title", "settings.import_desc", "json restaurer", Ghost::new("material-symbols:upload-rounded", t("settings.import_btn"), Msg::Noop)),
            ],
        ));
        let folder = crate::app::platform::paths::app_data_dir().to_string_lossy().into_owned();
        groups.push(self.option_group(
            "data",
            Some(t("settings.data_group")),
            Some(t("settings.data_group_hint")),
            Some("settings.data_group"),
            vec![
                self.keep(
                    OptionRow::new(
                        "open_data",
                        t("settings.open_data_folder"),
                        t("settings.open_data_folder_desc"),
                        Control::Buttons(vec![Ghost::new("material-symbols:folder-open-outline-rounded", t("settings.open_btn"), task(StorageTask::OpenData))]),
                    )
                    .value(folder),
                    "dossier base covers",
                ),
                button("reset", "settings.reset", "settings.reset_desc", "effacer supprimer", Ghost::new("material-symbols:delete-outline-rounded", t("settings.reset_btn"), Msg::Typed(crate::models::layout::TypedMsg::Reset)).danger()),
            ],
        ));
        groups
    }
}
