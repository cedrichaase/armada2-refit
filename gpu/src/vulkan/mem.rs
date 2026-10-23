//! Device memory in blocks, so that a few thousand textures and buffers do not take a
//! few thousand allocations (`maxMemoryAllocationCount` is 4096 on some drivers): each
//! block one memory type, images and buffers apart (`bufferImageGranularity`), handed
//! out first fit and merged back when freed. Big ones get an allocation of their own.
//! A host-visible block is mapped whole once, when first asked; blocks are never given
//! back (the game's working set settles).
use alloc::vec::Vec;
use ash::vk;

const BLOCK_DEVICE: u64 = 32 << 20;
/// Mapped whole: kept small in a 32-bit process's address space.
const BLOCK_HOST: u64 = 4 << 20;
const OWN: u64 = 8 << 20;

pub struct Alloc {
    pub mem: vk::DeviceMemory,
    pub offset: u64,
    pub size: u64,
    block: Option<usize>,
}

struct Block {
    mem: vk::DeviceMemory,
    ty: u32,
    image: bool,
    /// Free ranges (offset, length), by offset.
    free: Vec<(u64, u64)>,
    mapped: *mut u8,
}

#[derive(Default)]
pub struct Pools {
    blocks: Vec<Block>,
    pub count: u32,
}

impl Pools {
    pub fn alloc(&mut self, dev: &ash::Device, ty: u32, size: u64, align: u64, image: bool, host: bool) -> Option<Alloc> {
        let align = align.max(1);
        if size > OWN || (host && size > BLOCK_HOST / 2) {
            let mem = unsafe { dev.allocate_memory(&vk::MemoryAllocateInfo::default().allocation_size(size).memory_type_index(ty), None) }.ok()?;
            self.count += 1;
            return Some(Alloc { mem, offset: 0, size, block: None });
        }
        for (bi, b) in self.blocks.iter_mut().enumerate() {
            if b.ty != ty || b.image != image {
                continue;
            }
            if let Some(off) = take(&mut b.free, size, align) {
                return Some(Alloc { mem: b.mem, offset: off, size, block: Some(bi) });
            }
        }
        let bsize = if host { BLOCK_HOST } else { BLOCK_DEVICE };
        let mem = unsafe { dev.allocate_memory(&vk::MemoryAllocateInfo::default().allocation_size(bsize).memory_type_index(ty), None) }.ok()?;
        self.count += 1;
        let mut b = Block { mem, ty, image, free: alloc::vec![(0, bsize)], mapped: core::ptr::null_mut() };
        let off = take(&mut b.free, size, align)?;
        self.blocks.push(b);
        Some(Alloc { mem, offset: off, size, block: Some(self.blocks.len() - 1) })
    }

    pub fn free(&mut self, dev: &ash::Device, a: Alloc) {
        match a.block {
            None => {
                unsafe { dev.free_memory(a.mem, None) };
                self.count -= 1;
            }
            Some(bi) => {
                let f = &mut self.blocks[bi].free;
                let at = f.partition_point(|r| r.0 < a.offset);
                f.insert(at, (a.offset, a.size));
                // Merge with the neighbours.
                if at + 1 < f.len() && f[at].0 + f[at].1 == f[at + 1].0 {
                    f[at].1 += f[at + 1].1;
                    f.remove(at + 1);
                }
                if at > 0 && f[at - 1].0 + f[at - 1].1 == f[at].0 {
                    f[at - 1].1 += f[at].1;
                    f.remove(at);
                }
            }
        }
    }

    /// A pointer to `a`'s bytes (its memory must be host-visible). An allocation of its
    /// own is mapped by itself and unmapped when freed (`vkFreeMemory` does).
    pub fn map(&mut self, dev: &ash::Device, a: &Alloc) -> Option<*mut u8> {
        match a.block {
            None => unsafe { dev.map_memory(a.mem, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty()) }.ok().map(|p| p as *mut u8),
            Some(bi) => {
                let b = &mut self.blocks[bi];
                if b.mapped.is_null() {
                    b.mapped = unsafe { dev.map_memory(b.mem, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty()) }.ok()? as *mut u8;
                }
                Some(unsafe { b.mapped.add(a.offset as usize) })
            }
        }
    }
}

impl Pools {
    /// Unmaps an allocation of its own (a block stays mapped).
    pub fn unmap(&mut self, dev: &ash::Device, a: &Alloc) {
        if a.block.is_none() {
            unsafe { dev.unmap_memory(a.mem) };
        }
    }
}

/// The first free range that takes `size` at `align`, cut out of the list.
fn take(free: &mut Vec<(u64, u64)>, size: u64, align: u64) -> Option<u64> {
    for i in 0..free.len() {
        let (o, l) = free[i];
        let a = o.div_ceil(align) * align;
        if a + size > o + l {
            continue;
        }
        let tail = (a + size, o + l - (a + size));
        let head = (o, a - o);
        free.remove(i);
        let mut at = i;
        if head.1 > 0 {
            free.insert(at, head);
            at += 1;
        }
        if tail.1 > 0 {
            free.insert(at, tail);
        }
        return Some(a);
    }
    None
}
