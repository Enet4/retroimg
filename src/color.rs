use exoquant::ditherer::FloydSteinberg;
use exoquant::optimizer::{KMeans, Optimizer};
use exoquant::{convert_to_indexed, Color, Histogram, Quantizer, Remapper, SimpleColorSpace};
use image::{ImageBuffer, Rgb, RgbImage};
use itertools::Itertools;

pub mod cga;
pub mod ega;

pub use self::cga::CGA_4BIT;
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

/// calculate the L1 difference between 2 images
fn image_diff_l1(a: &[Color], b: &[Color]) -> u64 {
    assert_eq!(a.len(), b.len());
    Iterator::zip(a.iter(), b.iter())
        .map(|(a, b)| color_diff_l1(*a, *b))
        .sum()
}

/// calculate the median RGB color of the given buffer
fn color_median(colors: &[Color]) -> Color {
    let mut colors_r = colors.into_iter().map(|c| c.r).collect_vec();
    let mut colors_g = colors.into_iter().map(|c| c.g).collect_vec();
    let mut colors_b = colors.into_iter().map(|c| c.b).collect_vec();
    colors_r.sort();
    colors_g.sort();
    colors_b.sort();
    let r = colors_r[colors_r.len() / 2];
    let g = colors_r[colors_g.len() / 2];
    let b = colors_r[colors_b.len() / 2];

    Color { r, g, b, a: 255 }
}

/// Color depth image converter.
pub trait ColorDepth {
    /// Convert and retrieve the loss from converting an image.
    fn convert_image_with_loss(&self, image: &RgbImage, num_colors: Option<u32>) -> (Vec<Color>, u64);

    /// Convert an RGB image to this color depth.
    fn convert_image(&self, image: &RgbImage, num_colors: Option<u32>) -> Vec<Color> {
        self.convert_image_with_loss(image, num_colors).0
    }

    /// Estimate the loss obtained from converting an image.
    /// For the best results, greater discrepancies should result in higher
    /// loss values.
    fn loss(&self, image: &RgbImage, num_colors: Option<u32>) -> u64 {
        self.convert_image_with_loss(image, num_colors).1
    }
}

pub trait ColorMapper {
    /// Convert a single color
    fn convert_color(&self, c: Color) -> Color; 

    /// Calculate the L1 loss of converting a single color
    fn l1_loss(&self, pixel: Color) -> u64 {
        let converted = self.convert_color(pixel);
        color_diff_l1(pixel, converted)
    }
}

impl ColorMapper for fn(Color) -> Color {
    fn convert_color(&self, c: Color) -> Color {
        self(c)
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct MappingColorDepth<M>(M);

impl<M> ColorMapper for MappingColorDepth<M>
where
    M: ColorMapper,
{
    fn convert_color(&self, c: Color) -> Color {
        self.0.convert_color(c)
    }

    fn l1_loss(&self, pixel: Color) -> u64 {
        self.0.l1_loss(pixel)
    }
}

impl<M> ColorDepth for MappingColorDepth<M>
where
    M: ColorMapper,
{
    fn convert_image_with_loss(&self, image: &RgbImage, num_colors: Option<u32>) -> (Vec<Color>, u64) {
        let original = image
            .pixels()
            .map(|&p| {
                let Rgb { data: [r, g, b] } = p;
                Color { r, g, b, a: 255 }
            })
            .collect_vec();
        let pixels = image
            .pixels()
            .map(|&p| {
                let Rgb { data: [r, g, b] } = p;
                self.0.convert_color(Color { r, g, b, a: 255 })
            })
            .collect_vec();
        
        // optimize palette and dither
        let converted_pixels = if let Some(num_colors) = num_colors {
            let mut palette = build_palette(&pixels, num_colors);
    
            // reduce palette's color depth
            for c in &mut palette {
                *c = self.convert_color(*c);
            }
    
            let colorspace = SimpleColorSpace::default();
            let ditherer = FloydSteinberg::new();
            let remapper = Remapper::new(&palette, &colorspace, &ditherer);
            let indexed_data = remapper.remap(&pixels, image.width() as usize);
            let pixels = indexed_data
                .into_iter()
                .map(|i| palette[i as usize])
                .collect_vec();
            
            pixels
        } else {
            pixels
        };
        let loss = Iterator::zip(original.into_iter(), converted_pixels.iter())
            .map(|(a, b)| color_diff_l1(a, *b))
            .sum::<u64>();

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

    fn l1_loss(&self, _: Color) -> u64 {
        0
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
            r: (r.saturating_add(2)) & !0x03,
            g: (g.saturating_add(2)) & !0x03,
            b: (b.saturating_add(2)) & !0x03,
            a,
        }
    }

    fn l1_loss(&self, pixel: Color) -> u64 {
        u64::from(pixel.r & 0x03) +
        u64::from(pixel.g & 0x03) +
        u64::from(pixel.b & 0x03)
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct Vga16BitMapper;

impl ColorMapper for Vga16BitMapper {
    fn convert_color(&self, pixel: Color) -> Color {
        let Color { r, g, b, a } = pixel;
        Color {
            r: (r.saturating_add(2)) & !0x07,
            g: (g.saturating_add(2)) & !0x03,
            b: (b.saturating_add(2)) & !0x07,
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

impl<T> ColorDepth for FixedPalette<T>
where
    T: AsRef<[[u8; 3]]>,
{
    fn convert_image_with_loss(&self, image: &RgbImage, num_colors: Option<u32>) -> (Vec<Color>, u64) {
        let original = image
            .pixels()
            .map(|&p| {
                let Rgb { data: [r, g, b] } = p;
                Color { r, g, b, a: 255 }
            })
            .collect_vec();
        let pixels = image
            .pixels()
            .map(|&p| {
                let Rgb { data: [r, g, b] } = p;
                self.convert_color(Color { r, g, b, a: 255 })
            })
            .collect_vec();
        
        // optimize palette and dither
        let converted_pixels = if let Some(num_colors) = num_colors {
            let mut palette = build_palette(&pixels, num_colors);
    
            // reduce palette's color depth
            for c in &mut palette {
                *c = self.convert_color(*c);
            }
    
            let colorspace = SimpleColorSpace::default();
            let ditherer = FloydSteinberg::new();
            let remapper = Remapper::new(&palette, &colorspace, &ditherer);
            let indexed_data = remapper.remap(&pixels, image.width() as usize);
            let pixels = indexed_data
                .into_iter()
                .map(|i| palette[i as usize])
                .collect_vec();
            
            pixels
        } else {
            pixels
        };
        let loss = Iterator::zip(original.into_iter(), converted_pixels.iter())
            .map(|(a, b)| color_diff_l1(a, *b))
            .sum::<u64>();

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
    optimizer.optimize_palette(&colorspace, &palette, &histogram, 4)
}

/// Color depth emulating a combination of one freely selectable
/// background color (`B`) with any of the other colors (`F`).
#[derive(Debug)]
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
        let [r, g, b] = palette
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

    fn convert_color_back(&self, pixel: Color) -> Color {
        BackForePalette::<B, F>::convert_color(pixel, &self.0)
    }

    fn convert_color_fore(&self, pixel: Color) -> Color {
        BackForePalette::<B, F>::convert_color(pixel, &self.1)
    }

    /// Identify the best background color
    fn background_color(&self, image: &RgbImage) -> Color {
        // we'll fetch the median color of the image for the time being
        let original = image
            .pixels()
            .map(|&p| {
                let Rgb { data: [r, g, b] } = p;
                Color { r, g, b, a: 255 }
            })
            .collect_vec();
        let mut r = original.iter().cloned().map(|Color { r, ..}| r).collect_vec();
        let mut g = original.iter().cloned().map(|Color { r, ..}| r).collect_vec();
        let mut b = original.iter().cloned().map(|Color { r, ..}| r).collect_vec();
        r.sort_unstable();
        g.sort_unstable();
        b.sort_unstable();

        Color {
            r: r[r.len() / 2],
            g: g[g.len() / 2],
            b: b[b.len() / 2],
            a: 255,
        }
    }
}



impl<B, F> ColorDepth for BackForePalette<B, F>
where
    B: AsRef<[[u8; 3]]>,
    F: AsRef<[[u8; 3]]>,
{
    fn convert_image_with_loss(&self, image: &RgbImage, num_colors: Option<u32>) -> (Vec<Color>, u64) {
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
                let Rgb { data: [r, g, b] } = p;
                Color { r, g, b, a: 255 }
            })
            .collect_vec();
        let pixels = image
            .pixels()
            .map(|&p| {
                let Rgb { data: [r, g, b] } = p;
                fixed.convert_color(Color { r, g, b, a: 255 })
            })
            .collect_vec();
        
        // optimize palette and dither
        let converted_pixels = if let Some(num_colors) = num_colors {
            let mut palette = build_palette(&pixels, num_colors);
    
            // reduce palette's color depth
            for c in &mut palette {
                *c = fixed.convert_color(*c);
            }
    
            let colorspace = SimpleColorSpace::default();
            let ditherer = FloydSteinberg::new();
            let remapper = Remapper::new(&palette, &colorspace, &ditherer);
            let indexed_data = remapper.remap(&pixels, image.width() as usize);
            let pixels = indexed_data
                .into_iter()
                .map(|i| palette[i as usize])
                .collect_vec();
            
            pixels
        } else {
            pixels
        };
        let loss = Iterator::zip(original.into_iter(), converted_pixels.iter())
            .map(|(a, b)| color_diff_l1(a, *b))
            .sum::<u64>();

        (converted_pixels, loss)
    }
}


/// Reduce the color palette of the given image according to the provided
/// color depth and maximum number of simultaneous colors.
#[deprecated]
pub fn map_to_retro_color_palette<D>(
    image: RgbImage,
    depth: D,
    num_colors: Option<u32>,
) -> RgbImage
where
    D: ColorDepth + ColorMapper,
{
    let pixels = image
        .pixels()
        .map(|&Rgb { data: [r, g, b] }| Color { r, g, b, a: 255 })
        .collect_vec();

    if let Some(num_colors) = num_colors {
        let mut palette = build_palette(&pixels, num_colors);

        // reduce palette's color depth
        for c in &mut palette {
            *c = depth.convert_color(*c);
        }

        let colorspace = SimpleColorSpace::default();
        let ditherer = FloydSteinberg::new();
        let remapper = Remapper::new(&palette, &colorspace, &ditherer);
        let indexed_data = remapper.remap(&pixels, image.width() as usize);
        let pixels = indexed_data
            .into_iter()
            .map(|i| palette[i as usize])
            .flat_map(|Color { r, g, b, .. }| value_iter![r, g, b])
            .collect_vec();
        ImageBuffer::from_raw(image.width(), image.height(), pixels)
            .expect("there should be enough pixels")
    } else {
        colors_to_image(image.width(), image.height(), pixels)
    }
}

pub fn colors_to_image<I>(width: u32, height: u32, pixels: I) -> RgbImage 
where
    I: IntoIterator<Item = Color>,
{
    let pixels = pixels.into_iter()
            .flat_map(|Color { r, g, b, .. }| value_iter![r, g, b])
            .collect_vec();
    ImageBuffer::from_raw(width, height, pixels)
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

    let pixels = depth.convert_image(&image, None);

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
