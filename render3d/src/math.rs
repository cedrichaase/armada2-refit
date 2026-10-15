//! Vectors and matrices. Matrices are row-major with row vectors (v * M), as
//! Direct3D 9 and the engine use them; shaders receive them transposed.
use core::ops::{Add, Mul, Neg, Sub};

pub use libm::{acosf, atan2f, cosf, expf, fabsf, floorf, fmodf, logf, powf, sinf, sqrtf, tanf};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct V3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub const fn v3(x: f32, y: f32, z: f32) -> V3 {
    V3 { x, y, z }
}

impl V3 {
    pub const ZERO: V3 = v3(0.0, 0.0, 0.0);
    pub fn dot(self, o: V3) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    pub fn cross(self, o: V3) -> V3 {
        v3(self.y * o.z - self.z * o.y, self.z * o.x - self.x * o.z, self.x * o.y - self.y * o.x)
    }
    pub fn len(self) -> f32 {
        sqrtf(self.dot(self))
    }
    pub fn norm(self) -> V3 {
        let l = self.len();
        if l > 1e-20 { self * (1.0 / l) } else { self }
    }
    pub fn scale(self, s: f32) -> V3 {
        self * s
    }
    pub fn mul_comp(self, o: V3) -> V3 {
        v3(self.x * o.x, self.y * o.y, self.z * o.z)
    }
    pub fn lerp(self, o: V3, t: f32) -> V3 {
        self + (o - self) * t
    }
}

impl Add for V3 {
    type Output = V3;
    fn add(self, o: V3) -> V3 {
        v3(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}
impl Sub for V3 {
    type Output = V3;
    fn sub(self, o: V3) -> V3 {
        v3(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}
impl Mul<f32> for V3 {
    type Output = V3;
    fn mul(self, s: f32) -> V3 {
        v3(self.x * s, self.y * s, self.z * s)
    }
}
impl Neg for V3 {
    type Output = V3;
    fn neg(self) -> V3 {
        v3(-self.x, -self.y, -self.z)
    }
}

/// The engine's 3x4 transform: three rows of axes (right, up, front) then position,
/// 12 floats, row vectors. `Matrix34` in armada2.map.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct M34 {
    pub r: [V3; 3],
    pub p: V3,
}

impl M34 {
    pub const IDENTITY: M34 = M34 { r: [v3(1.0, 0.0, 0.0), v3(0.0, 1.0, 0.0), v3(0.0, 0.0, 1.0)], p: V3::ZERO };
    pub fn to_m4(&self) -> M4 {
        M4([
            [self.r[0].x, self.r[0].y, self.r[0].z, 0.0],
            [self.r[1].x, self.r[1].y, self.r[1].z, 0.0],
            [self.r[2].x, self.r[2].y, self.r[2].z, 0.0],
            [self.p.x, self.p.y, self.p.z, 1.0],
        ])
    }
    /// a then b: (v * a) * b
    pub fn then(&self, b: &M34) -> M34 {
        let rot = |v: V3| v3(v.dot(v3(b.r[0].x, b.r[1].x, b.r[2].x)), v.dot(v3(b.r[0].y, b.r[1].y, b.r[2].y)), v.dot(v3(b.r[0].z, b.r[1].z, b.r[2].z)));
        M34 { r: [rot(self.r[0]), rot(self.r[1]), rot(self.r[2])], p: rot(self.p) + b.p }
    }
    pub fn transform_point(&self, v: V3) -> V3 {
        self.r[0] * v.x + self.r[1] * v.y + self.r[2] * v.z + self.p
    }
    pub fn transform_dir(&self, v: V3) -> V3 {
        self.r[0] * v.x + self.r[1] * v.y + self.r[2] * v.z
    }
    /// Inverse of a rotation (+ uniform scale) with translation.
    pub fn inverse_affine(&self) -> M34 {
        let m = self.to_m4().inverse().unwrap_or(M4::IDENTITY);
        M34 {
            r: [v3(m.0[0][0], m.0[0][1], m.0[0][2]), v3(m.0[1][0], m.0[1][1], m.0[1][2]), v3(m.0[2][0], m.0[2][1], m.0[2][2])],
            p: v3(m.0[3][0], m.0[3][1], m.0[3][2]),
        }
    }
    pub fn det3(&self) -> f32 {
        self.r[0].dot(self.r[1].cross(self.r[2]))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct M4(pub [[f32; 4]; 4]);

impl Default for M4 {
    fn default() -> M4 {
        M4::IDENTITY
    }
}

impl M4 {
    pub const IDENTITY: M4 = M4([[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]]);
    pub fn mul(&self, b: &M4) -> M4 {
        let mut r = [[0.0f32; 4]; 4];
        for (i, row) in r.iter_mut().enumerate() {
            for (j, c) in row.iter_mut().enumerate() {
                *c = self.0[i][0] * b.0[0][j] + self.0[i][1] * b.0[1][j] + self.0[i][2] * b.0[2][j] + self.0[i][3] * b.0[3][j];
            }
        }
        M4(r)
    }
    pub fn transpose(&self) -> M4 {
        let mut r = [[0.0f32; 4]; 4];
        for (i, row) in r.iter_mut().enumerate() {
            for (j, c) in row.iter_mut().enumerate() {
                *c = self.0[j][i];
            }
        }
        M4(r)
    }
    pub fn transform(&self, v: [f32; 4]) -> [f32; 4] {
        let mut o = [0.0f32; 4];
        for (j, c) in o.iter_mut().enumerate() {
            *c = v[0] * self.0[0][j] + v[1] * self.0[1][j] + v[2] * self.0[2][j] + v[3] * self.0[3][j];
        }
        o
    }
    pub fn transform_point(&self, p: V3) -> V3 {
        let o = self.transform([p.x, p.y, p.z, 1.0]);
        v3(o[0], o[1], o[2])
    }
    pub fn as_floats(&self) -> &[f32; 16] {
        unsafe { &*(self.0.as_ptr() as *const [f32; 16]) }
    }
    pub fn inverse(&self) -> Option<M4> {
        let m = self.as_floats();
        let mut inv = [0.0f32; 16];
        inv[0] = m[5] * m[10] * m[15] - m[5] * m[11] * m[14] - m[9] * m[6] * m[15] + m[9] * m[7] * m[14] + m[13] * m[6] * m[11] - m[13] * m[7] * m[10];
        inv[4] = -m[4] * m[10] * m[15] + m[4] * m[11] * m[14] + m[8] * m[6] * m[15] - m[8] * m[7] * m[14] - m[12] * m[6] * m[11] + m[12] * m[7] * m[10];
        inv[8] = m[4] * m[9] * m[15] - m[4] * m[11] * m[13] - m[8] * m[5] * m[15] + m[8] * m[7] * m[13] + m[12] * m[5] * m[11] - m[12] * m[7] * m[9];
        inv[12] = -m[4] * m[9] * m[14] + m[4] * m[10] * m[13] + m[8] * m[5] * m[14] - m[8] * m[6] * m[13] - m[12] * m[5] * m[10] + m[12] * m[6] * m[9];
        inv[1] = -m[1] * m[10] * m[15] + m[1] * m[11] * m[14] + m[9] * m[2] * m[15] - m[9] * m[3] * m[14] - m[13] * m[2] * m[11] + m[13] * m[3] * m[10];
        inv[5] = m[0] * m[10] * m[15] - m[0] * m[11] * m[14] - m[8] * m[2] * m[15] + m[8] * m[3] * m[14] + m[12] * m[2] * m[11] - m[12] * m[3] * m[10];
        inv[9] = -m[0] * m[9] * m[15] + m[0] * m[11] * m[13] + m[8] * m[1] * m[15] - m[8] * m[3] * m[13] - m[12] * m[1] * m[11] + m[12] * m[3] * m[9];
        inv[13] = m[0] * m[9] * m[14] - m[0] * m[10] * m[13] - m[8] * m[1] * m[14] + m[8] * m[2] * m[13] + m[12] * m[1] * m[10] - m[12] * m[2] * m[9];
        inv[2] = m[1] * m[6] * m[15] - m[1] * m[7] * m[14] - m[5] * m[2] * m[15] + m[5] * m[3] * m[14] + m[13] * m[2] * m[7] - m[13] * m[3] * m[6];
        inv[6] = -m[0] * m[6] * m[15] + m[0] * m[7] * m[14] + m[4] * m[2] * m[15] - m[4] * m[3] * m[14] - m[12] * m[2] * m[7] + m[12] * m[3] * m[6];
        inv[10] = m[0] * m[5] * m[15] - m[0] * m[7] * m[13] - m[4] * m[1] * m[15] + m[4] * m[3] * m[13] + m[12] * m[1] * m[7] - m[12] * m[3] * m[5];
        inv[14] = -m[0] * m[5] * m[14] + m[0] * m[6] * m[13] + m[4] * m[1] * m[14] - m[4] * m[2] * m[13] - m[12] * m[1] * m[6] + m[12] * m[2] * m[5];
        inv[3] = -m[1] * m[6] * m[11] + m[1] * m[7] * m[10] + m[5] * m[2] * m[11] - m[5] * m[3] * m[10] - m[9] * m[2] * m[7] + m[9] * m[3] * m[6];
        inv[7] = m[0] * m[6] * m[11] - m[0] * m[7] * m[10] - m[4] * m[2] * m[11] + m[4] * m[3] * m[10] + m[8] * m[2] * m[7] - m[8] * m[3] * m[6];
        inv[11] = -m[0] * m[5] * m[11] + m[0] * m[7] * m[9] + m[4] * m[1] * m[11] - m[4] * m[3] * m[9] - m[8] * m[1] * m[7] + m[8] * m[3] * m[5];
        inv[15] = m[0] * m[5] * m[10] - m[0] * m[6] * m[9] - m[4] * m[1] * m[10] + m[4] * m[2] * m[9] + m[8] * m[1] * m[6] - m[8] * m[2] * m[5];
        let det = m[0] * inv[0] + m[1] * inv[4] + m[2] * inv[8] + m[3] * inv[12];
        if fabsf(det) < 1e-30 {
            return None;
        }
        let d = 1.0 / det;
        let mut r = [[0.0f32; 4]; 4];
        for i in 0..16 {
            r[i / 4][i % 4] = inv[i] * d;
        }
        Some(M4(r))
    }
}

pub fn clamp(x: f32, a: f32, b: f32) -> f32 {
    if x < a { a } else if x > b { b } else { x }
}

pub fn saturate(x: f32) -> f32 {
    clamp(x, 0.0, 1.0)
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// The engine's random generator, as `spec-particles.md` gives it: an LCG
/// s' = 2273 s + 31 mod 429397397, unsynchronised and seeded from the clock. Ours is
/// per system, so its sequence never touches the game's.
#[derive(Clone, Copy)]
pub struct Lcg(pub u32);

impl Lcg {
    pub fn next(&mut self) -> u32 {
        self.0 = ((self.0 as u64 * 2273 + 31) % 429_397_397) as u32;
        self.0
    }
    /// uniform in [0, 1)
    pub fn unit(&mut self) -> f32 {
        self.next() as f32 / 429_397_397.0
    }
    /// uniform in [-1, 1]
    pub fn signed(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }
    pub fn range(&mut self, m: f32) -> f32 {
        self.unit() * m
    }
}
