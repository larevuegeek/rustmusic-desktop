pub mod audio_player {
    pub mod audio_player;
    pub mod audio_utils;
    pub mod pipeline_info;
    pub mod preload;
    pub mod replay_gain;
    pub mod startup_timer;
    // WASAPI exclusive mode — Windows only. Sur les autres OS le module est
    // entièrement skipped grâce au `#![cfg(target_os = "windows")]` interne.
    #[cfg(target_os = "windows")]
    pub mod audio_output_wasapi;
    // Backends de sortie audio abstraits derrière le trait `AudioOutput`.
    pub mod output;
}

pub mod audio_analyser {
    pub mod audio_analyser;
}

pub mod audio_manager {
    pub mod audio_manager;
}

pub mod settings_manager {
    pub mod settings_manager;
}

pub mod audio_lyrics {
    pub mod lrclib_client;
    pub mod sidecar;
}

pub mod audio_metadata {
    pub mod extractor {
        pub mod extractor;
        pub mod error;
        pub mod format_sniffer;
    }
    pub mod file_format {
        pub mod dff;
        pub mod dsf;
    }
    pub mod tag_format {
        pub mod id3v2;
    }
    // Miroir en écriture de `extractor` : voir `injector/injector.rs`.
    pub mod injector {
        pub mod atomic_write;
        pub mod dff_container;
        pub mod dsf_container;
        pub mod edit;
        pub mod flac_container;
        pub mod id3v2_writer;
        pub mod image_prep;
        pub mod injector;
        pub mod mp3_container;
    }
}

pub mod audio_decoder {
    pub mod error;
    pub mod dsd {
        pub mod dsd_container;
        pub mod dsd_converter;
        pub mod dsd_player;
        pub mod dff_decoder;
        pub mod dsf_decoder;
        pub mod dop_encoder;
    }
}

pub mod audio_resampler {
    pub mod resampler;
}

pub mod metadata_source {
    pub mod deezer;
    pub mod matching;
}

pub mod batch {
    pub mod runner;
}

pub mod tag_audit {
    pub mod audit;
}

pub mod tag_clean {
    pub mod rules;
}

pub mod tag_pattern {
    pub mod pattern;
    pub mod planner;
    pub mod sanitize;
}

pub mod audio_quality;
pub mod gpu_sentinel;
pub mod render_mode;
pub mod system_detect;
pub mod media_controls;

pub mod dlna_server {
    pub mod server;
    pub mod config;
    pub mod error;
    pub mod xml;
    pub mod soap;
    pub mod net;
    pub mod ssdp {
        pub mod advertiser;
        pub mod listener;
    }
    pub mod http {
        pub mod router;
        pub mod description;
        pub mod control;
        pub mod media;
    }
    pub mod content_directory {
        pub mod browse;
        pub mod didl;
        pub mod object_id;
    }
    pub mod library {
        pub mod provider;
        pub mod sqlite_provider;
    }
}
pub mod smart_playlist;
