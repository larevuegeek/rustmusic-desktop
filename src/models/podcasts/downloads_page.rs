use crate::entity::podcast::podcast::DownloadView;

#[derive(Default)]
pub struct DownloadsPage {
    pub list: Option<Vec<DownloadView>>,
    pub confirm: Option<(Option<char>, Option<i64>)>,
}
