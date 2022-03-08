use image::imageops::{resize, FilterType};
use image::{GenericImage, ImageBuffer, Pixel, RgbImage};
use num::rational::Ratio;
use snafu::Snafu;

pub mod color;

pub use crate::color::{ColorDepth, FixedPalette};

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

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum ResolutionError {
    /// not enough components to resolve output resolution
    Non,
    /// 'width' or 'height' are required alongside 'pixel_ratio'
    RatioWithoutSide,
    /// 'pixel_ratio', 'width' and 'height' cannot be used together
    TooMany,
}

pub fn resolve_output_resolution(
    width: u32,
    height: u32,
    output_width: Option<u32>,
    output_height: Option<u32>,
    pixel_ratio: Option<Ratio<u32>>,
) -> Result<(u32, u32), ResolutionError> {
    match (pixel_ratio, output_width, output_height) {
        (None, None, None) => NonSnafu.fail(),
        (None, Some(w), Some(h)) => Ok((w, h)),
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
            let w = ((r * h * width) / height).round().to_integer();
            Ok((w, h))
        }
        (Some(r), Some(w), None) => {
            let h = (Ratio::from_integer(w) * height / (r * width))
                .round()
                .to_integer();
            Ok((w, h))
        }
        (None, None, Some(h)) => {
            let ir = Ratio::new(width, height);
            let w = (Ratio::from_integer(h) * ir).round().to_integer();
            Ok((w, h))
        }
        (None, Some(w), None) => {
            let ir = Ratio::new(width, height);
            let h = (Ratio::from_integer(w) / ir).round().to_integer();
            Ok((w, h))
        }
        (Some(_r), None, None) => RatioWithoutSideSnafu.fail(),
        (Some(_r), Some(_w), Some(_h)) => TooManySnafu.fail(),
    }
}
