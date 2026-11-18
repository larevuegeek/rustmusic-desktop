#[derive(Debug, Clone, PartialEq)]
pub enum DownloadsMsg {
    /// Confirmation : `a` tout, `p` les écoutés, ou une émission.
    Confirm(Option<char>, Option<i64>),
    Clear(Option<i64>, bool),
    Delete(i64),
    Play(i64),
}
