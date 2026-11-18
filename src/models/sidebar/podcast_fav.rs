/// Un podcast favori, sa ligne sous « Podcasts ».
#[derive(Debug)]
pub struct PodcastFav {
    pub id: i64,
    pub title: String,
    pub subtitle: String,
    pub cover: Option<String>,
}
