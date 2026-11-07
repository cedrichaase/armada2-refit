//! The sizes of a texture's levels, by format: bytes a block, the block's edge and what a
//! level of a given size takes (`stub/obj.rs` makes the memory, `stub/res.rs` hands out
//! the pitch). Pure, so it is host-tested (`display/hosttest`).
//!
//! Direct3D 8's block-compressed formats (DXT1 to DXT5) store 4x4 texels in 8 bytes (DXT1)
//! or 16 (the rest), and a level's pitch is one row of blocks. A level under one block
//! (2x2 and 1x1, at the end of a chain) is one block whole, so its pitch is a block's
//! bytes. A game that fills such a texture through `LockRect` (D3DX8 does) relies on it.

/// Bytes per block and the block's edge in texels, for a `D3DFORMAT` (None: one the
/// stub does not know, which it refuses).
pub fn format_block(fmt: u32) -> Option<(u32, u32)> {
    Some(match fmt {
        20 => (3, 1),
        21 | 22 | 32 | 33 | 31 | 35 | 62 | 63 | 64 | 65 | 67 | 71 | 75 | 77 | 79 => (4, 1),
        23 | 24 | 25 | 26 | 29 | 30 | 51 | 60 | 61 | 70 | 73 | 80 => (2, 1),
        27 | 28 | 41 | 50 | 52 => (1, 1),
        0x3154_5844 => (8, 4),                                            // DXT1
        0x3254_5844 | 0x3354_5844 | 0x3454_5844 | 0x3554_5844 => (16, 4), // DXT2..5
        _ => return None,
    })
}

/// The block-compressed kind of a `D3DFORMAT`: 1 for DXT1 (BC1), 2 for DXT2 and DXT3
/// (BC2), 3 for DXT4 and DXT5 (BC3). None: not block-compressed. DXT2 and DXT4 are the
/// premultiplied-alpha spellings of the same blocks.
pub fn bc_kind(fmt: u32) -> Option<u32> {
    match fmt {
        0x3154_5844 => Some(1),
        0x3254_5844 | 0x3354_5844 => Some(2),
        0x3454_5844 | 0x3554_5844 => Some(3),
        _ => None,
    }
}

/// A level's row pitch and size in bytes (one slice) for a `w` x `h` level of `fmt`.
pub fn level_size(fmt: u32, w: u32, h: u32) -> Option<(u32, u32)> {
    let (bytes, edge) = format_block(fmt)?;
    let (bw, bh) = (w.max(1).div_ceil(edge), h.max(1).div_ceil(edge));
    Some((bw * bytes, bw * bytes * bh))
}

/// The levels a texture of `levels` (0: a full chain) has.
pub fn level_count(levels: u32, w: u32, h: u32, d: u32) -> u32 {
    if levels != 0 {
        return levels;
    }
    let mut n = 1;
    let mut m = w.max(h).max(d);
    while m > 1 {
        m >>= 1;
        n += 1;
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    const DXT1: u32 = 0x3154_5844;
    const DXT3: u32 = 0x3354_5844;
    const DXT5: u32 = 0x3554_5844;

    #[test]
    fn dxt_blocks() {
        assert_eq!(format_block(DXT1), Some((8, 4)));
        assert_eq!(format_block(DXT3), Some((16, 4)));
        assert_eq!(format_block(DXT5), Some((16, 4)));
        assert_eq!(bc_kind(DXT1), Some(1));
        assert_eq!(bc_kind(0x3254_5844), Some(2));
        assert_eq!(bc_kind(DXT3), Some(2));
        assert_eq!(bc_kind(0x3454_5844), Some(3));
        assert_eq!(bc_kind(DXT5), Some(3));
        assert_eq!(bc_kind(21), None);
        assert_eq!(bc_kind(22), None);
    }

    #[test]
    fn dxt_pitch_is_a_row_of_blocks() {
        // 64x64 DXT1: 16 blocks of 8 bytes a row, 16 rows.
        assert_eq!(level_size(DXT1, 64, 64), Some((128, 128 * 16)));
        assert_eq!(level_size(DXT5, 64, 64), Some((256, 256 * 16)));
        assert_eq!(level_size(DXT5, 256, 128), Some((1024, 1024 * 32)));
    }

    #[test]
    fn small_levels_are_one_block() {
        for (w, h) in [(4, 4), (2, 2), (1, 1), (2, 1), (1, 2)] {
            assert_eq!(level_size(DXT1, w, h), Some((8, 8)), "{}x{}", w, h);
            assert_eq!(level_size(DXT3, w, h), Some((16, 16)), "{}x{}", w, h);
        }
        // A level that is not a multiple of 4 rounds up to whole blocks.
        assert_eq!(level_size(DXT1, 6, 6), Some((16, 32)));
        assert_eq!(level_size(DXT1, 4, 8), Some((8, 16)));
    }

    #[test]
    fn a_whole_chain_adds_up() {
        // A 64x64 DXT1 chain, seven levels: 16*16, 8*8, 4*4, 2*2 (1 block), 1x1 ... blocks of 8 bytes.
        let n = level_count(0, 64, 64, 1);
        assert_eq!(n, 7);
        let total: u32 = (0..n).map(|i| level_size(DXT1, 64 >> i, 64 >> i).unwrap().1).sum();
        assert_eq!(total, (256 + 64 + 16 + 4 + 1 + 1 + 1) * 8);
    }

    #[test]
    fn uncompressed_formats_are_unchanged() {
        assert_eq!(level_size(21, 128, 128), Some((512, 512 * 128)));
        assert_eq!(level_size(22, 3, 5), Some((12, 60)));
        assert_eq!(level_size(25, 3, 5), Some((6, 30)));
        assert_eq!(level_size(9999, 4, 4), None);
        assert_eq!(level_count(0, 128, 32, 1), 8);
        assert_eq!(level_count(3, 128, 32, 1), 3);
    }
}
