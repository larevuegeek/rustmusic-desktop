use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AudioTags {
    pub id3_version: Option<String>,

    // ID3v1, ID3v2, and other formats
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub year: Option<String>,
    pub comment: Option<String>,
    pub track_number: Option<u16>,
    pub genre: Option<String>,
    pub rating: Option<f64>, // 0,5 à 5,0 par demi (POPM → mappé)

    // Extensions for ID3v2 and other formats
    pub album_artist: Option<String>,
    pub composer: Option<String>,
    pub original_artist: Option<String>,
    pub part_of_set: Option<String>,
    pub publisher: Option<String>,
    pub encoded_by: Option<String>,
    pub encoding_settings: Option<String>,
    pub bpm: Option<String>,
    pub duration: Option<u32>, // Duration in seconds
    pub language: Option<String>,
    pub media_type: Option<String>,
    pub file_type: Option<String>,

    // Copyright and licensing
    pub copyright: Option<String>,
    pub internet_radio_station_name: Option<String>,
    pub internet_radio_station_owner: Option<String>,

    // Performance, recording, and musicians
    pub conductor: Option<String>,
    pub lyricist: Option<String>,
    pub remix_artist: Option<String>,
    pub arranged_by: Option<String>,
    pub interpreted_by: Option<String>, // or 'performer'

    // Additional information
    pub mood: Option<String>,
    pub isrc: Option<String>, // International Standard Recording Code
    pub disc_number: Option<u16>,
    pub total_discs: Option<u16>,
    pub compilation: Option<bool>, // True for compilation albums (TCMP=1)
    pub subtitle: Option<String>,
    pub key: Option<String>, // Musical key
    pub total_tracks: Option<u16>, // Second part of "track/total" (TRCK)

    // Lyrics and notation
    pub lyrics: Option<String>,
    pub unsynchronised_lyrics: Option<String>, // Unsynchronised lyrics

    // URLs and unique identifiers
    pub official_audio_source_url: Option<String>,
    pub official_audio_file_url: Option<String>,
    pub official_artist_url: Option<String>,
    pub payment_url: Option<String>,
    pub publisher_url: Option<String>,

    // User-defined tags and miscellaneous
    pub custom_tags: Vec<(String, String)>, // Key-value pairs for user-defined tags

    // Cover art and other images
    pub attached_images: Vec<AttachedImage>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AttachedImage {
    pub image_type: Option<ImageType>,
    pub mime_type: String, // MIME type of the image, e.g., "image/jpeg"
    pub description: Option<String>, // Optional description of the image
    // Jamais serialise : ~4 caracteres de JSON par octet, pour une donnee que
    // l'interface ne lit pas. Elle passe par `image_src` (base64) ou par la
    // vignette sur disque.
    #[serde(skip_serializing, default)]
    pub image_data: Vec<u8>,
    pub image_src: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ImageType {
    Other,
    Icon32x32PNG,
    IconOther,
    CoverFront,
    CoverBack,
    LeafletPage,
    MediaLabel,
    LeadArtist,
    Artist,
    Conductor,
    BandOrchestra,
    Composer,
    LyricistTextWriter,
    RecordingLocation,
    DuringRecording,
    DuringPerformance,
    MovieVideoScreenCapture,
    ABrightColouredFish,
    Illustration,
    BandArtistLogo,
    PublisherStudioLogo,
}

impl AudioTags {
    pub fn new() -> Self {
        AudioTags {
            id3_version: None,
            title: None,
            artist: None,
            album: None,
            year: None,
            comment: None,
            track_number: None,
            genre: None,
            rating: None,
            album_artist: None,
            composer: None,
            original_artist: None,
            part_of_set: None,
            publisher: None,
            encoded_by: None,
            encoding_settings: None,
            bpm: None,
            duration: None,
            language: None,
            media_type: None,
            file_type: None,
            copyright: None,
            internet_radio_station_name: None,
            internet_radio_station_owner: None,
            conductor: None,
            lyricist: None,
            remix_artist: None,
            arranged_by: None,
            interpreted_by: None,
            mood: None,
            isrc: None,
            disc_number: None,
            total_discs: None,
            compilation: None,
            subtitle: None,
            key: None,
            total_tracks: None,
            lyrics: None,
            unsynchronised_lyrics: None,
            official_audio_source_url: None,
            official_audio_file_url: None,
            official_artist_url: None,
            payment_url: None,
            publisher_url: None,
            custom_tags: Vec::new(),
            attached_images: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les octets de la pochette ne doivent pas franchir l'IPC.
    ///
    /// `serde` les rendait en tableau JSON — « 255,216,255,… » — soit environ
    /// quatre caractères par octet, à chaque ouverture de morceau, pour une
    /// donnée que l'interface ne lit jamais.
    #[test]
    fn les_octets_de_pochette_ne_sont_pas_serialises() {
        let mut tags = AudioTags::new();
        tags.attached_images.push(AttachedImage {
            image_type: None,
            mime_type: "image/jpeg".to_string(),
            description: None,
            image_data: vec![0xFF; 253 * 1024], // la moyenne mesurée sur la bibliothèque
            image_src: "data:image/jpeg;base64,/9j/4AAQ".to_string(),
        });

        let json = serde_json::to_string(&tags).expect("sérialisation");

        assert!(!json.contains("image_data"), "les octets sont encore envoyés");
        assert!(json.contains("image_src"), "la base64 doit rester, elle sert à l'affichage");
        assert!(
            json.len() < 4096,
            "charge utile inattendue : {} octets — les octets bruts sont probablement revenus",
            json.len()
        );
    }

    /// Le champ reste lisible quand il est absent : `default` le remplit.
    #[test]
    fn un_json_sans_octets_se_relit() {
        let json = r#"{"image_type":null,"mime_type":"image/png","description":null,"image_src":"x"}"#;
        let image: AttachedImage = serde_json::from_str(json).expect("désérialisation");
        assert!(image.image_data.is_empty());
        assert_eq!(image.image_src, "x");
    }
}
