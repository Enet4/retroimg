use super::{ColorDepth, FixedPalette};

/// 16 color palette established by the full-color CGA standard, often seen in
/// EGA mode as the default palette.
/// 
/// Note 1: In practice, CGA would not let you use all 16 colors simultaneously
/// unless in a very low resolution (160x100). Moreover, even in 320x200, three
/// of the four colors that a program could choose were fixed to specific
/// subsets of this pallete. For a more realistic simulation of CGA, see also
/// the separate palettes `CGA_MODE4_0_LOW`, `CGA_MODE4_0_HIGH`,
/// `CGA_MODE4_1_LOW` and `CGA_MODE4_1_HIGH` palettes.
/// 
/// Note 2: This does not attempt to reproduce CGA's composite color output.
pub static CGA_4BIT: FixedPalette<[[u8; 3]; 16]> = FixedPalette([
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

#[derive(Debug)]
pub struct CgaMode4;

/// Reproduce a specific CGA mode 4 color output, in which three of the colors
/// are fixed and the default color is configurable to any of the colors in
/// [`CGA_4BIT`].
/// 
/// [`CGA_4BIT`]: ./static.CGA_4BIT.html
#[derive(Debug)]
pub struct CgaMode4Palette([[u8; 3]; 3]);

/// CGA Mode 4, palette 0 in low intensity
pub static CGA_MODE4_0_LOW: CgaMode4Palette = CgaMode4Palette([
    [0, 0xAA, 0], // green
    [0xAA, 0, 0], // red
    [0xAA, 0x55, 0], // brown
]);

/// CGA Mode 4, palette 0 in high intensity
pub static CGA_MODE4_0_HIGH: CgaMode4Palette = CgaMode4Palette([
    [0x55, 0xFF, 0x55], // green
    [0xFF, 0x55, 0x55], // red
    [0xFF, 0xFF, 0x55], // brown
]);

/// CGA Mode 4, palette 1 in low intensity
pub static CGA_MODE4_1_LOW: CgaMode4Palette = CgaMode4Palette([
    [0x55, 0xFF, 0xFF], // cyan
    [0xFF, 0x55, 0xFF], // magenta
    [0xFF, 0xFF, 0xFF], // gray
]);

/// CGA Mode 4, palette 1 in high intensity
pub static CGA_MODE4_1_HIGH: CgaMode4Palette = CgaMode4Palette([
    [0x55, 0xFF, 0xFF], // cyan
    [0xFF, 0x55, 0xFF], // magenta
    [0xFF, 0xFF, 0xFF], // white
]);
