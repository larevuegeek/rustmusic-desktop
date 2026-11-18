use crate::models::layout::*;

impl Toast {
    /// « success », « error », « warning » ou « info » (`data-variant`).
    pub fn variant(&self) -> &'static str {
        match self.kind {
            ToastKind::Info => "info",
            ToastKind::Error => "error",
            ToastKind::Warning => "warning",
            ToastKind::Success => "success",
        }
    }
}
