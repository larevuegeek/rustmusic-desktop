//! Presse-papiers : une image (la pochette copiée depuis sa vue agrandie) ou du texte (le
//! détail de la chaîne audio).

use std::path::Path;
use std::sync::Mutex;

/// Le presse-papiers reste ouvert : sous X11 et Wayland, l'image n'est servie aux autres
/// applications que tant que son propriétaire existe.
static CLIPBOARD: Mutex<Option<arboard::Clipboard>> = Mutex::new(None);

/// Le presse-papiers ouvert une fois pour toutes, prêté à `write`.
fn with_clipboard(write: impl FnOnce(&mut arboard::Clipboard) -> Result<(), arboard::Error>) -> Result<(), String> {
    let mut clipboard = CLIPBOARD.lock().map_err(|e| e.to_string())?;
    if clipboard.is_none() {
        *clipboard = Some(arboard::Clipboard::new().map_err(|e| e.to_string())?);
    }
    let clipboard = clipboard.as_mut().ok_or_else(|| "presse-papiers indisponible".to_string())?;
    write(clipboard).map_err(|e| e.to_string())
}

/// Copie l'image du fichier `path`, décodée en RGBA.
pub fn copy_image(path: &Path) -> Result<(), String> {
    let image = image::open(path).map_err(|e| e.to_string())?.into_rgba8();
    let (width, height) = image.dimensions();
    let data = arboard::ImageData {
        width: width as usize,
        height: height as usize,
        bytes: image.into_raw().into(),
    };
    with_clipboard(|c| c.set_image(data))
}

/// Copie du texte.
pub fn copy_text(text: String) -> Result<(), String> {
    with_clipboard(|c| c.set_text(text))
}
