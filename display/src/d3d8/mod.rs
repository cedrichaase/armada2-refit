//! Direct3D 8's structures and method names as the stub (`stub/`) and the tracer use
//! them. Slot numbers and argument lists are Wine's `include/d3d8.h`; a method with the
//! wrong argument count unbalances the caller's stack, as `online/peer.c` learnt.

use crate::com::Kind;
use crate::stub::obj::{Body, SObj};
use crate::sfmt;
use alloc::string::String;
use core::ffi::c_void;

pub type HWND = *mut c_void;

/// `D3DPRESENT_PARAMETERS`, d3d8's.
#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct PresentParams {
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub count: u32,
    pub ms_type: u32,
    pub swap_effect: u32,
    pub window: usize,
    pub windowed: i32,
    pub auto_depth: i32,
    pub depth_format: u32,
    pub flags: u32,
    pub refresh: u32,
    pub interval: u32,
}

impl PresentParams {
    pub fn text(&self) -> String {
        sfmt!(
            "{}x{} fmt {} x{} ms {} swap {} hwnd {:#x} windowed {} autodepth {} fmt {} flags {:#x} refresh {} interval {:#x}",
            self.width,
            self.height,
            self.format,
            self.count,
            self.ms_type,
            self.swap_effect,
            self.window,
            self.windowed,
            self.auto_depth,
            self.depth_format,
            self.flags,
            self.refresh,
            self.interval
        )
    }
}

/// `D3DDISPLAYMODE`.
#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct DisplayMode {
    pub width: u32,
    pub height: u32,
    pub refresh: u32,
    pub format: u32,
}

/// `D3DSURFACE_DESC` (d3d8, with its `Size`).
#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct SurfaceDesc {
    pub format: u32,
    pub rtype: u32,
    pub usage: u32,
    pub pool: u32,
    pub size: u32,
    pub ms_type: u32,
    pub width: u32,
    pub height: u32,
}

/// `D3DVOLUME_DESC`.
#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct VolumeDesc {
    pub format: u32,
    pub rtype: u32,
    pub usage: u32,
    pub pool: u32,
    pub size: u32,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
}

/// `D3DVERTEXBUFFER_DESC` / `D3DINDEXBUFFER_DESC` (the index buffer's lacks `FVF`).
#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct BufferDesc {
    pub format: u32,
    pub rtype: u32,
    pub usage: u32,
    pub pool: u32,
    pub size: u32,
    pub fvf: u32,
}

/// `sizeof(D3DCAPS8)`.
pub const CAPS8_SIZE: usize = 212;

/// What a stub object is, for the log: its size, format, pool and usage.
pub fn describe(o: *mut SObj) -> String {
    let o = unsafe { &mut *o };
    match (o.kind, &o.body) {
        (Kind::Texture | Kind::CubeTexture | Kind::VolumeTexture, Body::Tex(t)) => {
            let l = t.levels.first();
            let (w, h, d) = l.map_or((0, 0, 0), |l| (l.w, l.h, l.d));
            sfmt!("{}x{}x{} fmt {} pool {} usage {:#x} levels {}", w, h, d, t.format, t.pool, t.usage, t.nlev)
        }
        (Kind::Surface | Kind::Volume, Body::Surf(s)) => sfmt!("{}x{} fmt {}", s.w, s.h, s.format),
        (Kind::VertexBuffer | Kind::IndexBuffer, Body::Buf(b)) => sfmt!("{} bytes fmt {:#x} pool {} usage {:#x}", b.data.len(), b.format, b.pool, b.usage),
        (k, _) => String::from(k.name()),
    }
}

/// `describe` for a pointer that may be null or not ours.
pub fn describe_any(p: *mut c_void) -> String {
    if p.is_null() {
        String::from("null")
    } else if crate::stub::obj::is_stub(p) {
        let o = p as *mut SObj;
        sfmt!("{}{} {}", unsafe { (*o).kind.name() }, unsafe { (*o).role.name() }, describe(o))
    } else {
        sfmt!("foreign {:#x}", p as usize)
    }
}

const D3D: [&str; 16] = [
    "QueryInterface", "AddRef", "Release", "RegisterSoftwareDevice", "GetAdapterCount", "GetAdapterIdentifier",
    "GetAdapterModeCount", "EnumAdapterModes", "GetAdapterDisplayMode", "CheckDeviceType", "CheckDeviceFormat",
    "CheckDeviceMultiSampleType", "CheckDepthStencilMatch", "GetDeviceCaps", "GetAdapterMonitor", "CreateDevice",
];

pub const DEVICE: [&str; 97] = [
    "QueryInterface", "AddRef", "Release", "TestCooperativeLevel", "GetAvailableTextureMem",
    "ResourceManagerDiscardBytes", "GetDirect3D", "GetDeviceCaps", "GetDisplayMode", "GetCreationParameters",
    "SetCursorProperties", "SetCursorPosition", "ShowCursor", "CreateAdditionalSwapChain", "Reset", "Present",
    "GetBackBuffer", "GetRasterStatus", "SetGammaRamp", "GetGammaRamp", "CreateTexture", "CreateVolumeTexture",
    "CreateCubeTexture", "CreateVertexBuffer", "CreateIndexBuffer", "CreateRenderTarget",
    "CreateDepthStencilSurface", "CreateImageSurface", "CopyRects", "UpdateTexture", "GetFrontBuffer",
    "SetRenderTarget", "GetRenderTarget", "GetDepthStencilSurface", "BeginScene", "EndScene", "Clear",
    "SetTransform", "GetTransform", "MultiplyTransform", "SetViewport", "GetViewport", "SetMaterial", "GetMaterial",
    "SetLight", "GetLight", "LightEnable", "GetLightEnable", "SetClipPlane", "GetClipPlane", "SetRenderState",
    "GetRenderState", "BeginStateBlock", "EndStateBlock", "ApplyStateBlock", "CaptureStateBlock",
    "DeleteStateBlock", "CreateStateBlock", "SetClipStatus", "GetClipStatus", "GetTexture", "SetTexture",
    "GetTextureStageState", "SetTextureStageState", "ValidateDevice", "GetInfo", "SetPaletteEntries",
    "GetPaletteEntries", "SetCurrentTexturePalette", "GetCurrentTexturePalette", "DrawPrimitive",
    "DrawIndexedPrimitive", "DrawPrimitiveUP", "DrawIndexedPrimitiveUP", "ProcessVertices", "CreateVertexShader",
    "SetVertexShader", "GetVertexShader", "DeleteVertexShader", "SetVertexShaderConstant",
    "GetVertexShaderConstant", "GetVertexShaderDeclaration", "GetVertexShaderFunction", "SetStreamSource",
    "GetStreamSource", "SetIndices", "GetIndices", "CreatePixelShader", "SetPixelShader", "GetPixelShader",
    "DeletePixelShader", "SetPixelShaderConstant", "GetPixelShaderConstant", "GetPixelShaderFunction",
    "DrawRectPatch", "DrawTriPatch", "DeletePatch",
];

const SWAPCHAIN: [&str; 5] = ["QueryInterface", "AddRef", "Release", "Present", "GetBackBuffer"];

const RESOURCE: [&str; 11] = [
    "QueryInterface", "AddRef", "Release", "GetDevice", "SetPrivateData", "GetPrivateData", "FreePrivateData",
    "SetPriority", "GetPriority", "PreLoad", "GetType",
];

const SURFACE: [&str; 11] = [
    "QueryInterface", "AddRef", "Release", "GetDevice", "SetPrivateData", "GetPrivateData", "FreePrivateData",
    "GetContainer", "GetDesc", "LockRect", "UnlockRect",
];

const VOLUME: [&str; 11] = [
    "QueryInterface", "AddRef", "Release", "GetDevice", "SetPrivateData", "GetPrivateData", "FreePrivateData",
    "GetContainer", "GetDesc", "LockBox", "UnlockBox",
];

const TEXTURE: [&str; 8] = ["SetLOD", "GetLOD", "GetLevelCount", "GetLevelDesc", "GetSurfaceLevel", "LockRect", "UnlockRect", "AddDirtyRect"];
const CUBE: [&str; 8] = ["SetLOD", "GetLOD", "GetLevelCount", "GetLevelDesc", "GetCubeMapSurface", "LockRect", "UnlockRect", "AddDirtyRect"];
const VOLTEX: [&str; 8] = ["SetLOD", "GetLOD", "GetLevelCount", "GetLevelDesc", "GetVolumeLevel", "LockBox", "UnlockBox", "AddDirtyBox"];
const BUFFER: [&str; 3] = ["Lock", "Unlock", "GetDesc"];

pub const USER32: [&str; 18] = [
    "GetCursorPos", "SetCursorPos", "ClipCursor", "GetClipCursor", "GetClientRect", "GetWindowRect",
    "ClientToScreen", "ScreenToClient", "GetSystemMetrics", "ChangeDisplaySettingsA", "EnumDisplaySettingsA",
    "SetWindowPos", "MoveWindow", "AdjustWindowRect", "AdjustWindowRectEx", "GetDeviceCaps", "ShowCursor",
    "SetCursor",
];

/// A method's name, for the inventory.
pub fn method_name(kind: Kind, slot: u8) -> &'static str {
    let s = slot as usize;
    let get = |t: &[&'static str], i: usize| t.get(i).copied().unwrap_or("?");
    match kind {
        Kind::D3D => get(&D3D, s),
        Kind::Device => get(&DEVICE, s),
        Kind::SwapChain => get(&SWAPCHAIN, s),
        Kind::Surface => get(&SURFACE, s),
        Kind::Volume => get(&VOLUME, s),
        Kind::Texture | Kind::CubeTexture | Kind::VolumeTexture | Kind::VertexBuffer | Kind::IndexBuffer => {
            if s < 11 {
                get(&RESOURCE, s)
            } else {
                match kind {
                    Kind::Texture => get(&TEXTURE, s - 11),
                    Kind::CubeTexture => get(&CUBE, s - 11),
                    Kind::VolumeTexture => get(&VOLTEX, s - 11),
                    _ => get(&BUFFER, s - 11),
                }
            }
        }
        Kind::User32 => get(&USER32, s),
    }
    .trim_start_matches(|_: char| false)
}
