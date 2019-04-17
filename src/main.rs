use image;
use num::integer::Integer;
use num::rational::Ratio;
use std::path::PathBuf;
use std::str::FromStr;
use structopt::StructOpt;

mod lib;

/// Retro effect to images
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

    /// Crop the input image to the rectangle (left, top, width, height)
    #[structopt(short = "C", long = "crop", parse(try_from_str = "parse_rect"))]
    crop: Option<(u16, u16, u16, u16)>,

    /// Resolution to resize the image into before color reduction
    #[structopt(
        name = "internal_resolution",
        short = "R",
        long = "res",
        default_value = "427x200",
        parse(try_from_str = "parse_resolution")
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
        parse(try_from_str = "parse_resolution")
    )]
    resolution: (u32, u32),

    /// Pixel ratio
    #[structopt(
        short = "r",
        long = "pixel-ratio",
        parse(try_from_str = "parse_ratio")
    )]
    pixel_ratio: Option<Ratio<u32>>,

    /// Output image width (defined separately)
    #[structopt(long = "width")]
    width: Option<u32>,

    /// Output image height (defined separately)
    #[structopt(long = "height")]
    height: Option<u32>,
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
        no_color_limit,
        num_colors,
        verbose,
    } = App::from_args();

    let mut img = image::open(input)?.to_rgb();

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
        (None, Some(w), Some(h)) => (w, h),
        (Some(r), None, Some(h)) => {
            /*
            Rule of proportions... with a twist.

            iW ----> oW

            iH ----> oH

            Without pixel scale correction (pixel ratio `r` = 1):

            oH = iH * oW / iW = oW / iR
            oW = iW * oH / iH = oH * iR

            For other pixel ratios:

            oR = iR * r

            Therefore:

            oW = oH * oR
               = oH * iR * r
               = oH * r * iW / iH

            and

            oH = oW / oR
               = oW / (iR * r)
               = oW / ( (iW / iH) * r)
               = oW * iH / (iW * r)
            */
            let w = ((r * h * in_width) / in_height).round().to_integer();
            (w, h)
        }
        (Some(r), Some(w), None) => {
            let h = (Ratio::from_integer(w) * in_height / (r * in_width))
                .round()
                .to_integer();
            (w, h)
        }
        (None, None, Some(h)) => {
            let ir = Ratio::new(in_width, in_height);
            let w = (Ratio::from_integer(h) * ir).round().to_integer();
            (w, h)
        }
        (None, Some(w), None) => {
            let ir = Ratio::new(in_width, in_height);
            let h = (Ratio::from_integer(w) / ir).round().to_integer();
            (w, h)
        }
        (Some(_r), None, None) => {
            panic!("'width' or 'height' are required alongside 'pixel_ratio'.")
        }
        (Some(_r), Some(_w), Some(_h)) => {
            panic!("The arguments 'pixel_ratio', 'width' and 'height' cannot be used together.")
        }
    };

    if verbose {
        eprintln!("External resolution: {} x {}", out_width, out_height);
    }
    let img = lib::reduce(&img, in_width, in_height);

    let num_colors = Some(num_colors as usize).filter(|_| !no_color_limit);

    let img = lib::map_to_retro_color_palette(img, num_colors);

    let img = lib::expand(&img, out_width, out_height);

    img.save(output)?;

    Ok(())
}
