//! The little linear algebra the UI needs.

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct M4(pub [[f32; 4]; 4]);

impl Default for M4 {
    fn default() -> M4 {
        M4::IDENTITY
    }
}

impl M4 {
    pub const IDENTITY: M4 = M4([[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]]);
}
