#[derive(Debug, Clone)]
pub struct DeviceDetails {
    pub index: usize,
    pub probing: bool,
    pub error: Option<String>,
}
