use super::{BackForePalette, BestPalette, FixedPalette};

pub static CGA_4BIT: [[u8; 3]; 16] = [
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
];

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
pub static PALETTE_CGA_4BIT: FixedPalette<[[u8; 3]; 16]> = FixedPalette(CGA_4BIT);

pub type CgaMod4Subpalette = BackForePalette<[[u8; 3]; 16], [[u8; 3]; 3]>;

/// CGA Mode 4, palette 0 in low intensity
pub static CGA_MODE4_0_LOW: [[u8; 3]; 3] = [
    [0, 0xAA, 0],    // green
    [0xAA, 0, 0],    // red
    [0xAA, 0x55, 0], // brown
];

/// CGA Mode 4, palette 0 in high intensity
pub static CGA_MODE4_0_HIGH: [[u8; 3]; 3] = [
    [0x55, 0xFF, 0x55], // green
    [0xFF, 0x55, 0x55], // red
    [0xFF, 0xFF, 0x55], // brown
];

/// CGA Mode 4, palette 1 in low intensity
pub static CGA_MODE4_1_LOW: [[u8; 3]; 3] = [
    [0x55, 0xFF, 0xFF], // cyan
    [0xFF, 0x55, 0xFF], // magenta
    [0xFF, 0xFF, 0xFF], // gray
];

/// CGA Mode 4, palette 1 in high intensity
pub static CGA_MODE4_1_HIGH: [[u8; 3]; 3] = [
    [0x55, 0xFF, 0xFF], // cyan
    [0xFF, 0x55, 0xFF], // magenta
    [0xFF, 0xFF, 0xFF], // white
];

pub static PALETTE_CGA_MODE4_1_HIGH: CgaMod4Subpalette = BackForePalette(CGA_4BIT, CGA_MODE4_1_HIGH);
pub static PALETTE_CGA_MODE4_0_HIGH: CgaMod4Subpalette = BackForePalette(CGA_4BIT, CGA_MODE4_0_HIGH);
pub static PALETTE_CGA_MODE4_1_LOW: CgaMod4Subpalette = BackForePalette(CGA_4BIT, CGA_MODE4_1_LOW);
pub static PALETTE_CGA_MODE4_0_LOW: CgaMod4Subpalette = BackForePalette(CGA_4BIT, CGA_MODE4_0_LOW);

/// CGA Mode 4, the best sub-palette is automatically discovered.
/// The default color is configurable to any of the colors in [`CGA_4BIT`].
///
/// [`CGA_4BIT`]: ./static.CGA_4BIT.html
pub static PALETTE_CGA_MODE4: BestPalette<
    &[BackForePalette<[[u8; 3]; 16], [[u8; 3]; 3]>],
> = BestPalette(&[
    BackForePalette(CGA_4BIT, CGA_MODE4_0_LOW),
    BackForePalette(CGA_4BIT, CGA_MODE4_0_HIGH),
    BackForePalette(CGA_4BIT, CGA_MODE4_1_LOW),
    BackForePalette(CGA_4BIT, CGA_MODE4_1_HIGH),
]);
