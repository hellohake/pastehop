use std::{env, fs, path::PathBuf};

use image::{ColorType, ImageFormat};
use tempfile::NamedTempFile;

use crate::errors::PasteHopError;

#[cfg(test)]
pub(crate) static CLIPBOARD_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub struct ClipboardImage {
    pub file: NamedTempFile,
    pub size_bytes: u64,
}

pub enum ClipboardContent {
    Files(Vec<PathBuf>),
    Image(ClipboardImage),
}

pub fn read_clipboard_content() -> Result<ClipboardContent, PasteHopError> {
    if let Some(fake_paths) = env::var_os("PH_FAKE_CLIPBOARD_FILES") {
        let paths: Vec<PathBuf> = env::split_paths(&fake_paths).collect();
        if !paths.is_empty() {
            return Ok(ClipboardContent::Files(paths));
        }
    }

    if env::var_os("PH_FAKE_CLIPBOARD_TEXT").is_some() {
        return Err(PasteHopError::ClipboardNotSupported);
    }

    if let Some(fake_path) = env::var_os("PH_FAKE_CLIPBOARD_IMAGE") {
        return materialize_png_from_file(PathBuf::from(fake_path)).map(ClipboardContent::Image);
    }

    let mut clipboard =
        arboard::Clipboard::new().map_err(|source| PasteHopError::ClipboardUnavailable {
            message: source.to_string(),
        })?;

    if let Some(paths) = read_clipboard_files(&mut clipboard)? {
        return Ok(ClipboardContent::Files(paths));
    }

    read_image_from_clipboard(&mut clipboard).map(ClipboardContent::Image)
}

#[cfg(target_os = "macos")]
fn read_clipboard_files(
    clipboard: &mut arboard::Clipboard,
) -> Result<Option<Vec<PathBuf>>, PasteHopError> {
    match clipboard.get().file_list() {
        Ok(paths) if !paths.is_empty() => Ok(Some(paths)),
        Ok(_) | Err(arboard::Error::ContentNotAvailable) => Ok(None),
        Err(error) => Err(PasteHopError::ClipboardUnavailable {
            message: error.to_string(),
        }),
    }
}

#[cfg(not(target_os = "macos"))]
fn read_clipboard_files(
    _clipboard: &mut arboard::Clipboard,
) -> Result<Option<Vec<PathBuf>>, PasteHopError> {
    Ok(None)
}

fn read_image_from_clipboard(
    clipboard: &mut arboard::Clipboard,
) -> Result<ClipboardImage, PasteHopError> {
    let image = clipboard
        .get_image()
        .map_err(|_| PasteHopError::ClipboardNotSupported)?;
    let bytes = image.bytes.into_owned();

    let file = NamedTempFile::new().map_err(|source| PasteHopError::ClipboardIo { source })?;
    image::save_buffer_with_format(
        file.path(),
        &bytes,
        image.width as u32,
        image.height as u32,
        ColorType::Rgba8,
        ImageFormat::Png,
    )
    .map_err(|source| PasteHopError::ClipboardImageEncoding { source })?;

    let size_bytes = fs::metadata(file.path())
        .map_err(|source| PasteHopError::ClipboardIo { source })?
        .len();

    Ok(ClipboardImage { file, size_bytes })
}

pub fn write_clipboard_text(text: &str) -> Result<(), PasteHopError> {
    if let Some(fake_path) = env::var_os("PH_FAKE_CLIPBOARD_WRITE_PATH") {
        fs::write(fake_path, text).map_err(|source| PasteHopError::ClipboardIo { source })?;
        return Ok(());
    }

    let mut clipboard =
        arboard::Clipboard::new().map_err(|source| PasteHopError::ClipboardUnavailable {
            message: source.to_string(),
        })?;
    clipboard
        .set_text(text.to_owned())
        .map_err(|source| PasteHopError::ClipboardUnavailable {
            message: source.to_string(),
        })
}

fn materialize_png_from_file(path: PathBuf) -> Result<ClipboardImage, PasteHopError> {
    let file = NamedTempFile::new().map_err(|source| PasteHopError::ClipboardIo { source })?;
    let image =
        image::open(&path).map_err(|source| PasteHopError::ClipboardImageEncoding { source })?;
    image
        .save_with_format(file.path(), ImageFormat::Png)
        .map_err(|source| PasteHopError::ClipboardImageEncoding { source })?;

    let size_bytes = fs::metadata(file.path())
        .map_err(|source| PasteHopError::ClipboardIo { source })?
        .len();

    Ok(ClipboardImage { file, size_bytes })
}

#[cfg(test)]
mod tests {
    use std::{env, fs, sync::MutexGuard};

    use image::{ImageBuffer, ImageFormat, Rgba};
    use tempfile::TempDir;

    use crate::errors::PasteHopError;

    use super::{
        CLIPBOARD_ENV_LOCK, ClipboardContent, materialize_png_from_file, read_clipboard_content,
        write_clipboard_text,
    };

    fn lock_clipboard_env() -> MutexGuard<'static, ()> {
        CLIPBOARD_ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[test]
    fn file_clipboard_content_takes_precedence_and_preserves_order() {
        let _guard = lock_clipboard_env();
        let temp_dir = TempDir::new().expect("temp dir should exist");
        let first = temp_dir.path().join("first.mov");
        let second = temp_dir.path().join("second.mp4");
        let preview = temp_dir.path().join("preview.png");
        fs::write(&first, b"mov").expect("first video should exist");
        fs::write(&second, b"mp4").expect("second video should exist");
        let buffer: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_fn(1, 1, |_, _| Rgba([10, 20, 30, 255]));
        buffer.save(&preview).expect("preview should save");

        unsafe {
            env::set_var(
                "PH_FAKE_CLIPBOARD_FILES",
                env::join_paths([&first, &second]).expect("paths should join"),
            );
            env::set_var("PH_FAKE_CLIPBOARD_IMAGE", &preview);
        }
        let result = read_clipboard_content();
        unsafe {
            env::remove_var("PH_FAKE_CLIPBOARD_FILES");
            env::remove_var("PH_FAKE_CLIPBOARD_IMAGE");
        }

        match result.expect("clipboard files should resolve") {
            ClipboardContent::Files(paths) => assert_eq!(paths, vec![first, second]),
            ClipboardContent::Image(_) => panic!("file URLs must take precedence over images"),
        }
    }

    #[test]
    fn image_clipboard_content_still_materializes_png() {
        let _guard = lock_clipboard_env();
        let temp_dir = TempDir::new().expect("temp dir should exist");
        let source = temp_dir.path().join("screenshot.png");
        let buffer: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_fn(2, 2, |_, _| Rgba([10, 20, 30, 255]));
        buffer.save(&source).expect("screenshot should save");

        unsafe {
            env::set_var("PH_FAKE_CLIPBOARD_IMAGE", &source);
        }
        let result = read_clipboard_content();
        unsafe {
            env::remove_var("PH_FAKE_CLIPBOARD_IMAGE");
        }

        match result.expect("clipboard image should resolve") {
            ClipboardContent::Image(image) => {
                assert!(image.size_bytes > 0);
                assert_eq!(
                    image::guess_format(
                        &fs::read(image.file.path()).expect("clipboard output should exist")
                    )
                    .expect("format should be detected"),
                    ImageFormat::Png
                );
            }
            ClipboardContent::Files(_) => panic!("image pixels must remain an image"),
        }
    }

    #[test]
    fn text_clipboard_content_is_not_claimed() {
        let _guard = lock_clipboard_env();
        unsafe {
            env::set_var("PH_FAKE_CLIPBOARD_TEXT", "plain text");
        }
        let result = read_clipboard_content();
        unsafe {
            env::remove_var("PH_FAKE_CLIPBOARD_TEXT");
        }

        assert!(matches!(result, Err(PasteHopError::ClipboardNotSupported)));
    }

    #[test]
    fn materializes_png_from_source_image() {
        let temp_dir = TempDir::new().expect("temp dir should exist");
        let source = temp_dir.path().join("clipboard.png");
        let buffer: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_fn(2, 2, |_, _| Rgba([10, 20, 30, 255]));
        buffer.save(&source).expect("source image should save");

        let clipboard = materialize_png_from_file(source).expect("png should materialize");

        assert!(clipboard.size_bytes > 0);
        assert!(clipboard.file.path().exists());
        assert_eq!(
            image::guess_format(
                &std::fs::read(clipboard.file.path()).expect("clipboard output should exist")
            )
            .expect("format should be detected"),
            ImageFormat::Png
        );
    }

    #[test]
    fn writes_text_to_fake_clipboard_sink() {
        let _guard = lock_clipboard_env();
        let temp_dir = TempDir::new().expect("temp dir should exist");
        let sink = temp_dir.path().join("clipboard.txt");

        unsafe {
            env::set_var("PH_FAKE_CLIPBOARD_WRITE_PATH", &sink);
        }
        let result = write_clipboard_text("~/remote/path.png");
        unsafe {
            env::remove_var("PH_FAKE_CLIPBOARD_WRITE_PATH");
        }

        result.expect("clipboard write should succeed");
        assert_eq!(
            fs::read_to_string(&sink).expect("clipboard sink should be written"),
            "~/remote/path.png"
        );
    }
}
