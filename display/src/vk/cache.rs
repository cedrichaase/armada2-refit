//! The pipeline cache on disk: `%LOCALAPPDATA%\armada2-refit\Display8.pipelines`, not
//! the game directory (GOG installs into `Program Files`, which is not writable, and an
//! exe with no manifest is silently redirected to VirtualStore there). Read when the
//! device is made, if its header names this device; written again whenever the cache
//! has grown, checked every few hundred frames, through a temporary file and a rename,
//! so a reader never sees half of one.
use crate::log;
use crate::rt;
use alloc::string::String;
use alloc::vec::Vec;
use ash::vk;

/// Frames between looks at the cache's size.
pub const EVERY: u64 = 600;

fn dir() -> Option<String> {
    let mut d = rt::env("LOCALAPPDATA")?;
    d.push_str("\\armada2-refit");
    rt::create_dir(&d).then_some(d)
}

fn path() -> Option<String> {
    let mut p = dir()?;
    p.push_str("\\Display8.pipelines");
    Some(p)
}

/// What was saved for this device (`VkPipelineCacheHeaderVersionOne`: length, version,
/// vendor, device, the cache UUID), or nothing. Vulkan must ignore data of another
/// device, but a driver that does not is not ours to find out about.
pub fn load(props: &vk::PhysicalDeviceProperties) -> Vec<u8> {
    let Some(p) = path() else { return Vec::new() };
    let Some(data) = rt::read_file(&p) else { return Vec::new() };
    let word = |i: usize| u32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
    let ok = data.len() >= 32
        && word(0) >= 32
        && word(4) == 1
        && word(8) == props.vendor_id
        && word(12) == props.device_id
        && data[16..32] == props.pipeline_cache_uuid;
    if ok {
        log!("vulkan: pipeline cache: {} KB from {}", data.len() >> 10, p);
        data
    } else {
        log!("vulkan: pipeline cache: {} is another device's or driver's; starting empty", p);
        Vec::new()
    }
}

/// Writes the cache when it has grown past `last` bytes; the size written.
pub fn save(dev: &ash::Device, cache: vk::PipelineCache, last: usize) -> usize {
    if cache == vk::PipelineCache::null() {
        return last;
    }
    let Ok(data) = (unsafe { dev.get_pipeline_cache_data(cache) }) else { return last };
    if data.len() <= last {
        return last;
    }
    let Some(p) = path() else { return data.len() };
    let mut tmp = p.clone();
    tmp.push_str(".tmp");
    if rt::write_file(&tmp, &data) && rt::move_file(&tmp, &p) {
        log!("vulkan: pipeline cache: {} KB written to {}", data.len() >> 10, p);
    } else {
        log!("vulkan: pipeline cache: could not write {}", p);
    }
    data.len()
}
