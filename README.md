# retroimg

Convert images to appear to be reproduced on retro IBM hardware.

| original (640x480, 24-bit RGB) | VGA (320x200, 256 colors, 4:5 pixels) | EGA (320x200, 16 colors, 4:5 pixels) |
|--------------------------------|---------------------------------------|--------------------------------------|
| ![](samples/pourville.png)     | ![](outputs/pourville-vga.png)        | ![](outputs/pourville-ega.png)       |

The full image processing pipeline is composed of the following steps:

1. Image cropping and resizing to a low resolution;
2. Color quantization and mapping to a restricted color palette and limit in number of colors with dithering;
3. Nearest-neighbor resizing to a high resolution, to make pixels look good, also enabling non-square pixels.


**Note:** This application does not claim to achieve a perfect emulation of old hardware, but it should hopefully attain sufficiently good results for the intended nostalgia kick.

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
