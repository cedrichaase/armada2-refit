//! An LZO1X decompressor, written from the stream format alone (the byte-oriented
//! instruction set described in public format notes), with every read and write checked.
//!
//! The stream is a run of instructions, each starting with a byte `b`:
//!
//! - `b >= 64`: a short match of 3 to 8 bytes at a distance up to 2048 (`b` carries the
//!   length, three bits of distance and the count of literals that follow; one more
//!   byte carries the rest of the distance).
//! - `32..64`: a match of 3 or more bytes at a distance up to 16384 (length in the low
//!   five bits, extended by a run of bytes when they are zero; a 16-bit word gives the
//!   distance and the literal count).
//! - `16..32`: a match at a distance of 16384 and more, the same way with a distance bit
//!   in `b`; a distance that comes to exactly 16384 is the end of the stream.
//! - `b < 16`: after a match with no literals, a run of literals (`b + 3`, extended when
//!   `b` is zero); after a match with 1 to 3 literals, a 2-byte match at a short
//!   distance; after a run of literals, a 3-byte match at a distance beyond 2048.
//!
//! The stream may open with a run of literals (a first byte above 17). The output size is
//! known by the caller and must be reached exactly: a stream that writes beyond it, reads
//! beyond its input, or ends short is an error.

use alloc::vec::Vec;

/// The most one stream may be asked to produce (a game file is far smaller).
pub const MAX_OUTPUT: usize = 64 << 20;

/// Why a stream was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// The input ended inside an instruction.
    Input,
    /// The output would pass the size the caller gave.
    Overrun,
    /// A match reaches back before the start of the output.
    Distance,
    /// The output buffer could not be allocated.
    Alloc,
    /// The stream ended (or the input ran out) before the output was full.
    Short,
}

struct Reader<'a> {
    src: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn byte(&mut self) -> Result<u8, Error> {
        let b = *self.src.get(self.pos).ok_or(Error::Input)?;
        self.pos += 1;
        Ok(b)
    }

    /// The extension of a length: each zero byte adds 255, the first other byte ends it.
    /// A run past `cap` (the output size) can only be a lie, and ends here.
    fn extension(&mut self, cap: usize) -> Result<usize, Error> {
        let mut n = 0usize;
        loop {
            let b = self.byte()?;
            if b != 0 {
                return Ok(n + b as usize);
            }
            n += 255;
            if n > cap {
                return Err(Error::Overrun);
            }
        }
    }
}

fn literals(r: &mut Reader, out: &mut Vec<u8>, n: usize, size: usize) -> Result<(), Error> {
    if size - out.len() < n {
        return Err(Error::Overrun);
    }
    let end = r.pos.checked_add(n).ok_or(Error::Input)?;
    let run = r.src.get(r.pos..end).ok_or(Error::Input)?;
    out.extend_from_slice(run);
    r.pos = end;
    Ok(())
}

fn copy_match(out: &mut Vec<u8>, dist: usize, len: usize, size: usize) -> Result<(), Error> {
    if dist == 0 || dist > out.len() {
        return Err(Error::Distance);
    }
    if size - out.len() < len {
        return Err(Error::Overrun);
    }
    let start = out.len() - dist;
    for i in 0..len {
        let b = out[start + i];
        out.push(b);
    }
    Ok(())
}

/// Decompresses `src` to exactly `size` bytes.
pub fn decompress(src: &[u8], size: usize) -> Result<Vec<u8>, Error> {
    // The most an instruction can expand is bounded (about 255 output bytes per input
    // byte), so a size far beyond that is a lie that would only cost an allocation.
    if size > MAX_OUTPUT || size > src.len().saturating_mul(255).saturating_add(512) {
        return Err(Error::Short);
    }
    let mut out: Vec<u8> = Vec::new();
    out.try_reserve_exact(size).map_err(|_| Error::Alloc)?;
    let mut r = Reader { src, pos: 0 };
    // How many literals the last instruction left to copy: 0 to 3 after a match, 4 after
    // a run of literals.
    let mut state = 0usize;

    if let Some(&first) = src.first() {
        if first > 17 {
            r.pos = 1;
            let n = first as usize - 17;
            literals(&mut r, &mut out, n, size)?;
            state = n.min(4);
        }
    }

    while r.pos < src.len() {
        let b = r.byte()? as usize;
        let (dist, len, trailing);
        if b >= 64 {
            let next = r.byte()? as usize;
            len = (b >> 5) + 1;
            dist = ((b >> 2) & 7) + (next << 3) + 1;
            trailing = b & 3;
        } else if b >= 32 {
            let mut l = b & 31;
            if l == 0 {
                l = 31 + r.extension(size)?;
            }
            let lo = r.byte()? as usize;
            let hi = r.byte()? as usize;
            let v = lo | hi << 8;
            len = l + 2;
            dist = (v >> 2) + 1;
            trailing = v & 3;
        } else if b >= 16 {
            let mut l = b & 7;
            if l == 0 {
                l = 7 + r.extension(size)?;
            }
            let lo = r.byte()? as usize;
            let hi = r.byte()? as usize;
            let v = lo | hi << 8;
            let far = ((b & 8) << 11) + (v >> 2);
            if far == 0 {
                break;
            }
            len = l + 2;
            dist = far + 16384;
            trailing = v & 3;
        } else if state == 0 {
            let n = if b == 0 { 15 + r.extension(size)? } else { b } + 3;
            literals(&mut r, &mut out, n, size)?;
            state = 4;
            continue;
        } else {
            let next = r.byte()? as usize;
            trailing = b & 3;
            if state < 4 {
                len = 2;
                dist = (b >> 2) + (next << 2) + 1;
            } else {
                len = 3;
                dist = (b >> 2) + (next << 2) + 2049;
            }
        }
        copy_match(&mut out, dist, len, size)?;
        literals(&mut r, &mut out, trailing, size)?;
        state = trailing;
    }

    if out.len() != size {
        return Err(Error::Short);
    }
    Ok(out)
}
