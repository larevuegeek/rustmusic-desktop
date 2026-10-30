use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::core::audio_metadata::extractor::extractor as audio_metadata_extractor;
use crate::entity::audio::audio_file::{AudioFile, AudioFormat};
use crate::entity::audio::audio_tags::{AttachedImage, AudioTags, ImageType};
use crate::helper::string::string::normalize_year;
use base64::engine::general_purpose;
use base64::Engine;
use symphonia::core::codecs::audio::{well_known as codec_ids, AudioCodecId};
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::{
    MetadataOptions, MetadataRevision, StandardTag, StandardVisualKey, Visual,
};
use symphonia::core::units::Timestamp;
use symphonia::default::get_probe;

pub struct AudioAnalyser;

impl AudioAnalyser {
    pub fn analyse_audio_file(
        file_path: &PathBuf,
    ) -> Result<AudioFile, Box<dyn std::error::Error>> {
        Self::analyse_inner(file_path, false, true)
    }

    /// Pour le scan : seule la première image sert (la pochette), sans base64.
    /// Un livret de 30 scans par morceau, huit morceaux à la fois, saturait la mémoire.
    pub fn analyse_for_scan(
        file_path: &PathBuf,
    ) -> Result<AudioFile, Box<dyn std::error::Error>> {
        Self::analyse_inner(file_path, true, true)
    }

    /// Comme `analyse_for_scan`, sans chercher la pochette du dossier : l'import la
    /// cherche une seule fois par dossier, au lieu de la relire pour chaque morceau.
    pub fn analyse_for_import(
        file_path: &PathBuf,
    ) -> Result<AudioFile, Box<dyn std::error::Error>> {
        Self::analyse_inner(file_path, true, false)
    }

    fn analyse_inner(
        file_path: &PathBuf,
        scan: bool,
        folder_cover: bool,
    ) -> Result<AudioFile, Box<dyn std::error::Error>> {
        // Court-circuit pour les formats gérés par notre extractor maison
        // (Symphonia ne sait pas lire DSD, DSF ni DFF).
        //
        // On enveloppe l'extracteur dans `catch_unwind` pour garantir qu'aucun
        // panic (slice out-of-bounds sur un fichier tronqué/corrompu, etc.) ne
        // remonte au scanner : à l'échelle d'un batch rayon, un panic dans
        // un worker thread propage et tue tout le batch. Une erreur normale
        // (`ExtractError`) est déjà gérée par-fichier en amont.
        if let Some(ext) = file_path.extension().and_then(|s| s.to_str()) {
            let ext_lower = ext.to_ascii_lowercase();
            if ext_lower == "dsf" || ext_lower == "dff" {
                let path = file_path.clone();
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    audio_metadata_extractor::extract(&path)
                }));
                return match result {
                    Ok(Ok(audio)) => Ok(audio),
                    Ok(Err(e)) => Err(Box::new(e) as Box<dyn std::error::Error>),
                    Err(_) => Err(format!(
                        "panic while parsing DSD file {:?} (corrupted or truncated)",
                        file_path
                    )
                    .into()),
                };
            }
        }

        //Indice permet de deviner plus facilement le format notamment grâce à l'extension du fichier
        let mut hint: Hint = Hint::new();
        if let Some(ext) = file_path.extension().and_then(|s| s.to_str()) {
            hint.with_extension(ext);
        }

        //On ouvre le fichier dans une box ; sa taille vient du même descripteur.
        let fichier: File = File::open(&file_path)?;
        let file_size: u64 = fichier.metadata()?.len();
        let source: Box<File> = Box::new(fichier);

        //Enveloppe le fichier dans un flux média lisible par Symphonia (buffering, seeking, etc.).
        let mss: MediaSourceStream = MediaSourceStream::new(source, Default::default());

        //Options pour l’analyse du conteneur (format) et des métadonnées. Valeurs par défaut
        let format_opts: FormatOptions = Default::default();
        let metadata_opts: MetadataOptions = Default::default();

        //Sonde qui permet à symphonia de récupérer le format du fichier audio.
        //0.6 : probe() retourne directement le FormatReader ; les métadonnées
        //lues pendant le probe sont disponibles sur le reader lui-même.
        let mut format: Box<dyn FormatReader> =
            get_probe().probe(&hint, mss, format_opts, metadata_opts)?;
        // Récupérer les infos de la première piste audio trouvé dans le fichier (qui possède un SampleRate)
        let (track_id, codec_params, track_duration, track_time_base, track_num_frames) = {
            let track = format
                .tracks()
                .iter()
                .find(|t| {
                    t.codec_params
                        .as_ref()
                        .and_then(|c| c.audio())
                        .map_or(false, |a| a.sample_rate.is_some())
                })
                .ok_or("Aucune piste audio trouvée")?;

            let audio_params = track
                .codec_params
                .as_ref()
                .and_then(|c| c.audio())
                .cloned()
                .ok_or("Paramètres codec audio manquants")?;

            (track.id, audio_params, track.duration, track.time_base, track.num_frames)
        };

        // ALAC : la profondeur n'est que dans le magic cookie.
        let bits_per_sample: u32 = crate::core::audio_player::pipeline_info::source_bit_depth(&codec_params).unwrap_or(0);
        let source_sample_rate: u32 = codec_params.sample_rate.ok_or("Pas de sample rate")?;
        let channels: usize = codec_params.channels.as_ref().ok_or("Pas d'info channels")?.count();

        let mut duration_sec: Option<f64> = None;

        // Calculer la durée si les infos sont disponibles
        // 1️⃣ Méthode 1 : via duration + time_base (timing sur Track en 0.6)
        if let (Some(dur), Some(tb)) = (track_duration, track_time_base) {
            duration_sec = tb
                .calc_time(Timestamp::from(dur.get() as i64))
                .map(|t| t.as_secs_f64());
        }

        // 2️⃣ Méthode 2 : via num_frames (souvent pour FLAC)
        if duration_sec.is_none() {
            if let Some(total_samples) = track_num_frames {
                duration_sec = Some(total_samples as f64 / source_sample_rate as f64);
            }
        }

        let duration: f32 = duration_sec.unwrap_or(0.0) as f32;

        let path_str: &str = file_path.to_str().ok_or("Chemin non-UTF8")?;

        let audio_format: AudioFormat = Self::map_codec_to_format(codec_params.codec, file_path);

        // Seuls MP3 et Ogg Vorbis portent un débit dans leur en-tête : pour les
        // autres, inutile de rouvrir le fichier (coûteux sur un disque réseau).
        let debit_entete: Option<u32> = match audio_format {
            AudioFormat::MP3 | AudioFormat::OGG => Self::read_audio_header_bitrate(path_str)?,
            _ => None,
        };
        let bitrate: u32 = match debit_entete {
            Some(br) => br,
            None => {
                if duration > 0.0 {
                    (((file_size as f64 * 8.0) / duration as f64) / 1000.0).round() as u32
                } else {
                    0
                }
            }
        };

        //////////////////////// Gestion des TAGS ////////////////////////////
        let mut tags: AudioTags = AudioTags::new();

        // 0.6 : les métadonnées du probe et du format sont unifiées sur le
        // reader — une seule source à lire (tags + visuels).
        let mut cover_found: bool = false;

        if let Some(metadata_rev) = format.metadata().current() {
            let itunes = matches!(Self::map_codec_to_format(codec_params.codec, file_path), AudioFormat::AAC | AudioFormat::ALAC);
            tags = Self::tags_reader(tags, metadata_rev, itunes);

            // Récupérer la vignette (cover art) — media-level puis per-track
            let visuals = metadata_rev
                .media
                .visuals
                .iter()
                .chain(metadata_rev.per_track.iter().flat_map(|pt| pt.metadata.visuals.iter()));
            for visual in visuals {
                cover_found = true;

                let attached_image: AttachedImage = Self::visual_tag_reader(visual, !scan);
                tags.attached_images.push(attached_image);
                if scan {
                    break;
                }
            }
        }

        if !cover_found && folder_cover {
            // On check le cover.jpg ou folder.jpg présent dans le dossier du fichier
            if let Some(cover_filepath) = file_path.parent().and_then(Self::find_folder_cover) {
                if let Ok(attached_image) = Self::visual_file_reader(&cover_filepath, !scan) {
                    tags.attached_images.push(attached_image);
                }
            }
        }

        let audiofile: AudioFile = AudioFile {
            path: file_path.to_string_lossy().to_string(),
            audio_format,
            file_size,
            duration,
            bits_per_sample,
            bitrate,
            sample_rate: source_sample_rate,
            channels,
            track_id: Some(track_id),
            tags,
        };

        return Ok(audiofile);
    }

    fn map_codec_to_format(codec: AudioCodecId, path: &Path) -> AudioFormat {
        use crate::core::audio_player::pipeline_info::is_lossless_integer;
        if codec == codec_ids::CODEC_ID_MP3 {
            AudioFormat::MP3
        } else if codec == codec_ids::CODEC_ID_FLAC {
            AudioFormat::FLAC
        } else if codec == codec_ids::CODEC_ID_ALAC {
            AudioFormat::ALAC
        } else if codec == codec_ids::CODEC_ID_AAC {
            AudioFormat::AAC
        } else if codec == codec_ids::CODEC_ID_VORBIS || codec == codec_ids::CODEC_ID_OPUS {
            AudioFormat::OGG
        } else if is_lossless_integer(codec)
            || [codec_ids::CODEC_ID_PCM_F32LE, codec_ids::CODEC_ID_PCM_F32BE, codec_ids::CODEC_ID_PCM_F64LE, codec_ids::CODEC_ID_PCM_F64BE].contains(&codec)
        {
            // Du PCM : le conteneur dit s'il s'agit d'AIFF ou de WAV.
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
            if matches!(ext.as_str(), "aif" | "aiff" | "aifc") { AudioFormat::AIFF } else { AudioFormat::WAV }
        } else {
            AudioFormat::Unknown
        }
    }

    fn tags_reader(mut tags: AudioTags, metadata_rev: &MetadataRevision, itunes: bool) -> AudioTags {
        // 0.6 : les tags sont dans un MetadataContainer (media-level +
        // per-track) et un Tag = RawTag + Option<StandardTag> typé.
        let all_tags = metadata_rev
            .media
            .tags
            .iter()
            .chain(metadata_rev.per_track.iter().flat_map(|pt| pt.metadata.tags.iter()));

        for tag in all_tags {
            if let Some(std_tag) = &tag.std {
                match std_tag {
                    StandardTag::TrackTitle(v) => tags.title = Some(v.to_string()),
                    StandardTag::Artist(v) => {
                        // Vorbis/FLAC peut avoir plusieurs tags ARTIST séparés
                        // On les accumule avec ";" pour que split_artists() les sépare
                        let val = v.to_string();
                        tags.artist = Some(match tags.artist.take() {
                            Some(existing) => format!("{}; {}", existing, val),
                            None => val,
                        });
                    }
                    StandardTag::Album(v) => tags.album = Some(v.to_string()),
                    StandardTag::Genre(v) => tags.genre = Some(v.to_string()),
                    // 0.5 "Date" → 0.6 décliné en ReleaseDate/RecordingDate/…
                    StandardTag::ReleaseDate(v) | StandardTag::RecordingDate(v) => {
                        if tags.year.is_none() {
                            tags.year = normalize_year(Some(v.to_string()));
                        }
                    }
                    StandardTag::ReleaseYear(y) | StandardTag::RecordingYear(y) => {
                        if tags.year.is_none() {
                            tags.year = normalize_year(Some(y.to_string()));
                        }
                    }
                    StandardTag::TrackNumber(num) => {
                        tags.track_number = u16::try_from(*num).ok();
                    }
                    StandardTag::Composer(v) => tags.composer = Some(v.to_string()),
                    // iTunes range le compositeur dans `©wrt`, que Symphonia nomme « Writer ».
                    StandardTag::Writer(v) if itunes && tags.composer.is_none() => tags.composer = Some(v.to_string()),
                    StandardTag::Encoder(v) | StandardTag::EncodedBy(v) => {
                        tags.encoded_by = Some(v.to_string())
                    }
                    StandardTag::Copyright(v) => tags.copyright = Some(v.to_string()),
                    StandardTag::AlbumArtist(v) => {
                        let val = v.to_string();
                        tags.album_artist = Some(match tags.album_artist.take() {
                            Some(existing) => format!("{}; {}", existing, val),
                            None => val,
                        });
                    }
                    StandardTag::Bpm(v) => tags.bpm = Some(v.to_string()),
                    StandardTag::DiscNumber(num) => {
                        tags.disc_number = u16::try_from(*num).ok();
                    }
                    // Sans eux, l'éditeur affichait les totaux vides.
                    StandardTag::TrackTotal(num) => tags.total_tracks = u16::try_from(*num).ok(),
                    StandardTag::DiscTotal(num) => tags.total_discs = u16::try_from(*num).ok(),
                    StandardTag::Comment(v) => tags.comment = Some(v.to_string()),
                    StandardTag::Rating(ppm) => {
                        // 0.6 normalise le rating en PPM (0..=1_000_000),
                        // quel que soit le format source (POPM, Vorbis…).
                        //
                        // Cette échelle est bien plus fine que ce qu'on affiche :
                        // on peut donc arrondir à la demi-étoile sans rien
                        // inventer, là où l'arrondi à l'étoile entière jetait la
                        // moitié de l'information portée par le fichier.
                        let crans = ((*ppm as f64 / 1_000_000.0) * 10.0).round();
                        tags.rating = Some(crans.clamp(0.0, 10.0) / 2.0);
                    }
                    other => {
                        // Clé connue mais non encore mappée : on garde la clé
                        // brute du fichier + sa valeur telle qu'écrite.
                        let _ = other;
                        tags.custom_tags
                            .push((tag.raw.key.clone(), tag.raw.value.to_string()));
                    }
                }
            }
        }

        tags
    }

    fn visual_tag_reader(visual: &Visual, with_base64: bool) -> AttachedImage {

        // 0.6 : media_type est devenu Option<String>
        let mime_type: String = visual
            .media_type
            .clone()
            .unwrap_or_else(|| "image/unknown".to_string());

        let image_src: String = if with_base64 {
            format!("data:{};base64,{}", mime_type, general_purpose::STANDARD.encode(&visual.data))
        } else {
            String::new()
        };

        let image_type: ImageType = match visual.usage {
            Some(StandardVisualKey::FrontCover) => ImageType::CoverFront,
            Some(StandardVisualKey::BackCover) => ImageType::CoverBack,
            Some(StandardVisualKey::Leaflet) => ImageType::LeafletPage,
            Some(StandardVisualKey::Media) => ImageType::MediaLabel,
            Some(StandardVisualKey::ArtistPerformer)
            | Some(StandardVisualKey::LeadArtistPerformerSoloist)
            | Some(StandardVisualKey::BandOrchestra) => ImageType::Artist,
            Some(StandardVisualKey::Conductor) => ImageType::Conductor,
            Some(StandardVisualKey::Composer) => ImageType::Composer,
            Some(StandardVisualKey::Lyricist) => ImageType::LyricistTextWriter,
            Some(StandardVisualKey::RecordingLocation) => ImageType::RecordingLocation,
            Some(StandardVisualKey::RecordingSession) => ImageType::DuringRecording,
            Some(StandardVisualKey::Performance) => ImageType::DuringPerformance,
            Some(StandardVisualKey::ScreenCapture) => ImageType::MovieVideoScreenCapture,
            Some(StandardVisualKey::Illustration) => ImageType::Illustration,
            Some(StandardVisualKey::PublisherStudioLogo)
            | Some(StandardVisualKey::BandArtistLogo) => ImageType::PublisherStudioLogo,
            _ => ImageType::Other,
        };

        let attached_image: AttachedImage = AttachedImage {
            image_type: Some(image_type),
            mime_type,
            description: visual
                .tags
                .iter()
                .find(|t| t.raw.key.eq_ignore_ascii_case("description"))
                .map(|t| t.raw.value.to_string()),
            image_data: visual.data.to_vec(),
            image_src,
        };

        attached_image
    }

    /// Pochette posée à côté des morceaux (cover.jpg, folder.png…), par ordre de
    /// préférence. Le dossier est lu une fois, au lieu de tester chaque nom : sur un
    /// disque réseau ou le portail Flatpak, chaque test est un aller-retour. La casse
    /// exacte d'abord, puis sans tenir compte de la casse (COVER.JPG, disque FAT).
    pub fn find_folder_cover(folder: &Path) -> Option<PathBuf> {
        const COVER_NAMES: [&str; 5] = ["cover", "folder", "Cover", "Folder", "front"];
        const COVER_EXTS: [&str; 5] = ["jpg", "jpeg", "png", "webp", "aviff"];

        let noms: Vec<String> = std::fs::read_dir(folder)
            .ok()?
            .filter_map(Result::ok)
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();
        let candidats = || {
            COVER_NAMES
                .iter()
                .flat_map(|name| COVER_EXTS.iter().map(move |ext| format!("{name}.{ext}")))
        };
        let trouve = candidats()
            .find(|c| noms.iter().any(|n| n == c))
            .or_else(|| candidats().find_map(|c| noms.iter().find(|n| n.eq_ignore_ascii_case(&c)).cloned()))?;
        Some(folder.join(trouve))
    }

    /// Lit une pochette de dossier (sans base64), pour l'import.
    pub fn read_cover_file(file: &PathBuf) -> Result<AttachedImage, std::io::Error> {
        Self::visual_file_reader(file, false)
    }

    fn visual_file_reader(file: &PathBuf, with_base64: bool) -> Result<AttachedImage, std::io::Error> {
        let data: Vec<u8> = std::fs::read(file)?;

        // Déterminer le type MIME à partir de l’extension
        let mime_type: String = match file.extension().and_then(|ext| ext.to_str()) {
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("png") => "image/png",
            Some("webp") => "image/webp",
            Some("avif") | Some("aviff") => "image/avif",
            _ => "image/unknown",
        }
        .to_string();

        let image_src: String = if with_base64 {
            format!("data:{};base64,{}", mime_type, general_purpose::STANDARD.encode(&data))
        } else {
            String::new()
        };

        let attached_image: AttachedImage = AttachedImage {
            image_type: Some(ImageType::CoverFront),
            mime_type: "image/".to_string(),
            description: None,
            image_data: data,
            image_src,
        };

        Ok(attached_image)
    }

    pub fn read_audio_header_bitrate(path: &str) -> io::Result<Option<u32>> {
        let mut file: File = File::open(path)?;

        // Lecture des 4 premiers octets pour identifier le format
        let mut signature = [0u8; 4];
        file.read_exact(&mut signature)?;
        file.seek(SeekFrom::Start(0))?;

        let is_ogg: bool = &signature == b"OggS";
        let is_mp3: bool = &signature[0..2] == b"ID" || signature[0] == 0xFF;

        // Si ce n’est ni OGG ni MP3, on sort directement
        if !is_ogg && !is_mp3 {
            return Ok(None);
        }

        // ====== OGG (Vorbis uniquement) ======
        if is_ogg {
            let mut header: Vec<u8> = vec![0u8; 128];
            file.read_exact(&mut header)?;

            // Vérifie la présence de "vorbis" dans le paquet d'identification
            if header.windows(6).any(|w| w == b"vorbis") {
                // Les bitrates sont stockés dans le header Vorbis : [min][nominal][max]
                if let Some(pos) = header.windows(6).position(|w| w == b"vorbis") {
                    if pos + 18 <= header.len() {
                        let nominal = u32::from_le_bytes([
                            header[pos + 10],
                            header[pos + 11],
                            header[pos + 12],
                            header[pos + 13],
                        ]);
                        if nominal > 0 {
                            return Ok(Some(nominal / 1000)); // convertit en kb/s
                        }
                    }
                }
            }

            return Ok(None);
        }

        // ====== MP3 ======
        let mut id3_header: [u8; 10] = [0u8; 10];
        file.read_exact(&mut id3_header)?;
        let mut start_offset = 0u64;

        // Saute un éventuel tag ID3v2
        if &id3_header[0..3] == b"ID3" {
            let size: u32 = ((id3_header[6] as u32 & 0x7F) << 21)
                | ((id3_header[7] as u32 & 0x7F) << 14)
                | ((id3_header[8] as u32 & 0x7F) << 7)
                | (id3_header[9] as u32 & 0x7F);
            start_offset = 10 + size as u64;
            file.seek(SeekFrom::Start(start_offset))?;
        } else {
            file.seek(SeekFrom::Start(0))?;
        }

        let mut buffer: [u8; 4] = [0u8; 4];
        while file.read_exact(&mut buffer).is_ok() {
            let header: u32 = u32::from_be_bytes(buffer);

            // Vérifie la frame sync (11 bits à 1)
            if (header >> 21) & 0x7FF != 0x7FF {
                start_offset += 1;
                file.seek(SeekFrom::Start(start_offset))?;
                continue;
            }

            // Extraction des champs MPEG
            let version_id: u32 = (header >> 19) & 0b11;
            let layer_index: u32 = (header >> 17) & 0b11;
            let bitrate_index: u32 = (header >> 12) & 0b1111;
            let sample_rate_index: u32 = (header >> 10) & 0b11;

            if version_id == 0b01
                || layer_index == 0b00
                || bitrate_index == 0b1111
                || sample_rate_index == 0b11
            {
                start_offset += 1;
                file.seek(SeekFrom::Start(start_offset))?;
                continue;
            }

            // Tables officielles MPEG (Layer III)
            const TABLE_MPEG1: [u32; 16] = [
                0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 0,
            ];
            const TABLE_MPEG2: [u32; 16] = [
                0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160, 0,
            ];

            let bitrate = match (version_id, layer_index) {
                (0b11, 0b01) => TABLE_MPEG1[bitrate_index as usize], // MPEG1 Layer III
                (0b10, 0b01) => TABLE_MPEG2[bitrate_index as usize], // MPEG2 Layer III
                (0b00, 0b01) => TABLE_MPEG2[bitrate_index as usize], // MPEG2.5 Layer III
                _ => 0,
            };

            if bitrate > 0 {
                return Ok(Some(bitrate));
            }

            start_offset += 1;
            file.seek(SeekFrom::Start(start_offset))?;
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::AudioAnalyser;

    fn dossier(nom: &str, fichiers: &[&str]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("rustmusic-pochette-{nom}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for f in fichiers {
            std::fs::write(dir.join(f), b"x").unwrap();
        }
        dir
    }

    #[test]
    fn folder_cover_follows_preference_order() {
        // « cover » passe avant « folder », et .jpg avant .png.
        let dir = dossier("ordre", &["folder.jpg", "cover.png", "cover.jpg", "01.flac"]);
        assert_eq!(AudioAnalyser::find_folder_cover(&dir), Some(dir.join("cover.jpg")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn folder_cover_ignores_case_as_a_fallback() {
        let dir = dossier("casse", &["COVER.JPG", "01.flac"]);
        assert_eq!(AudioAnalyser::find_folder_cover(&dir), Some(dir.join("COVER.JPG")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// ALAC et AIFF étaient « Unknown », donc affichés avec perte (issue #8).
    #[test]
    fn lossless_containers_get_their_format() {
        let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        for (file, format, bits) in [("alac16.m4a", "ALAC", 16), ("alac24.m4a", "ALAC", 24), ("aiff24.aiff", "AIFF", 24), ("wav24.wav", "WAV", 24)] {
            let audio = AudioAnalyser::analyse_for_import(&fixtures.join(file)).unwrap();
            assert_eq!((format!("{:?}", audio.audio_format).as_str(), audio.bits_per_sample), (format, bits), "{file}");
        }
    }

    #[test]
    fn folder_without_cover_gives_none() {
        let dir = dossier("vide", &["01.flac", "livret.pdf"]);
        assert_eq!(AudioAnalyser::find_folder_cover(&dir), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
