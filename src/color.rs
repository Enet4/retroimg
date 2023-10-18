//! Color depth manipulation module
use exoquant::ditherer::FloydSteinberg;
use exoquant::optimizer::{KMeans, Optimizer};
use exoquant::{Color, Histogram, Quantizer, Remapper, SimpleColorSpace};
use image::{ImageBuffer, Rgb, RgbImage};
use itertools::Itertools;
use num_integer::Roots;
use std::str::FromStr;

pub mod cga;
pub mod ega;

/// Enumeration of supported color distance algorithms
/// for loss calculation.
///
/// The use of one algorithm or the other may affect slightly
/// which palette colors are chosen,
/// especially in modes such as CGA.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub enum LossAlgorithm {
    /// L2, Euclidean distance
    #[default]
    L2,
    /// L1, Manhattan distance
    L1,
}

impl std::fmt::Display for LossAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LossAlgorithm::L1 => f.write_str("L1"),
            LossAlgorithm::L2 => f.write_str("L2"),
        }
    }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub struct LossAlgorithmParseError;

impl std::fmt::Display for LossAlgorithmParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid distance/loss algorithm, should be \"L1\" or \"L2\"")
    }
}

impl std::error::Error for LossAlgorithmParseError {}

impl FromStr for LossAlgorithm {
    type Err = LossAlgorithmParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "L1" | "l1" => Ok(LossAlgorithm::L1),
            "L2" | "l2" => Ok(LossAlgorithm::L2),
            _ => Err(LossAlgorithmParseError),
        }
    }
}

impl LossAlgorithm {
    /// calculate the difference between 2 colors
    /// using the given loss algorithm
    #[inline]
    pub fn color_diff(self, c1: Color, c2: Color) -> u64 {
        match self {
            LossAlgorithm::L1 => color_diff_l1(c1, c2),
            LossAlgorithm::L2 => color_diff_l2(c1, c2),
        }
    }

    /// calculate the difference between 2 images
    /// using the default loss
    ///
    /// # Panic
    ///
    /// Panics if the two slices of colors do not have the same length.
    pub fn image_diff(self, a: &[Color], b: &[Color]) -> u64 {
        assert_eq!(a.len(), b.len());
        Iterator::zip(a.iter(), b.iter())
            .map(|(a, b)| self.color_diff(*a, *b))
            .sum()
    }
}

/// calculate the L1 difference between 2 colors
fn color_diff_l1(c1: Color, c2: Color) -> u64 {
    let Color {
        r: r1,
        g: g1,
        b: b1,
        ..
    } = c1;
    let Color {
        r: r2,
        g: g2,
        b: b2,
        ..
    } = c2;
    let (r1, r2) = (i64::from(r1), i64::from(r2));
    let (g1, g2) = (i64::from(g1), i64::from(g2));
    let (b1, b2) = (i64::from(b1), i64::from(b2));
    (r1 - r2).abs() as u64 + (g1 - g2).abs() as u64 + (b1 - b2).abs() as u64
}

/// calculate the L2 difference between 2 colors
fn color_diff_l2(c1: Color, c2: Color) -> u64 {
    let Color {
        r: r1,
        g: g1,
        b: b1,
        ..
    } = c1;
    let Color {
        r: r2,
        g: g2,
        b: b2,
        ..
    } = c2;
    let (r1, r2) = (i64::from(r1), i64::from(r2));
    let (g1, g2) = (i64::from(g1), i64::from(g2));
    let (b1, b2) = (i64::from(b1), i64::from(b2));
    let dr = (r1 - r2) as u64;
    let dg = (g1 - g2) as u64;
    let db = (b1 - b2) as u64;

    (dr.saturating_mul(dr)
        .saturating_add(dg.saturating_mul(dg))
        .saturating_add(db.saturating_mul(db)))
    .sqrt()
}

/// calculate the median RGB color of the given buffer
fn color_median(colors: &[Color]) -> Color {
    let mut colors_r = colors.iter().map(|c| c.r).collect_vec();
    let mut colors_g = colors.iter().map(|c| c.g).collect_vec();
    let mut colors_b = colors.iter().map(|c| c.b).collect_vec();
    colors_r.sort_unstable();
    colors_g.sort_unstable();
    colors_b.sort_unstable();
    let r = colors_r[colors_r.len() / 2];
    let g = colors_r[colors_g.len() / 2];
    let b = colors_r[colors_b.len() / 2];

    Color { r, g, b, a: 255 }
}

/// The options for transforming an image to have a different color depth.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub struct ColorOptions {
    /// The maximum number of colors to admit.
    /// `None` means no limit
    pub num_colors: Option<u32>,

    /// The distance algorithm to use for calculating the loss between colors.
    ///
    /// The default is L2.
    pub loss: LossAlgorithm,
}

/// Color depth image converter.
pub trait ColorDepth {
    /// Convert and retrieve the loss from converting an image.
    fn convert_image_with_loss(&self, image: &RgbImage, options: ColorOptions)
        -> (Vec<Color>, u64);

    /// Convert an RGB image to this color depth.
    fn convert_image(&self, image: &RgbImage, options: ColorOptions) -> Vec<Color> {
        self.convert_image_with_loss(image, options).0
    }

    /// Estimate the loss obtained from converting an image.
    /// For the best results, greater discrepancies should result in higher
    /// loss values.
    fn loss(&self, image: &RgbImage, options: ColorOptions) -> u64 {
        self.convert_image_with_loss(image, options).1
    }
}

impl<'a, T: ColorDepth> ColorDepth for &'a T {
    fn convert_image_with_loss(
        &self,
        image: &RgbImage,
        options: ColorOptions,
    ) -> (Vec<Color>, u64) {
        (**self).convert_image_with_loss(image, options)
    }

    /// Convert an RGB image to this color depth.
    fn convert_image(&self, image: &RgbImage, options: ColorOptions) -> Vec<Color> {
        (**self).convert_image(image, options)
    }

    /// Estimate the loss obtained from converting an image.
    /// For the best results, greater discrepancies should result in higher
    /// loss values.
    fn loss(&self, image: &RgbImage, options: ColorOptions) -> u64 {
        (**self).loss(image, options)
    }
}

pub trait ColorMapper {
    /// Convert a single color
    fn convert_color(&self, c: Color) -> Color;
}

impl<'a, T: ColorMapper> ColorMapper for &'a T {
    fn convert_color(&self, c: Color) -> Color {
        (**self).convert_color(c)
    }
}

impl ColorMapper for fn(Color) -> Color {
    fn convert_color(&self, c: Color) -> Color {
        self(c)
    }
}

/// A color depth implementation with color mapping.
#[derive(Debug, Default, Copy, Clone)]
pub struct MappingColorDepth<M>(M);

impl<M> ColorMapper for MappingColorDepth<M>
where
    M: ColorMapper,
{
    fn convert_color(&self, c: Color) -> Color {
        self.0.convert_color(c)
    }
}

impl<M> ColorDepth for MappingColorDepth<M>
where
    M: ColorMapper,
{
    fn convert_image_with_loss(
        &self,
        image: &RgbImage,
        options: ColorOptions,
    ) -> (Vec<Color>, u64) {
        let original = image
            .pixels()
            .map(|&p| {
                let Rgb([r, g, b]) = p;
                Color { r, g, b, a: 255 }
            })
            .collect_vec();
        let pixels = image
            .pixels()
            .map(|&p| {
                let Rgb([r, g, b]) = p;
                self.0.convert_color(Color { r, g, b, a: 255 })
            })
            .collect_vec();

        // optimize palette and dither
        let converted_pixels = if let Some(num_colors) = options.num_colors {
            let mut palette = build_palette(&pixels, num_colors);

            // reduce palette's color depth
            for c in &mut palette {
                *c = self.convert_color(*c);
            }

            let colorspace = SimpleColorSpace::default();
            let ditherer = FloydSteinberg::new();
            let remapper = Remapper::new(&palette, &colorspace, &ditherer);
            let indexed_data = remapper.remap(&pixels, image.width() as usize);
            indexed_data
                .into_iter()
                .map(|i| palette[i as usize])
                .collect_vec()
        } else {
            pixels
        };
        let loss = options.loss.image_diff(&original, &converted_pixels);
        (converted_pixels, loss)
    }
}

/// True 24-bit color, 8 bits per channel, virtually no limit in color depth.
#[derive(Debug, Default, Copy, Clone)]
pub struct TrueColor24BitMapper;

impl ColorMapper for TrueColor24BitMapper {
    fn convert_color(&self, pixel: Color) -> Color {
        pixel
    }
}

pub type TrueColor24Bit = MappingColorDepth<TrueColor24BitMapper>;

impl TrueColor24Bit {
    pub fn new() -> Self {
        MappingColorDepth::default()
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct Vga18BitMapper;

impl ColorMapper for Vga18BitMapper {
    fn convert_color(&self, pixel: Color) -> Color {
        let Color { r, g, b, a } = pixel;
        Color {
            r: (r & !0x03) | r >> 6,
            g: (g & !0x03) | g >> 6,
            b: (b & !0x03) | b >> 6,
            a,
        }
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct Vga16BitMapper;

impl ColorMapper for Vga16BitMapper {
    fn convert_color(&self, pixel: Color) -> Color {
        let Color { r, g, b, a } = pixel;
        Color {
            r: (r & !0x07) | r >> 5,
            g: (g & !0x03) | g >> 6,
            b: (b & !0x07) | b >> 5,
            a,
        }
    }
}

/// VGA 18-bit color (64 levels per channel)
pub type Vga18Bit = MappingColorDepth<Vga18BitMapper>;

impl Vga18Bit {
    pub fn new() -> Self {
        MappingColorDepth::default()
    }
}

/// VGA 16-bit color, also called High color mode in legacy systems
/// (5 bits for red and blue channels, 6 bits for green)
pub type Vga16Bit = MappingColorDepth<Vga16BitMapper>;

impl Vga16Bit {
    pub fn new() -> Self {
        MappingColorDepth::default()
    }
}

/// Color depth defined by a hardware-level palette of RGB colors.
#[derive(Debug, Copy, Clone)]
pub struct FixedPalette<T>(T);

impl<T> FixedPalette<T>
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
        let [r, g, b] = *self
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
            .unwrap();
        Color { r, g, b, a: 255 }
    }
}

impl<T> ColorDepth for FixedPalette<T>
where
    T: AsRef<[[u8; 3]]>,
{
    fn convert_image_with_loss(
        &self,
        image: &RgbImage,
        options: ColorOptions,
    ) -> (Vec<Color>, u64) {
        let original = image
            .pixels()
            .map(|&p| {
                let Rgb([r, g, b]) = p;
                Color { r, g, b, a: 255 }
            })
            .collect_vec();

        // optimize palette and dither
        let converted_pixels = if let Some(num_colors) = options.num_colors {
            let mut palette = build_palette(&original, num_colors);

            // reduce palette's color depth
            for c in &mut palette {
                *c = self.convert_color(*c);
            }

            let colorspace = SimpleColorSpace::default();
            let ditherer = FloydSteinberg::new();
            let remapper = Remapper::new(&palette, &colorspace, &ditherer);
            let indexed_data = remapper.remap(&original, image.width() as usize);
            indexed_data
                .into_iter()
                .map(|i| palette[i as usize])
                .collect_vec()
        } else {
            original.clone()
        };
        let loss = options.loss.image_diff(&original, &converted_pixels);
        (converted_pixels, loss)
    }
}

fn build_palette(pixels: &[Color], num_colors: u32) -> Vec<Color> {
    // optimize palette and dither
    let mut histogram = Histogram::new();
    histogram.extend(pixels.iter().cloned());
    let colorspace = SimpleColorSpace::default();
    let optimizer = KMeans;
    let mut quantizer = Quantizer::new(&histogram, &colorspace);
    while quantizer.num_colors() < num_colors as usize {
        quantizer.step();
        // very optional optimization, !very slow!
        // you probably only want to do this every N steps, if at all.
        if quantizer.num_colors() % 256 == 0 {
            quantizer = quantizer.optimize(&optimizer, 16);
        }
    }

    let palette = quantizer.colors(&colorspace);
    // this optimization is more useful than the above and a lot less slow
    optimizer.optimize_palette(&colorspace, &palette, &histogram, 8)
}

/// Color depth emulating a combination of one freely selectable
/// background color (`B`) with any of the other colors (`F`).
#[derive(Debug, Copy, Clone)]
pub struct BackForePalette<B, F>(B, F);

impl<B, F> BackForePalette<B, F>
where
    B: AsRef<[[u8; 3]]>,
    F: AsRef<[[u8; 3]]>,
{
    fn convert_color<T>(pixel: Color, palette: T) -> Color
    where
        T: AsRef<[[u8; 3]]>,
    {
        let Color {
            r: sr,
            g: sg,
            b: sb,
            a: _,
        } = pixel;
        let (sr, sg, sb) = (i32::from(sr), i32::from(sg), i32::from(sb));
        let [r, g, b] = *palette
            .as_ref()
            .iter()
            .min_by_key(|[pr, pg, pb]| {
                let (pr, pg, pb) = (i32::from(*pr), i32::from(*pg), i32::from(*pb));
                let rd = sr - pr;
                let rg = sg - pg;
                let rb = sb - pb;
                rd * rd + rg * rg + rb * rb
            })
            .unwrap();
        Color { r, g, b, a: 255 }
    }

    fn convert_color_back(&self, pixel: Color) -> Color {
        BackForePalette::<B, F>::convert_color(pixel, &self.0)
    }

    /// Identify the best background color
    fn background_color(&self, image: &RgbImage) -> Color {
        // we'll fetch the median color of the image for the time being
        let original = image
            .pixels()
            .map(|&p| {
                let Rgb([r, g, b]) = p;
                Color { r, g, b, a: 255 }
            })
            .collect_vec();
        color_median(&original)
    }
}

impl<B, F> ColorDepth for BackForePalette<B, F>
where
    B: AsRef<[[u8; 3]]>,
    F: AsRef<[[u8; 3]]>,
{
    fn convert_image_with_loss(
        &self,
        image: &RgbImage,
        options: ColorOptions,
    ) -> (Vec<Color>, u64) {
        // first try to identify the background color
        let bkg_color = self.background_color(image);
        let bkg_color = self.convert_color_back(bkg_color);

        // then build a palette with the extra color
        let mut fixed = self.1.as_ref().to_vec();
        fixed.push([bkg_color.r, bkg_color.g, bkg_color.b]);
        let fixed = FixedPalette(fixed);

        let original = image
            .pixels()
            .map(|&p| {
                let Rgb([r, g, b]) = p;
                Color { r, g, b, a: 255 }
            })
            .collect_vec();

        // optimize palette and dither
        let converted_pixels = if let Some(num_colors) = options.num_colors {
            let mut palette = build_palette(&original, num_colors);

            // reduce palette's color depth
            for c in &mut palette {
                *c = fixed.convert_color(*c);
            }

            let colorspace = SimpleColorSpace::default();
            let ditherer = FloydSteinberg::new();
            let remapper = Remapper::new(&palette, &colorspace, &ditherer);
            let indexed_data = remapper.remap(&original, image.width() as usize);
            indexed_data
                .into_iter()
                .map(|i| palette[i as usize])
                .collect_vec()
        } else {
            original.clone()
        };
        let loss = options.loss.image_diff(&original, &converted_pixels);

        (converted_pixels, loss)
    }
}

/// A collection of palettes, the one yielding the lowest loss is used.
#[derive(Debug, Copy, Clone)]
pub struct BestPalette<C>(C);

impl<C, P> ColorDepth for BestPalette<C>
where
    C: std::ops::Deref<Target = [P]>,
    P: ColorDepth,
{
    fn convert_image_with_loss(
        &self,
        image: &RgbImage,
        options: ColorOptions,
    ) -> (Vec<Color>, u64) {
        self.0
            .iter()
            .map(|cd| cd.convert_image_with_loss(image, options))
            .min_by_key(|(_pixels, loss)| *loss)
            .unwrap()
    }
}

pub fn colors_to_image<I>(width: u32, height: u32, pixels: I) -> RgbImage
where
    I: IntoIterator<Item = Color>,
{
    let pixels = pixels
        .into_iter()
        .flat_map(|Color { r, g, b, .. }| [r, g, b])
        .collect_vec();
    ImageBuffer::from_raw(width, height, pixels).expect("there should be enough pixels")
}

/// 64 color palette established by the full-color EGA standard.
pub static PALETTE_BW_1BIT: FixedPalette<&[[u8; 3]]> = FixedPalette(BW_1BIT);

/// 64 color palette established by the full-color EGA standard.
pub static BW_1BIT: &[[u8; 3]] = &[[0, 0, 0], [0xFF, 0xFF, 0xFF]];
