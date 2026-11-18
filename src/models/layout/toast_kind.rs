#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToastKind {
    Info,
    Error,
    Warning,
    Success,
}
