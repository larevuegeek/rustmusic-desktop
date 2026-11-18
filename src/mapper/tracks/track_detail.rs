//! Le modèle de vue de la fiche d'un morceau.

use crate::models::tracks::*;
use chrono::{DateTime, Datelike, Utc};
use crate::app::music_app::MusicApp;
use karbon_ui::intl;
use crate::helper::date::elapsed_seconds;
use karbon_ui::intl::t;
use crate::models::app::Route;
use crate::mapper::albums::album_detail::best_quality;
use crate::models::albums::AlbumTrack;
use crate::helper::audio::dsd_label;
use crate::mapper::tracks::tier;
use crate::models::tracks::Tier;
use crate::query::track::track_list_row::TrackListRow;

/// `toLocaleDateString({ day, month: "long", year })`.
pub(crate) fn long_date(d: &DateTime<Utc>) -> String {
    let d = d.with_timezone(&chrono::Local);
    crate::helper::date::long_date(d.day(), d.month(), d.year())
}

/// « 44,1 kHz ».
pub(crate) fn khz(r: i32) -> String {
    let v = r as f64 / 1000.;
    if r % 1000 == 0 {
        format!("{} kHz", r / 1000)
    } else {
        format!("{:.1} kHz", v).replace('.', ",")
    }
}

/// « 2,82 MHz ».
pub(crate) fn dsd_rate(r: i32) -> String {
    format!("{:.2} MHz", r as f64 / 1_000_000.).replace('.', ",")
}

impl crate::models::tracks::TrackView {
    /// Chemin complet du fichier.
    pub fn path(&self) -> String {
        format!("{}{}", self.folder, self.file)
    }
}

impl MusicApp {
    /// Le titre de l'album qui porte ce morceau, dans sa version « liste » (tags complets).
    pub(crate) fn track_current(&self) -> Option<&TrackListRow> {
        let p = self.views.track.as_ref()?;
        let id = &p.track.as_ref()?.id;
        p.album.iter().find(|x| x.id == *id)
    }

    /// Les morceaux précédent et suivant de l'album.
    pub(crate) fn track_neighbours(&self) -> (Option<Route>, Option<Route>) {
        let Some(p) = self.views.track.as_ref() else { return (None, None) };
        let Some(id) = p.track.as_ref().map(|t| t.id.clone()) else { return (None, None) };
        let Some(rank) = p.album.iter().position(|x| x.id == id) else { return (None, None) };
        let route = |i: usize| p.album.get(i).map(|x| Route::Track(x.id.clone()));
        (rank.checked_sub(1).and_then(route), route(rank + 1))
    }

    pub fn track_view(&self) -> Option<TrackView> {
        let page = self.views.track.as_ref()?;
        let track = page.track.as_ref()?;
        let current_one = self.track_current();
        let album = track.album_id.as_ref().and_then(|id| self.data.albums.iter().find(|a| a.id == *id));
        let artist_key = current_one.and_then(|c| c.artist_id.clone()).or_else(|| page.credits.first().map(|c| c.artist_id.clone()));
        let artist_name = artist_key.as_ref().and_then(|id| self.data.artists.iter().find(|a| a.id == *id));
        let cover = track.thumbnail_path.clone().or_else(|| album.and_then(|a| a.cover_url.clone())).or_else(|| current_one.and_then(|c| c.thumbnail_path.clone()));
        let tint = cover.as_deref().and_then(|c| self.assets.as_ref()?.covers.get(c)).map(|img| img.dominant_hue()).unwrap_or((150., 0.));

        // Tags complets : compositeur, totaux, label.
        let (mut composer, mut total_tracks, mut total_discs, mut label) = (None, None, None, None);
        if let Some(fields) = current_one.and_then(|c| c.tags.as_deref()).and_then(|j| serde_json::from_str::<serde_json::Value>(j).ok()) {
            let custom = |k: &str| {
                fields["custom_tags"].as_array().and_then(|l| {
                    l.iter().find_map(|p| (p[0].as_str()?.to_uppercase() == k).then(|| p[1].as_str().map(String::from)).flatten())
                })
            };
            let integer = |v: &serde_json::Value| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok())).filter(|n| *n > 0);
            let text = |v: &serde_json::Value| v.as_str().filter(|s| !s.is_empty()).map(String::from);
            composer = text(&fields["composer"]).or_else(|| custom("COMPOSER"));
            total_tracks = integer(&fields["total_tracks"]).or_else(|| custom("TRACKTOTAL").and_then(|s| s.parse().ok())).or_else(|| custom("TOTALTRACKS").and_then(|s| s.parse().ok()));
            total_discs = integer(&fields["total_discs"]).or_else(|| custom("DISCTOTAL").and_then(|s| s.parse().ok())).or_else(|| custom("TOTALDISCS").and_then(|s| s.parse().ok()));
            label = text(&fields["publisher"]).or_else(|| custom("LABEL")).or_else(|| custom("ORGANIZATION"));
        }
        let rank = page.album.iter().position(|x| x.id == track.id);
        let number = track.track_number.map(|n| n as i64).or(rank.map(|r| r as i64 + 1));
        let total = total_tracks.or((!page.album.is_empty()).then_some(page.album.len() as i64));

        // Qualité.
        let format_ = track.audio_format.clone().unwrap_or_else(|| track.extension.clone()).to_uppercase();
        let dsd = track.bits_per_sample == Some(1) || format_ == "DSF" || format_ == "DFF";
        let tier = tier(track.audio_format.as_deref(), track.bits_per_sample, track.sample_rate);
        let reso = match (track.bits_per_sample, track.sample_rate) {
            (Some(b), Some(r)) if b > 0 && r > 0 => format!("{b} bit / {}", khz(r)),
            _ => String::new(),
        };
        let join = |l: Vec<String>| l.into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · ");
        let resume = Some(if dsd {
            (t("track_view.q_dsd"), format!("{} · {}", dsd_label(track.sample_rate.map_or(0, |r| r.max(0) as u32)), track.sample_rate.map(dsd_rate).unwrap_or_default()), tier, "HI-RES".to_string())
        } else if tier == Tier::Hires {
            (t("track_view.q_hires"), format!("{format_} {} · {reso}", t("track_view.lossless_word")), tier, "HI-RES".into())
        } else if tier == Tier::Lossless {
            let cd = track.bits_per_sample == Some(16) && track.sample_rate == Some(44100);
            (t(if cd { "track_view.q_cd" } else { "track_view.q_lossless" }), join(vec![format!("{format_} {}", t("track_view.lossless_word")), reso]), tier, "LOSSLESS".into())
        } else {
            let br = track.bitrate.filter(|b| *b > 0).map(|b| format!("{} kb/s", intl::number(b as i64))).unwrap_or_default();
            (t("track_view.q_lossy"), join(vec![format!("{format_} {}", t("track_view.lossy_word")), br]), tier, "LOSSY".into())
        });
        let channels = match track.channels {
            Some(1) => t("track_view.mono"),
            Some(2) => t("track_view.stereo"),
            Some(6) => "5.1".into(),
            Some(8) => "7.1".into(),
            Some(c) => format!("{c} ch"),
            None => "—".into(),
        };
        let line = |l: &str, v: String| InfoRow { label: t(l), value: v, route: None, extra: None, subtle: false, ruled: true };
        let mut audio = vec![
            line("track_view.duration", track.duration.map(intl::clock).unwrap_or_else(|| "—".into())),
            line("track_view.format", format_.clone()),
            line("track_view.bitrate", track.bitrate.filter(|b| *b > 0).map(|b| format!("{} kb/s", intl::number(b as i64))).unwrap_or_else(|| "—".into())),
            line("track_view.sample_rate", track.sample_rate.map(|r| if dsd { dsd_rate(r) } else { khz(r) }).unwrap_or_else(|| "—".into())),
            line("track_view.resolution", track.bits_per_sample.map(|b| format!("{b} bit")).unwrap_or_else(|| "—".into())),
            line("track_view.channels", channels),
            line("track_view.size", Some(track.file_size.unwrap_or(track.size)).filter(|s| *s > 0).map(|s| karbon_ui::intl::bytes(s as u64)).unwrap_or_else(|| "—".into())),
        ];
        let mut listens = line("track_view.plays", intl::number(track.play_count));
        listens.subtle = track.play_count == 0;
        audio.push(listens);
        // Grille de deux colonnes : pas de filet sous la dernière rangée.
        let count = audio.len();
        for (i, row) in audio.iter_mut().enumerate() {
            row.ruled = i + 2 < count;
        }
        let listen = if track.play_count == 0 {
            t("track_view.never_played")
        } else {
            let times = t(if track.play_count == 1 { "track_view.played_once" } else { "track_view.played_n" }).replace("{n}", &intl::number(track.play_count));
            match &track.last_played_at {
                Some(d) => format!("{times} · {}", elapsed_seconds(&d.to_rfc3339(), Utc::now()).map(intl::relative_time).unwrap_or_default()),
                None => times,
            }
        };

        // Tags.
        let mut tags = vec![
            InfoRow {
                label: t("track_view.album_artist"),
                value: track.album_artist.clone().unwrap_or_else(|| "—".into()),
                route: track.album_artist.as_ref().and(album.and_then(|a| a.artist_id.clone())).map(Route::Artist),
                extra: None,
                subtle: false,
                ruled: true,
            },
            InfoRow { label: t("track_view.genre"), value: track.genre.clone().unwrap_or_else(|| "—".into()), route: track.genre.clone().map(Route::Genre), extra: None, subtle: false, ruled: true },
            line("track_view.year", track.year.clone().filter(|y| !y.is_empty()).unwrap_or_else(|| "—".into())),
            InfoRow { label: t("track_view.track"), value: track.track_number.map(|n| n.to_string()).unwrap_or_else(|| "—".into()), route: None, extra: total_tracks.map(|n| format!(" / {n}")), subtle: false, ruled: true },
            InfoRow { label: t("track_view.disc"), value: track.disc_number.max(1).to_string(), route: None, extra: total_discs.map(|n| format!(" / {n}")), subtle: false, ruled: true },
        ];
        if let Some(c) = composer {
            tags.push(InfoRow { label: t("track_view.composer"), route: Some(Route::Composer(c.clone())), value: c, extra: None, subtle: false, ruled: true });
        }
        if let Some(l) = label {
            tags.push(line("album_view.label", l));
        }

        // Album et artiste.
        let mut cards = Vec::new();
        if let Some(id) = &track.album_id {
            let n = album.map(|a| a.total_tracks).unwrap_or(page.album.len() as i64);
            let sub = [
                album.and_then(|a| a.year).map(|y| y.to_string()),
                Some(format!("{} {}", intl::number(n), t(if n == 1 { "library_head.tracks_one" } else { "library_head.tracks_n" }))),
                album.filter(|a| a.total_duration > 0.).map(|a| intl::duration(a.total_duration)),
            ];
            cards.push(LinkCard {
                image: album.and_then(|a| a.cover_url.clone()).or_else(|| track.thumbnail_path.clone()),
                kicker: t("track_view.album"),
                name: track.album.clone().or_else(|| album.map(|a| a.title.clone())).unwrap_or_default(),
                sub: Some(sub.into_iter().flatten().collect::<Vec<_>>().join(" · ")),
                route: Route::Album(id.clone()),
                round: false,
            });
        }
        if let Some(id) = &artist_key {
            cards.push(LinkCard {
                image: artist_name.and_then(|a| a.thumbnail_path.clone()),
                kicker: t("artist_view.kicker"),
                name: artist_name.map(|a| a.name.clone()).or_else(|| track.artist.clone()).unwrap_or_default(),
                sub: artist_name.map(|a| {
                    format!(
                        "{} {} · {} {}",
                        intl::number(a.total_albums),
                        t(if a.total_albums == 1 { "library_head.albums_one" } else { "library_head.albums_n" }),
                        intl::number(a.total_tracks),
                        t(if a.total_tracks == 1 { "library_head.tracks_one" } else { "library_head.tracks_n" })
                    )
                }),
                route: Route::Artist(id.clone()),
                round: true,
            });
        }

        // Le reste de l'album, en deux colonnes lues de haut en bas.
        let playing = self.data.player.path.as_deref();
        let album_artist = album.map(|a| a.artist.clone()).or_else(|| track.album_artist.clone());
        let lines: Vec<AlbumTrack> = page
            .album
            .iter()
            .enumerate()
            .map(|(i, x)| AlbumTrack {
                i,
                number: x.track_number.map(|n| n.to_string()).unwrap_or_else(|| (i + 1).to_string()),
                numbered: x.track_number.is_some(),
                cover: x.thumbnail_path.clone(),
                title: x.title.clone(),
                invite: if x.id == track.id { Some(t("track_view.this_track")) } else { x.artist.clone().filter(|a| Some(a) != album_artist.as_ref()) },
                duration: x.duration.map(intl::clock).unwrap_or_default(),
                liked: self.data.liked.contains(&x.path),
                in_progress: playing == Some(x.path.as_str()),
                artwork: false,
                mark: x.id == track.id,
            })
            .collect();
        let half = lines.len().div_ceil(2);
        let same = (lines.len() > 1).then(|| {
            let mut left = lines;
            let right = left.split_off(half);
            (
                format!("{} · {} {}", track.album.clone().unwrap_or_default(), intl::number(page.album.len() as i64), t("library_head.tracks_n")),
                left,
                right,
                Route::Album(track.album_id.clone().unwrap_or_default()),
            )
        });
        let (prev, next) = self.track_neighbours();
        let (folder, file) = match track.path.rfind(['\\', '/']) {
            Some(i) => (track.path[..=i].to_string(), track.path[i + 1..].to_string()),
            None => (String::new(), track.path.clone()),
        };
        let year = track.year.as_deref().and_then(|y| y.get(..4)).and_then(|y| y.parse::<i64>().ok()).filter(|y| *y > 0);

        Some(TrackView {
            tint,
            quality: current_one.and_then(|c| best_quality(std::slice::from_ref(c))),
            title: track.title.clone(),
            title_long: track.title.chars().count() > 40,
            credits: if page.credits.is_empty() {
                track.artist.clone().map(|a| (a, artist_key.clone().map(Route::Artist))).into_iter().collect()
            } else {
                page.credits.iter().map(|c| (c.name.clone(), c.library_artist_id.as_ref().map(|_| Route::Artist(c.artist_id.clone())))).collect()
            },
            album: track.album.clone().map(|a| (a, track.album_id.clone().map(Route::Album))),
            year: year.map(|y| (y.to_string(), Route::Decade(y))),
            duration: track.duration.map(intl::clock),
            number: number.map(|n| (n.to_string(), total.map(|tt| t("track_view.of_total").replace("{total}", &tt.to_string())))),
            listen,
            added: Some(long_date(&track.created_at)),
            liked: self.data.liked.contains(&track.path),
            rating: track.rating.unwrap_or(0.),
            resume,
            audio,
            tags,
            remote: crate::service::external_server::track_service::is_remote(&track.path),
            cards,
            folder,
            file,
            scanned: track.last_scanned_at.as_ref().map(long_date),
            same,
            prev,
            next,
            cover,
        })
    }

    /// Les pochettes de la fiche : la grande, puis les vignettes des cartes.
    pub(crate) fn track_covers(&self) -> Vec<String> {
        let Some(v) = self.track_view() else { return Vec::new() };
        let mut paths: Vec<String> = v.cover.into_iter().collect();
        paths.extend(v.cards.iter().filter_map(|c| c.image.as_deref().map(crate::app::resources::assets::small_key)));
        paths
    }
}
