use exoquant::ditherer::FloydSteinberg;
use exoquant::optimizer::KMeans;
use exoquant::{convert_to_indexed, Color};
use image::imageops::resize;
use image::{FilterType, GenericImage, ImageBuffer, Pixel, Rgb, RgbImage};
use itertools::Itertools;

macro_rules! value_iter {
    () => {
        std::iter::empty()
    };
    ($v: expr, $( $rest: expr ), +) => {
        std::iter::once($v).chain(
            value_iter!($($rest),*)
        )
    };
    ($v: expr) => {
        std::iter::once($v)
    };
}

pub fn reduce<I: 'static>(
    img: &I,
    nwidth: u32,
    nheight: u32,
) -> ImageBuffer<I::Pixel, Vec<<I::Pixel as Pixel>::Subpixel>>
where
    I: GenericImage,
{
    resize(img, nwidth, nheight, FilterType::CatmullRom)
}

pub fn crop(mut image: RgbImage, left: u32, top: u32, width: u32, height: u32) -> RgbImage {
    image::imageops::crop(&mut image, left, top, width, height);
    image
}

pub fn map_to_retro_color_palette(image: RgbImage, num_colors: Option<usize>) -> RgbImage {
    let ditherer = FloydSteinberg::new();

    let pixels = image
        .pixels()
        .map(|Rgb { data: [r, g, b] }| Color {
            r: (r.saturating_add(2)) & !0x03,
            g: (g.saturating_add(2)) & !0x03,
            b: (b.saturating_add(2)) & !0x03,
            a: 255,
        })
        .collect_vec();

    let new_pixels = if let Some(num_colors) = num_colors {
        let (palette, indexed_data) = convert_to_indexed(
            &pixels,
            image.width() as usize,
            num_colors,
            &KMeans,
            &ditherer,
        );
        indexed_data
            .into_iter()
            .map(|i| palette[i as usize])
            .flat_map(|Color { r, g, b, .. }| value_iter![r, g, b])
            .collect_vec()
    } else {
        pixels
            .into_iter()
            .flat_map(|Color { r, g, b, .. }| value_iter![r, g, b])
            .collect_vec()
    };

    ImageBuffer::from_raw(image.width(), image.height(), new_pixels)
        .expect("there should be enough pixels")
}

pub fn expand<I: 'static>(
    img: &I,
    nwidth: u32,
    nheight: u32,
) -> ImageBuffer<I::Pixel, Vec<<I::Pixel as Pixel>::Subpixel>>
where
    I: GenericImage,
{
    resize(img, nwidth, nheight, FilterType::Nearest)
}

