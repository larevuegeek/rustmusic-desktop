use super::*;
use crate::core::jellyfin::client::Server as Jellyfin;
use crate::core::subsonic::client::{Auth, Server as Subsonic};

#[test]
fn stored_credentials_round_trip() {
    let j = Backend::Jellyfin(Jellyfin { base: "http://nas:8096".into(), token: "tok".into(), user_id: "u1".into(), api_key: false });
    let back = Backend::restore(Kind::Jellyfin, "http://nas:8096".into(), "david".into(), j.stored()).unwrap();
    assert!(matches!(back, Backend::Jellyfin(ref x) if x.token == "tok" && x.user_id == "u1" && !x.api_key));
    let s = Backend::Subsonic(Subsonic { base: "http://nas:4533".into(), auth: Auth::ApiKey("k".into()) });
    assert!(matches!(Backend::restore(Kind::Subsonic, "x".into(), "d".into(), s.stored()).unwrap(), Backend::Subsonic(ref x) if x.auth == Auth::ApiKey("k".into())));
    assert!(Backend::restore(Kind::Jellyfin, "x".into(), "d".into(), Stored::default()).is_err());
    assert_eq!(Kind::parse("JellyFin"), Kind::Jellyfin);
    assert_eq!(Kind::parse("navidrome"), Kind::Subsonic);
    assert_eq!((source_of("remote://12/al/s"), source_of(r"C:\Musique\a.flac"), source_of("remote://x/a")), (12, 0, 0));
    assert!(matches!(Backend::restore(Kind::Dlna, "http://nas:8200/rootDesc.xml".into(), String::new(), Stored::default()).unwrap(), Backend::Dlna(_)));
}

#[tokio::test]
async fn subsonic_stream_urls() {
    let s = Backend::Subsonic(Subsonic { base: "http://nas:4533".into(), auth: Auth::ApiKey("k".into()) });
    let raw = s.stream_url("s1", None).await.unwrap().to_string();
    assert!(raw.contains("/rest/stream?") && raw.contains("format=raw") && raw.contains("id=s1"));
    let mp3 = s.stream_url("s1", Some(("mp3", 192))).await.unwrap().to_string();
    assert!(mp3.contains("format=mp3") && mp3.contains("maxBitRate=192"));
}
