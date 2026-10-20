// SPDX-License-Identifier: MIT

//! System clipboard helpers. On desktop we use `arboard` because WKWebView
//! often never fires `copy`/`paste` DOM events for Cmd+C / Cmd+V.

/// Image taken from the OS clipboard, ready to be saved as an attachment.
#[derive(Clone, Debug, PartialEq)]
pub struct PastedImage {
    /// Original file name for a copied file (Finder); `None` for raw pixels
    /// (screenshots, "Copy image"), where the host picks `Pasted image <stamp>.png`.
    pub filename: Option<String>,
    pub bytes: Vec<u8>,
}

#[cfg_attr(not(feature = "desktop"), allow(dead_code))]
pub enum ClipboardContent {
    Image(PastedImage),
    Text(String),
}

#[cfg(feature = "desktop")]
const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "svg", "avif", "bmp"];

/// Write plain text to the OS clipboard.
pub fn write_text(text: &str) -> Result<(), String> {
    #[cfg(feature = "desktop")]
    {
        let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
        clipboard
            .set_text(text.to_string())
            .map_err(|e| e.to_string())
    }
    #[cfg(not(feature = "desktop"))]
    {
        let _ = text;
        Err("Clipboard unavailable".into())
    }
}

/// Read what a paste should insert: copied image files first, then text,
/// then raw image pixels (Obsidian order).
pub fn read_paste() -> Option<ClipboardContent> {
    #[cfg(feature = "desktop")]
    {
        let mut clipboard = arboard::Clipboard::new().ok()?;
        if let Ok(files) = clipboard.get().file_list() {
            for path in files {
                let is_image = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| IMAGE_EXTS.contains(&e.to_ascii_lowercase().as_str()));
                if !is_image {
                    continue;
                }
                if let Ok(bytes) = std::fs::read(&path) {
                    let filename = path.file_name().and_then(|n| n.to_str()).map(String::from);
                    return Some(ClipboardContent::Image(PastedImage { filename, bytes }));
                }
            }
        }
        if let Ok(text) = clipboard.get_text() {
            if !text.is_empty() {
                return Some(ClipboardContent::Text(text));
            }
        }
        if let Ok(img) = clipboard.get_image() {
            if let Some(bytes) = encode_png(img.width as u32, img.height as u32, img.bytes.into_owned()) {
                return Some(ClipboardContent::Image(PastedImage {
                    filename: None,
                    bytes,
                }));
            }
        }
        None
    }
    #[cfg(not(feature = "desktop"))]
    {
        None
    }
}

#[cfg(feature = "desktop")]
fn encode_png(width: u32, height: u32, rgba: Vec<u8>) -> Option<Vec<u8>> {
    let buf = image::RgbaImage::from_raw(width, height, rgba)?;
    let mut out = std::io::Cursor::new(Vec::new());
    buf.write_to(&mut out, image::ImageFormat::Png).ok()?;
    Some(out.into_inner())
}
