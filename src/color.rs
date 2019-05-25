use exoquant::ditherer::FloydSteinberg;
use exoquant::optimizer::{KMeans, Optimizer};
use exoquant::{convert_to_indexed, Color, Histogram, Quantizer, Remapper, SimpleColorSpace};
use image::{ImageBuffer, Rgb, RgbImage};
use itertools::Itertools;

mod ega;

pub use self::ega::EGA_6BIT;

#[macro_export]
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

pub trait ColorDepth {
    fn convert_color_depth(&self, image: &RgbImage) -> Vec<Color> {
        image
            .pixels()
            .map(|&p| {
                let Rgb { data: [r, g, b] } = p;
                self.convert_color(Color { r, g, b, a: 255 })
            })
            .collect()
    }

    fn convert_color(&self, pixel: Color) -> Color;
}

/// True 24-bit color, 8 bits per channel, virtually no limit in color depth.
#[derive(Debug, Copy, Clone)]
pub struct TrueColor24Bit;

impl ColorDepth for TrueColor24Bit {
    fn convert_color(&self, pixel: Color) -> Color {
        pixel
    }
}

/// VGA 18-bit color (64 levels per channel)
#[derive(Debug, Copy, Clone)]
pub struct Vga18Bit;

impl ColorDepth for Vga18Bit {
    fn convert_color(&self, pixel: Color) -> Color {
        let Color { r, g, b, a } = pixel;
        Color {
            r: (r.saturating_add(2)) & !0x03,
            g: (g.saturating_add(2)) & !0x03,
            b: (b.saturating_add(2)) & !0x03,
            a,
        }
    }
}

/// VGA 16-bit color, also called High color mode in legacy systems
/// (5 bits for red and blue channels, 6 bits for green)
#[derive(Debug, Copy, Clone)]
pub struct Vga16Bit;

impl ColorDepth for Vga16Bit {
    fn convert_color(&self, pixel: Color) -> Color {
        let Color { r, g, b, a } = pixel;
        Color {
            r: (r.saturating_add(4)) & !0x07,
            g: (g.saturating_add(2)) & !0x03,
            b: (b.saturating_add(4)) & !0x07,
            a,
        }
    }
}

/// Color depth defined by a "hardware-level" palette
/// of RGB colors.
#[derive(Debug, Copy, Clone)]
pub struct NearestInPalette<T>(T);

impl<T> ColorDepth for NearestInPalette<T>
where
    T: AsRef<[[u8; 3]]>,
{
    fn convert_color(&self, pixel: Color) -> Color {
        let Color {
            r: sr,
            g: sg,
            b: sb,
            a: _,
        } = pixel;
        let (sr, sg, sb) = (i32::from(sr), i32::from(sg), i32::from(sb));
        let [r, g, b] = self
            .0
            .as_ref()
            .iter()
            .min_by_key(|[pr, pg, pb]| {
                let (pr, pg, pb) = (i32::from(*pr), i32::from(*pg), i32::from(*pb));
                let rd = sr - pr;
                let rg = sg - pg;
                let rb = sb - pb;
                rd * rd + rg * rg + rb * rb
            })
            .unwrap()
            .clone();
        Color { r, g, b, a: 255 }
    }
}

/// 16 color palette established by the full-color CGA standard,
/// often seen in EGA mode as the default palette.
pub static CGA_4BIT: NearestInPalette<[[u8; 3]; 16]> = NearestInPalette([
    [0, 0, 0],
    [0, 0, 0xAA],
    [0, 0xAA, 0],
    [0, 0xAA, 0xAA],
    [0xAA, 0, 0],
    [0xAA, 0, 0xAA],
    [0xAA, 0x55, 0],
    [0xAA, 0xAA, 0xAA],
    [0x55, 0x55, 0x55],
    [0x55, 0x55, 0xFF],
    [0x55, 0xFF, 0x55],
    [0x55, 0xFF, 0xFF],
    [0xFF, 0x55, 0x55],
    [0xFF, 0x55, 0xFF],
    [0xFF, 0xFF, 0x55],
    [0xFF, 0xFF, 0xFF],
]);

/// Reduce the color palette of the given image according to the provided
/// color depth and maximum number of simultaneous colors.
pub fn map_to_retro_color_palette<D>(
    image: RgbImage,
    depth: D,
    num_colors: Option<usize>,
) -> RgbImage
where
    D: ColorDepth,
{
    let pixels = image
        .pixels()
        .map(|&Rgb { data: [r, g, b] }| Color { r, g, b, a: 255 })
        .collect_vec();

    let new_pixels = if let Some(num_colors) = num_colors {
        let mut histogram = Histogram::new();
        histogram.extend(pixels.iter().cloned());
        let colorspace = SimpleColorSpace::default();
        let optimizer = KMeans;
        let mut quantizer = Quantizer::new(&histogram, &colorspace);
        while quantizer.num_colors() < num_colors {
            quantizer.step();
            // very optional optimization, !very slow!
            // you probably only want to do this every N steps, if at all.
            if quantizer.num_colors() % 256 == 0 {
                quantizer = quantizer.optimize(&optimizer, 16);
            }
        }

        let palette = quantizer.colors(&colorspace);
        // this optimization is more useful than the above and a lot less slow
        let mut palette = optimizer.optimize_palette(&colorspace, &palette, &histogram, 4);

        // reduce palette's color depth
        for c in &mut palette {
            *c = depth.convert_color(*c);
        }

        let ditherer = FloydSteinberg::new();
        let remapper = Remapper::new(&palette, &colorspace, &ditherer);
        let indexed_data = remapper.remap(&pixels, image.width() as usize);
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

/// Reduce the image's color depth by quantizing to the nearest color, without
/// dithering.
pub fn reduce_color_depth<D>(image: RgbImage, depth: D) -> RgbImage
where
    D: ColorDepth,
{
    let pixels = depth
        .convert_color_depth(&image)
        .into_iter()
        .flat_map(|Color { r, g, b, .. }| value_iter![r, g, b])
        .collect_vec();

    ImageBuffer::from_raw(image.width(), image.height(), pixels)
        .expect("there should be enough pixels")
}

pub fn map_to_retro_color_palette_old<D>(
    image: RgbImage,
    depth: D,
    num_colors: Option<usize>,
) -> RgbImage
where
    D: ColorDepth,
{
    let ditherer = FloydSteinberg::new();

    let pixels = depth.convert_color_depth(&image);

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
