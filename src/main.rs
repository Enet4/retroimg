use num::integer::Integer;
use num::rational::Ratio;
use std::path::PathBuf;
use std::str::FromStr;
use structopt::StructOpt;

use retroimg as lib;

/// Convert images to look like in retro IBM hardware
#[derive(Debug, StructOpt)]
pub struct App {
    /// Image file
    #[structopt(name = "FILE", parse(from_os_str))]
    input: PathBuf,

    /// Output image file path
    #[structopt(
        short = "o",
        long = "out",
        default_value = "out.png",
        parse(from_os_str)
    )]
    output: PathBuf,

    /// Color standard
    #[structopt(short = "s", long = "standard", default_value = "vga")]
    standard: ColorStandard,

    /// Crop the input image to the rectangle (left, top, width, height)
    #[structopt(short = "C", long = "crop", parse(try_from_str = parse_rect))]
    crop: Option<(u16, u16, u16, u16)>,

    /// Resolution to resize the image into before color reduction
    #[structopt(
        name = "internal_resolution",
        short = "R",
        long = "res",
        default_value = "427x200",
        parse(try_from_str = parse_resolution)
    )]
    resolution: (u16, u16),

    #[structopt(flatten)]
    out_size: OutSizeOpts,

    /// Do not limit number of simultaneous colors (invalidates num_colors)
    #[structopt(long = "no-color-limit", conflicts_with = "num_colors")]
    no_color_limit: bool,

    /// Maximum number of simultaneous colors (emulates palette indexing)
    #[structopt(short = "c", long = "num-colors", default_value = "256")]
    num_colors: u16,

    /// Print some info to stderr
    #[structopt(short = "V", long = "verbose")]
    verbose: bool,
}

#[derive(Debug, StructOpt)]
struct OutSizeOpts {
    /// Output image size
    #[structopt(
        name = "external_resolution",
        short = "S",
        long = "out-size",
        default_value = "1920x1080",
        parse(try_from_str = parse_resolution),
    )]
    resolution: (u32, u32),

    /// Pixel ratio (format `w:h`)
    #[structopt(short = "r", long = "pixel-ratio", parse(try_from_str = parse_ratio))]
    pixel_ratio: Option<Ratio<u32>>,

    /// Output image width (defined separately)
    #[structopt(long = "width")]
    width: Option<u32>,

    /// Output image height (defined separately)
    #[structopt(long = "height")]
    height: Option<u32>,
}

/// Options for the kind of color palette to be simulated.
/// This doesn't affect the image's resolution.
#[derive(Debug)]
pub enum ColorStandard {
    /// True color 24-bit RGB (8 bits per channel)
    True24Bit,
    /// 18-bit RGB (6 bits per channel)
    Vga18Bit,
    /// 16-bit RGB, also called High color (5-6-5 bits per R-G-B channel)
    Vga16Bit,
    /// Mode 4 of CGA: 3 colors from hardcoded sub-palettes + 1 back color
    CgaMode4,
    /// Mode 4 of CGA, high intensity of sub-palette 1:
    /// white, cyan, magenta, and one arbitrary back color
    CgaMode4High1,
    /// All 16 colors from the CGA palette
    FullCga,
    /// All 64 colors from the EGA palette
    FullEga,
}

impl FromStr for ColorStandard {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "true" | "24bit" => Ok(ColorStandard::True24Bit),
            "vga" | "18bit" => Ok(ColorStandard::Vga18Bit),
            "high" | "16bit" => Ok(ColorStandard::Vga16Bit),
            "cga" | "cgamode4" => Ok(ColorStandard::CgaMode4),
            "cgamode4high1" => Ok(ColorStandard::CgaMode4High1),
            "fullcga" => Ok(ColorStandard::FullCga),
            "ega" => Ok(ColorStandard::FullEga),
            _ => Err("no such color standard"),
        }
    }
}

fn parse_rect<T>(value: &str) -> Result<(T, T, T, T), <T as FromStr>::Err>
where
    T: FromStr,
{
    let parts: Vec<_> = value.split(',').collect();

    assert_eq!(parts.len(), 4);

    Ok((
        parts[0].parse()?,
        parts[1].parse()?,
        parts[2].parse()?,
        parts[3].parse()?,
    ))
}

fn parse_resolution<T>(value: &str) -> Result<(T, T), <T as FromStr>::Err>
where
    T: FromStr,
{
    let parts: Vec<_> = value.split('x').collect();

    assert_eq!(
        parts.len(),
        2,
        "Number of components should be 2 (<width>x<height>)"
    );

    Ok((parts[0].parse()?, parts[1].parse()?))
}

fn parse_ratio<T>(value: &str) -> Result<Ratio<T>, <T as FromStr>::Err>
where
    T: FromStr,
    T: Clone,
    T: Integer,
{
    let parts: Vec<_> = value.split(':').collect();

    assert_eq!(
        parts.len(),
        2,
        "Number of components should be 2 (<width>:<height>)"
    );

    Ok(Ratio::new(parts[0].parse()?, parts[1].parse()?))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let App {
        input,
        output,
        crop,
        resolution: (in_width, in_height),
        out_size:
            OutSizeOpts {
                resolution: (res_out_width, res_out_height),
                pixel_ratio,
                width: out_width,
                height: out_height,
            },
        standard,
        no_color_limit,
        num_colors,
        verbose,
    } = App::from_args();

    let mut img = image::open(input)?.to_rgb8();

    if let Some((left, top, width, height)) = crop {
        img = lib::crop(
            img,
            u32::from(left),
            u32::from(top),
            u32::from(width),
            u32::from(height),
        );
    }
    let in_width = u32::from(in_width);
    let in_height = u32::from(in_height);

    if verbose {
        eprintln!("Emulated internal resolution: {} x {}", in_width, in_height);
    }

    let (out_width, out_height) = match (pixel_ratio, out_width, out_height) {
        (None, None, None) => (res_out_width, res_out_height),
        _ => {
            lib::resolve_output_resolution(in_width, in_height, out_width, out_height, pixel_ratio)
                .unwrap()
        }
    };

    if verbose {
        eprintln!("External resolution: {} x {}", out_width, out_height);
    }
    let img = lib::reduce(&img, in_width, in_height);

    let num_colors = Some(num_colors as u32).filter(|_| !no_color_limit);

    let depth: Box<dyn lib::ColorDepth> = match standard {
        ColorStandard::True24Bit => Box::new(lib::color::TrueColor24Bit::default()),
        ColorStandard::Vga18Bit => Box::new(lib::color::Vga18Bit::default()),
        ColorStandard::Vga16Bit => Box::new(lib::color::Vga16Bit::default()),
        ColorStandard::FullEga => Box::new(lib::color::ega::PALETTE_EGA_6BIT),
        ColorStandard::FullCga => Box::new(lib::color::cga::PALETTE_CGA_4BIT),
        ColorStandard::CgaMode4 => Box::new(lib::color::cga::PALETTE_CGA_MODE4),
        ColorStandard::CgaMode4High1 => Box::new(lib::color::cga::PALETTE_CGA_MODE4_1_HIGH),
    };

    let colorbuffer = depth.convert_image(&img, num_colors);
    let img = lib::color::colors_to_image(img.width(), img.height(), colorbuffer);
    let img = lib::expand(&img, out_width, out_height);

    img.save(output)?;

    Ok(())
}
