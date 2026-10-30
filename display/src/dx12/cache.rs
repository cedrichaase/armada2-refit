//! The pipeline library on disk: `%LOCALAPPDATA%\armada2-refit\Display8.d3d12pipelines`,
//! beside the Vulkan cache (`vk/cache.rs`, the same reasons for the place and the same
//! discipline). Display8 owns one `ID3D12PipelineLibrary` and hands it to the plugins
//! (`Dx12Device8::pipeline_library`, contract 1.2), whose `gpu` stores and loads
//! pipeline states through it; Display8 also keeps its own present pass there.
//!
//! The file is a 64-byte header of ours followed by the library's blob. The header
//! names the file format, Display8's version and the adapter (vendor, device,
//! subsystem, revision, and the user-mode driver's version where DXGI says it); a file
//! whose header is not this device's is discarded whole. The library checks the blob
//! against the driver on its own (`D3D12_ERROR_DRIVER_VERSION_MISMATCH`,
//! `..._ADAPTER_NOT_FOUND`): that too starts it empty. It is written again whenever it
//! has grown, checked every `EVERY` frames, through a temporary file and a rename, so a
//! reader never sees half of one. The blob must outlive the library (D3D12 may point
//! into it), so the file's bytes stay in `Dx`.
use super::raw::*;
use crate::log;
use crate::rt;
use alloc::string::String;
use alloc::vec::Vec;
use core::ffi::c_void;

/// Frames between looks at the library's size.
pub const EVERY: u64 = 120;
const HEADER: usize = 64;
const MAGIC: u32 = 0x4c50_3241; // "A2PL"
const FORMAT: u32 = 1;

fn path() -> Option<String> {
    let mut d = rt::env("LOCALAPPDATA")?;
    d.push_str("\\armada2-refit");
    rt::create_dir(&d).then_some(d.clone())?;
    d.push_str("\\Display8.d3d12pipelines");
    Some(d)
}

fn fnv(s: &str) -> u32 {
    let mut h = 0x811c_9dc5u32;
    for b in s.bytes() {
        h = (h ^ b as u32).wrapping_mul(0x0100_0193);
    }
    h
}

/// The header words for the adapter the default device was made on: format, Display8's
/// version, vendor, device, subsystem, revision, driver version (low, high).
pub fn identity(create_factory: usize) -> [u32; 8] {
    let mut w = [MAGIC, FORMAT, fnv(env!("CARGO_PKG_VERSION")), 0, 0, 0, 0, 0];
    let create: unsafe extern "system" fn(*const crate::com::Guid, *mut P) -> i32 = unsafe { core::mem::transmute(create_factory) };
    let mut factory: P = core::ptr::null_mut();
    if unsafe { create(&IID_DXGI_FACTORY2, &mut factory) } < 0 || factory.is_null() {
        return w;
    }
    let mut adapter: P = core::ptr::null_mut();
    vcall!(factory, slot::FACTORY_ENUM_ADAPTERS, fn(u32, *mut P) -> i32, 0, &mut adapter);
    if !adapter.is_null() {
        // DXGI_ADAPTER_DESC: 128 WCHARs of name, then VendorId, DeviceId, SubSysId, Revision.
        let mut desc = [0u32; 80];
        if vcall!(adapter, slot::ADAPTER_GET_DESC, fn(*mut u32) -> i32, desc.as_mut_ptr()) >= 0 {
            w[3] = desc[64];
            w[4] = desc[65];
            w[5] = desc[66];
            w[6] = desc[67];
        }
        let mut umd = [0u32; 2];
        if vcall!(adapter, slot::ADAPTER_CHECK_INTERFACE_SUPPORT, fn(*const crate::com::Guid, *mut u32) -> i32, &IID_DXGI_DEVICE, umd.as_mut_ptr()) >= 0 {
            w[7] = umd[0] ^ umd[1].rotate_left(16);
        }
        release(adapter);
    }
    release(factory);
    w
}

fn header(id: &[u32; 8]) -> [u8; HEADER] {
    let mut h = [0u8; HEADER];
    for (i, v) in id.iter().enumerate() {
        h[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
    }
    h
}

/// The file as saved for this device (header included, so that the blob keeps its
/// place in the buffer), or nothing.
pub fn load(id: &[u32; 8]) -> Vec<u8> {
    let Some(p) = path() else { return Vec::new() };
    let Some(data) = rt::read_file(&p) else { return Vec::new() };
    if data.len() > HEADER && data[..HEADER] == header(id) {
        log!("d3d12: pipeline library: {} KB from {}", data.len() >> 10, p);
        data
    } else {
        log!("d3d12: pipeline library: {} is another device's, driver's or version's; starting empty", p);
        Vec::new()
    }
}

/// `ID3D12Device1::CreatePipelineLibrary` over `blob` (the file after its header; empty:
/// a new library). Null: the device has none, or refuses.
pub fn create(dev: P, blob: &[u8]) -> P {
    let mut dev1: P = core::ptr::null_mut();
    if vcall!(dev, slot::QUERY_INTERFACE, fn(*const crate::com::Guid, *mut P) -> i32, &IID_DEVICE1, &mut dev1) < 0 || dev1.is_null() {
        log!("d3d12: pipeline library: the device has no ID3D12Device1; the plugins create pipelines plainly");
        return core::ptr::null_mut();
    }
    let mut lib: P = core::ptr::null_mut();
    let ptr = if blob.is_empty() { core::ptr::null() } else { blob.as_ptr() as *const c_void };
    let mut hr = vcall!(dev1, slot::DEV1_CREATE_PIPELINE_LIBRARY, fn(*const c_void, usize, *const crate::com::Guid, *mut P) -> i32, ptr, blob.len(), &IID_PIPELINE_LIBRARY, &mut lib);
    if hr < 0 && !blob.is_empty() {
        log!("d3d12: pipeline library: the driver refused the saved one ({:#x}); starting empty", hr as u32);
        lib = core::ptr::null_mut();
        hr = vcall!(dev1, slot::DEV1_CREATE_PIPELINE_LIBRARY, fn(*const c_void, usize, *const crate::com::Guid, *mut P) -> i32, core::ptr::null(), 0, &IID_PIPELINE_LIBRARY, &mut lib);
    }
    release(dev1);
    if hr < 0 || lib.is_null() {
        log!("d3d12: pipeline library: CreatePipelineLibrary failed ({:#x}); the plugins create pipelines plainly", hr as u32);
        return core::ptr::null_mut();
    }
    lib
}

/// `StorePipeline` of `pso` under `name` (a wide string): false if the library refuses
/// (a name it has already is `E_INVALIDARG`).
pub fn store(lib: P, name: &[u16], pso: P) -> bool {
    vcall!(lib, slot::LIB_STORE_PIPELINE, fn(*const u16, P) -> i32, name.as_ptr(), pso) >= 0
}

pub fn size(lib: P) -> usize {
    vcall!(lib, slot::LIB_GET_SERIALIZED_SIZE, fn() -> usize,)
}

/// Writes the library when it has grown past `last` bytes; the size now.
pub fn save(lib: P, id: &[u32; 8], last: usize) -> usize {
    if lib.is_null() {
        return last;
    }
    let n = size(lib);
    if n <= last {
        return last;
    }
    let mut data = alloc::vec![0u8; HEADER + n];
    data[..HEADER].copy_from_slice(&header(id));
    if vcall!(lib, slot::LIB_SERIALIZE, fn(*mut c_void, usize) -> i32, data[HEADER..].as_mut_ptr() as *mut c_void, n) < 0 {
        log!("d3d12: pipeline library: Serialize failed");
        return last;
    }
    let Some(p) = path() else { return n };
    let mut tmp = p.clone();
    tmp.push_str(".tmp");
    if rt::write_file(&tmp, &data) && rt::move_file(&tmp, &p) {
        log!("d3d12: pipeline library: {} KB written to {}", data.len() >> 10, p);
    } else {
        log!("d3d12: pipeline library: could not write {}", p);
    }
    n
}
