use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::PathBuf;

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

        //On ouvre le fichier dans une box
        let source: Box<File> = Box::new(File::open(&file_path)?);

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

        let bits_per_sample: u32 = codec_params.bits_per_sample.unwrap_or(0);
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

        // Taille du fichier
        let file_size: u64 = std::fs::metadata(&file_path)?.len();

        let path_str: &str = file_path.to_str().ok_or("Chemin non-UTF8")?;

        let bitrate: u32 = match Self::read_audio_header_bitrate(path_str)? {
            Some(br) => br,
            None => {
                if duration > 0.0 {
                    (((file_size as f64 * 8.0) / duration as f64) / 1000.0).round() as u32
                } else {
                    0
                }
            }
        };

        let audio_format: AudioFormat = Self::map_codec_to_format(codec_params.codec);

        //////////////////////// Gestion des TAGS ////////////////////////////
        let mut tags: AudioTags = AudioTags::new();

        // 0.6 : les métadonnées du probe et du format sont unifiées sur le
        // reader — une seule source à lire (tags + visuels).
        let mut cover_found: bool = false;

        if let Some(metadata_rev) = format.metadata().current() {
            tags = Self::tags_reader(tags, metadata_rev);

            // Récupérer la vignette (cover art) — media-level puis per-track
            let visuals = metadata_rev
                .media
                .visuals
                .iter()
                .chain(metadata_rev.per_track.iter().flat_map(|pt| pt.metadata.visuals.iter()));
            for visual in visuals {
                cover_found = true;

                let attached_image: AttachedImage = Self::visual_tag_reader(visual);
                tags.attached_images.push(attached_image);
            }
        }

        if !cover_found {
            // On check le cover.jpg ou folder.jpg présent dans le dossier du fichier
            if let Some(folder) = file_path.parent() {
                const COVER_NAMES: [&'static str; 5] =
                    ["cover", "folder", "Cover", "Folder", "front"];
                const COVER_EXTS: [&'static str; 5] = ["jpg", "jpeg", "png", "webp", "aviff"];

                if let Some(cover_filepath) = COVER_NAMES
                    .iter()
                    .flat_map(|name| {
                        COVER_EXTS
                            .iter()
                            .map(move |ext| folder.join(format!("{name}.{ext}")))
                    })
                    .find(|path| path.exists())
                {
                    if let Ok(attached_image) = Self::visual_file_reader(&cover_filepath) {
                        tags.attached_images.push(attached_image);
                    }
                }
            }

            //println!("Aucune vignette trouvée");
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

    fn map_codec_to_format(codec: AudioCodecId) -> AudioFormat {
        if codec == codec_ids::CODEC_ID_MP3 {
            AudioFormat::MP3
        } else if codec == codec_ids::CODEC_ID_FLAC {
            AudioFormat::FLAC
        } else if codec == codec_ids::CODEC_ID_VORBIS || codec == codec_ids::CODEC_ID_OPUS {
            AudioFormat::OGG
        } else if codec == codec_ids::CODEC_ID_PCM_S16LE
            || codec == codec_ids::CODEC_ID_PCM_S24LE
            || codec == codec_ids::CODEC_ID_PCM_F32LE
        {
            AudioFormat::WAV
        } else {
            AudioFormat::Unknown
        }
    }

    fn tags_reader(mut tags: AudioTags, metadata_rev: &MetadataRevision) -> AudioTags {
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

    fn visual_tag_reader(visual: &Visual) -> AttachedImage {
        let base64_cover: String = general_purpose::STANDARD.encode(&visual.data);

        // 0.6 : media_type est devenu Option<String>
        let mime_type: String = visual
            .media_type
            .clone()
            .unwrap_or_else(|| "image/unknown".to_string());

        let image_src: String = format!("data:{};base64,{}", mime_type, base64_cover);

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

    fn visual_file_reader(file: &PathBuf) -> Result<AttachedImage, std::io::Error> {
        let data: Vec<u8> = std::fs::read(file)?;
        let base64_cover: String = general_purpose::STANDARD.encode(&data);

        // Déterminer le type MIME à partir de l’extension
        let mime_type: String = match file.extension().and_then(|ext| ext.to_str()) {
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("png") => "image/png",
            Some("webp") => "image/webp",
            Some("avif") | Some("aviff") => "image/avif",
            _ => "image/unknown",
        }
        .to_string();

        let image_src: String = format!("data:{};base64,{}", mime_type, base64_cover);

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
