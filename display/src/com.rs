//! The COM vocabulary of Direct3D 8 that the stub's objects (`stub/obj.rs`) and the
//! tracer share: `HRESULT`s, interface IDs, the interfaces (`Kind`) and what the engine
//! was handed an object as (`Role`). Display8 wraps no one's objects any more (forward
//! mode went in display 1.0.0): the stub's are its own, and after a bail-out the
//! engine holds the system's Direct3D 8 objects themselves (`bail.rs`).

pub type HRESULT = i32;
pub const S_OK: HRESULT = 0;
pub const E_NOINTERFACE: HRESULT = 0x8000_4002u32 as i32;
pub const E_POINTER: HRESULT = 0x8000_4003u32 as i32;
pub const D3DERR_INVALIDCALL: HRESULT = 0x8876_086cu32 as i32;

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Guid {
    pub d1: u32,
    pub d2: u16,
    pub d3: u16,
    pub d4: [u8; 8],
}

impl core::fmt::Display for Guid {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{:08x}-{:04x}-{:04x}-", self.d1, self.d2, self.d3)?;
        for (i, b) in self.d4.iter().enumerate() {
            if i == 2 {
                f.write_str("-")?;
            }
            write!(f, "{:02x}", b)?;
        }
        Ok(())
    }
}

const fn guid(d1: u32, d2: u16, d3: u16, d4: [u8; 8]) -> Guid {
    Guid { d1, d2, d3, d4 }
}

pub const IID_IUNKNOWN: Guid = guid(0, 0, 0, [0xc0, 0, 0, 0, 0, 0, 0, 0x46]);
pub const IID_D3D8: Guid = guid(0x1dd9e8da, 0x1c77, 0x4d40, [0xb0, 0xcf, 0x98, 0xfe, 0xfd, 0xff, 0x95, 0x12]);
pub const IID_DEVICE8: Guid = guid(0x7385e5df, 0x8fe8, 0x41d5, [0x86, 0xb6, 0xd7, 0xb4, 0x85, 0x47, 0xb6, 0xcf]);
pub const IID_RESOURCE8: Guid = guid(0x1b36bb7b, 0x09b7, 0x410a, [0xb4, 0x45, 0x7d, 0x14, 0x30, 0xd7, 0xb3, 0x3f]);
pub const IID_VERTEXBUFFER8: Guid = guid(0x8aeeeac7, 0x05f9, 0x44d4, [0xb5, 0x91, 0x00, 0x0b, 0x0d, 0xf1, 0xcb, 0x95]);
pub const IID_VOLUME8: Guid = guid(0xbd7349f5, 0x14f1, 0x42e4, [0x9c, 0x79, 0x97, 0x23, 0x80, 0xdb, 0x40, 0xc0]);
pub const IID_SWAPCHAIN8: Guid = guid(0x928c088b, 0x76b9, 0x4c6b, [0xa5, 0x36, 0xa5, 0x90, 0x85, 0x38, 0x76, 0xcd]);
pub const IID_SURFACE8: Guid = guid(0xb96eebca, 0xb326, 0x4ea5, [0x88, 0x2f, 0x2f, 0xf5, 0xba, 0xe0, 0x21, 0xdd]);
pub const IID_INDEXBUFFER8: Guid = guid(0x0e689c9a, 0x053d, 0x44a0, [0x9d, 0x92, 0xdb, 0x0e, 0x3d, 0x75, 0x0f, 0x86]);
pub const IID_BASETEXTURE8: Guid = guid(0xb4211cfa, 0x51b9, 0x4a9f, [0xab, 0x78, 0xdb, 0x99, 0xb2, 0xbb, 0x67, 0x8e]);
pub const IID_TEXTURE8: Guid = guid(0xe4cdd575, 0x2866, 0x4f01, [0xb1, 0x2e, 0x7e, 0xec, 0xe1, 0xec, 0x93, 0x58]);
pub const IID_CUBETEXTURE8: Guid = guid(0x3ee5b968, 0x2aca, 0x4c34, [0x8b, 0xb5, 0x7e, 0x0c, 0x3d, 0x19, 0xb7, 0x50]);
pub const IID_VOLUMETEXTURE8: Guid = guid(0x4b8aaafa, 0x140f, 0x42ba, [0x91, 0x31, 0x59, 0x7e, 0xaf, 0xaa, 0x2e, 0xad]);

/// Which interface an object of ours is.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u8)]
pub enum Kind {
    D3D,
    Device,
    SwapChain,
    Texture,
    CubeTexture,
    VolumeTexture,
    Surface,
    Volume,
    VertexBuffer,
    IndexBuffer,
    /// Not a COM interface: the user32 and gdi32 calls the tracer watches (`user32.rs`).
    User32,
}

pub const KINDS: usize = 11;

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::D3D => "IDirect3D8",
            Kind::Device => "IDirect3DDevice8",
            Kind::SwapChain => "IDirect3DSwapChain8",
            Kind::Texture => "IDirect3DTexture8",
            Kind::CubeTexture => "IDirect3DCubeTexture8",
            Kind::VolumeTexture => "IDirect3DVolumeTexture8",
            Kind::Surface => "IDirect3DSurface8",
            Kind::Volume => "IDirect3DVolume8",
            Kind::VertexBuffer => "IDirect3DVertexBuffer8",
            Kind::IndexBuffer => "IDirect3DIndexBuffer8",
            Kind::User32 => "user32",
        }
    }

    /// Whether `iid` names this interface or one it derives from.
    pub fn answers(self, iid: &Guid) -> bool {
        if *iid == IID_IUNKNOWN {
            return true;
        }
        match self {
            Kind::D3D => *iid == IID_D3D8,
            Kind::Device => *iid == IID_DEVICE8,
            Kind::SwapChain => *iid == IID_SWAPCHAIN8,
            Kind::Texture => *iid == IID_TEXTURE8 || *iid == IID_BASETEXTURE8 || *iid == IID_RESOURCE8,
            Kind::CubeTexture => *iid == IID_CUBETEXTURE8 || *iid == IID_BASETEXTURE8 || *iid == IID_RESOURCE8,
            Kind::VolumeTexture => *iid == IID_VOLUMETEXTURE8 || *iid == IID_BASETEXTURE8 || *iid == IID_RESOURCE8,
            Kind::Surface => *iid == IID_SURFACE8,
            Kind::Volume => *iid == IID_VOLUME8,
            Kind::VertexBuffer => *iid == IID_VERTEXBUFFER8 || *iid == IID_RESOURCE8,
            Kind::IndexBuffer => *iid == IID_INDEXBUFFER8 || *iid == IID_RESOURCE8,
            Kind::User32 => false,
        }
    }

    /// A resource's kind from `D3DRESOURCETYPE`.
    pub fn of_rtype(t: u32) -> Option<Kind> {
        Some(match t {
            1 => Kind::Surface,
            2 => Kind::Volume,
            3 => Kind::Texture,
            4 => Kind::VolumeTexture,
            5 => Kind::CubeTexture,
            6 => Kind::VertexBuffer,
            7 => Kind::IndexBuffer,
            _ => return None,
        })
    }
}

/// What the engine was handed an object as, where the call that made it says so.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Role {
    Plain,
    BackBuffer,
    DepthStencil,
    RenderTarget,
    Image,
}

impl Role {
    pub fn name(self) -> &'static str {
        match self {
            Role::Plain => "",
            Role::BackBuffer => " backbuffer",
            Role::DepthStencil => " depth",
            Role::RenderTarget => " target",
            Role::Image => " image",
        }
    }
}

/// The most slots a vtable of ours has (the device's 97).
pub const MAX_SLOTS: usize = 97;
