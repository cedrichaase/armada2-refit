//! A bounds-checked little-endian cursor over a byte slice.

use alloc::string::String;

/// Where a read ran past the end: the offset it started at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Eof(pub usize);

pub struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8], pos: usize) -> Self {
        Reader { data, pos }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    pub fn take(&mut self, n: usize) -> Result<&'a [u8], Eof> {
        let end = self.pos.checked_add(n).ok_or(Eof(self.pos))?;
        if end > self.data.len() {
            return Err(Eof(self.pos));
        }
        let s = &self.data[self.pos..end];
        self.pos = end;
        Ok(s)
    }

    pub fn u8(&mut self) -> Result<u8, Eof> {
        Ok(self.take(1)?[0])
    }

    pub fn u16(&mut self) -> Result<u16, Eof> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    pub fn u32(&mut self) -> Result<u32, Eof> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn f32(&mut self) -> Result<f32, Eof> {
        Ok(f32::from_bits(self.u32()?))
    }

    pub fn f32s<const N: usize>(&mut self) -> Result<[f32; N], Eof> {
        let mut out = [0.0; N];
        for v in out.iter_mut() {
            *v = self.f32()?;
        }
        Ok(out)
    }

    /// A `u16` length, then that many bytes, read as Latin-1 (every byte is a char,
    /// so this cannot fail on content). A trailing NUL, if a writer left one, is dropped.
    pub fn string(&mut self) -> Result<String, Eof> {
        let n = self.u16()? as usize;
        let b = self.take(n)?;
        let b = match b.iter().position(|&c| c == 0) {
            Some(i) => &b[..i],
            None => b,
        };
        Ok(crate::text::latin1(b))
    }
}
