/// Un scan ou un import en cours (`rescan-progress`, `import-progress`).
#[derive(Debug, Clone, Default)]
pub struct ScanProgress {
    pub current: i64,
    pub total: i64,
    pub label: String,
}
