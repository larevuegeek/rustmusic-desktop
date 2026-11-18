use crate::models::settings::*;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::layout::ToastKind;
use crate::commands::library::{artist_split_command, library_command};

impl MusicApp {
    pub(crate) fn settings_storage_update(&mut self, task: StorageTask) -> bool {
        let app = self.backend.app.clone();
        let st = &mut self.views.settings.storage;
        match task {
            StorageTask::Rescan => {
                if st.rescanning {
                    return false;
                }
                st.rescanning = true;
                let ids: Vec<i64> = self.data.libraries.iter().map(|l| l.id).collect();
                self.backend.call(
                    async move {
                        for id in ids {
                            if let Err(e) = library_command::rescan_library(app.clone(), app.state(), id).await {
                                log::warn!("Rescan {id} : {e}");
                            }
                        }
                    },
                    |me, ()| {
                        me.views.settings.storage.rescanning = false;
                        me.load_library_data();
                        true
                    },
                );
            }
            StorageTask::Portraits => {
                st.portraits = true;
                self.backend.call(async move { library_command::fetch_all_artist_images(app.clone(), app.state(), Some(true)).await }, |me, r| {
                    me.views.settings.storage.portraits = false;
                    if let Err(e) = r {
                        me.toast(ToastKind::Error, t("settings.storage_artist_images"), Some(e));
                    }
                    true
                });
            }
            StorageTask::Covers => {
                st.covers = true;
                let ids: Vec<i64> = self.data.libraries.iter().map(|l| l.id).collect();
                self.backend.call(
                    async move {
                        for id in ids {
                            let _ = library_command::fetch_all_album_covers(app.clone(), app.state(), id).await;
                        }
                    },
                    |me, ()| {
                        me.views.settings.storage.covers = false;
                        true
                    },
                );
            }
            StorageTask::Clean => {
                st.cleaning = true;
                self.backend.call(async move { library_command::clean_image_cache(app.state()).await }, |me, r| {
                    me.views.settings.storage.cleaning = false;
                    match r {
                        Ok(b) => {
                            let message = if b.deleted + b.alleges == 0 {
                                t("settings.clean_images_none")
                            } else {
                                t("settings.clean_images_done")
                                    .replace("{n}", &b.deleted.to_string())
                                    .replace("{m}", &b.alleges.to_string())
                                    .replace("{size}", &karbon_ui::intl::bytes(b.freed as u64))
                            };
                            me.toast(ToastKind::Success, t("settings.clean_images"), Some(message));
                        }
                        Err(e) => me.toast(ToastKind::Error, t("settings.clean_images"), Some(e)),
                    }
                    true
                });
            }
            StorageTask::Repair => {
                st.repairing = true;
                self.backend.call(async move { library_command::repair_artist_links(app.state(), None).await }, |me, r| {
                    me.views.settings.storage.repairing = false;
                    match r {
                        Ok(b) => {
                            let message = if b.links_created == 0 && b.main_artist_fixed == 0 {
                                t("settings.repair_links_none")
                            } else {
                                t("settings.repair_links_done")
                                    .replace("{links}", &b.links_created.to_string())
                                    .replace("{multi}", &b.tracks_multi.to_string())
                                    .replace("{artists}", &b.artists_created.to_string())
                                    .replace("{fixed}", &b.main_artist_fixed.to_string())
                            };
                            me.toast(ToastKind::Success, t("settings.repair_links"), Some(message));
                            me.load_library_data();
                        }
                        Err(e) => me.toast(ToastKind::Error, t("settings.repair_links"), Some(e)),
                    }
                    true
                });
            }
            StorageTask::ArtistCheck => {
                self.backend.fire(async move {
                    let _ = artist_split_command::artist_check_start(app.clone(), app.state()).await;
                });
            }
            StorageTask::OpenData => {
                let dir = crate::app::platform::paths::app_data_dir();
                if let Err(e) = std::process::Command::new(if cfg!(target_os = "windows") { "explorer" } else if cfg!(target_os = "macos") { "open" } else { "xdg-open" })
                    .arg(&dir)
                    .spawn()
                {
                    log::warn!("Dossier des données : {e}");
                }
                return false;
            }
        }
        true
    }
}
