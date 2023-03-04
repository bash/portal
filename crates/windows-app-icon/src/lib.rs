use image::codecs::ico::{IcoEncoder, IcoFrame};
use image::codecs::png::PngEncoder;
use image::imageops::{resize, FilterType};
use image::io::Reader as ImageReader;
use image::{ColorType, DynamicImage, ImageEncoder};
use std::borrow::Cow;
use std::fs::OpenOptions;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::{env, io};
use thiserror::Error;

/// Builds an ICO file from individual files.
/// For each size, the closest source image is scaled down to the appropriate size.
/// The source icons are assumed to be squares.
///
/// Use [`cargo::build_ico_file`] if you want to use this in the context of `build.rs`.
///
/// ## Example
/// In this example, the 16px, 24px, and 32px versions of this icon will
/// be resized versions of `windows-app-icon-32x32.png` while the 48px and 256px
/// versions will be resized from `windows-app-icon.png`.
///
/// ```no_run
/// use windows_app_icon::{build_ico, IconSizes};
///
/// let icon = build_ico_file(
///     vec!["windows-app-icon-32x32.png", "windows-app-icon.png"],
///     IconSizes::MINIMAL,
/// );
/// ```
pub fn build_ico_file<'a>(
    file_path: impl AsRef<Path>,
    icon_sources: impl IntoIterator<Item = impl AsRef<Path>>,
    sizes: impl Into<IconSizes<'a>>,
) -> Result<()> {
    let icons = decode_icons(icon_sources)?;

    let sizes = sizes.into();
    let sizes = sizes.0.iter().copied();
    let frames: Vec<_> = sizes
        .map(|size| create_ico_frame(&icons, size))
        .collect::<std::result::Result<_, _>>()?;

    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&file_path)?;
    IcoEncoder::new(file).encode_images(&frames)?;

    Ok(())
}

fn decode_icons(
    icon_sources: impl IntoIterator<Item = impl AsRef<Path>>,
) -> Result<Vec<DynamicImage>> {
    icon_sources
        .into_iter()
        .map(|path| ImageReader::open(path).unwrap().decode())
        .collect::<std::result::Result<_, _>>()
        .map_err(Into::into)
}

fn find_next_bigger_icon(icons: &[DynamicImage], size: u32) -> Result<&DynamicImage> {
    icons
        .iter()
        .filter(|icon| icon.width() >= size)
        .min_by_key(|icon| icon.width())
        .ok_or(Error::MissingIconSize(size))
}

fn create_ico_frame(icons: &[DynamicImage], size: u32) -> Result<IcoFrame<'static>> {
    let next_bigger_icon = find_next_bigger_icon(icons, size)?;
    let resized = resize(next_bigger_icon, size, size, FilterType::Lanczos3);
    encode_ico_frame(resized.as_raw(), size)
}

fn encode_ico_frame(buf: &[u8], size: u32) -> Result<IcoFrame<'static>> {
    let color_type = ColorType::Rgba8;
    let mut encoded = Vec::new();
    PngEncoder::new(Cursor::new(&mut encoded)).write_image(buf, size, size, color_type)?;
    Ok(IcoFrame::with_encoded(encoded, size, size, color_type)?)
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    Image(#[from] image::ImageError),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("No icon in the sources is >= {0}px")]
    MissingIconSize(u32),
}

pub type Result<T> = std::result::Result<T, Error>;

pub mod cargo {
    use std::ffi::OsStr;

    use super::*;

    /// Builds a windows ICO file. This function is mostly the same as [`super::build_ico_file`]
    /// but this function is intended to be used in the context of `build.rs`.
    pub fn build_ico_file<'a>(
        file_name: impl AsRef<OsStr>,
        icon_sources: impl IntoIterator<Item = impl AsRef<Path>>,
        sizes: impl Into<IconSizes<'a>>,
    ) -> Result<PathBuf> {
        let out_dir = env::var("OUT_DIR").unwrap();
        let mut output_path = PathBuf::from(out_dir);
        output_path.push(file_name.as_ref());

        super::build_ico_file(
            &output_path,
            icon_sources
                .into_iter()
                .inspect(|path| println!("cargo:rerun-if-changed={}", path.as_ref().display())),
            sizes,
        )?;

        Ok(output_path)
    }
}

pub struct IconSizes<'a>(Cow<'a, [u32]>);

impl<'a> IconSizes<'a> {
    /// The [bare minimum] recommended icon sizes.
    ///
    /// [bare minimum]: https://learn.microsoft.com/en-us/windows/apps/design/style/iconography/app-icon-construction#icon-scaling
    pub const MINIMAL: Self = Self(Cow::Borrowed(&[16, 24, 32, 48, 256]));
}

impl<'a, I> From<I> for IconSizes<'a>
where
    I: IntoIterator<Item = &'a u32>,
{
    fn from(value: I) -> Self {
        IconSizes(value.into_iter().copied().collect::<Vec<_>>().into())
    }
}
