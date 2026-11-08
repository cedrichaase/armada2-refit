//! One entry point for a texture file in either format Armada II and Fleet Operations:
//! Roots use: `DDS ` magic is DDS, everything else is TGA.

use crate::{dds, tga};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Tga(tga::Error),
    Dds(dds::Error),
}

pub fn is_dds(data: &[u8]) -> bool {
    data.starts_with(b"DDS ")
}

/// Decode to BGRA8, rows top-down, whichever format the bytes are.
pub fn decode(data: &[u8]) -> Result<tga::Image, Error> {
    if is_dds(data) {
        dds::decode(data).map_err(Error::Dds)
    } else {
        tga::decode(data).map_err(Error::Tga)
    }
}

/// The base width from the header alone (`None` for a file too short or not readable).
pub fn width(data: &[u8]) -> Option<u32> {
    if is_dds(data) {
        dds::Header::parse(data).ok().map(|h| h.width)
    } else {
        tga::Header::parse(data).ok().map(|h| h.width as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniffs_both() {
        let mut t = [0u8; 18 + 3];
        t[2] = 2;
        t[12] = 5;
        t[14] = 1;
        t[16] = 24;
        assert_eq!(width(&t), Some(5));
        assert_eq!(decode(&t[..18]), Err(Error::Tga(tga::Error::Truncated)));
        let mut d = [0u8; 128];
        d[..4].copy_from_slice(b"DDS ");
        d[4] = 124;
        d[76] = 32;
        d[80] = 4;
        d[84..88].copy_from_slice(b"DXT1");
        d[12] = 4;
        d[16] = 12;
        assert_eq!(width(&d), Some(12));
        assert_eq!(decode(&d), Err(Error::Dds(dds::Error::Truncated)));
        assert_eq!(width(b"DDS "), None);
        assert_eq!(width(&[]), None);
    }
}
