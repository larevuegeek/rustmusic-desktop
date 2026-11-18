#[derive(Debug, Clone, PartialEq)]
pub enum DeviceMsg {
    Open(usize),
    Close,
    Select,
}
