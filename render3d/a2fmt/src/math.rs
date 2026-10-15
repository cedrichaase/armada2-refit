//! The engine's 3x4 transform, as SOD files store it.

/// Twelve floats: three basis vectors (the node's x, y and z axes, in its parent's
/// space) and then the translation. Points are row vectors, as in Direct3D:
/// `p' = p.x * axis(0) + p.y * axis(1) + p.z * axis(2) + translation`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Matrix34 {
    pub m: [f32; 12],
}

impl Matrix34 {
    pub const IDENTITY: Matrix34 = Matrix34 {
        m: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
    };

    /// Basis vector `i` (0 = x, 1 = y, 2 = z).
    pub fn axis(&self, i: usize) -> [f32; 3] {
        let i = i.min(2) * 3;
        [self.m[i], self.m[i + 1], self.m[i + 2]]
    }

    pub fn translation(&self) -> [f32; 3] {
        [self.m[9], self.m[10], self.m[11]]
    }

    pub fn transform_point(&self, p: [f32; 3]) -> [f32; 3] {
        let m = &self.m;
        [
            p[0] * m[0] + p[1] * m[3] + p[2] * m[6] + m[9],
            p[0] * m[1] + p[1] * m[4] + p[2] * m[7] + m[10],
            p[0] * m[2] + p[1] * m[5] + p[2] * m[8] + m[11],
        ]
    }

    pub fn transform_vector(&self, v: [f32; 3]) -> [f32; 3] {
        let m = &self.m;
        [
            v[0] * m[0] + v[1] * m[3] + v[2] * m[6],
            v[0] * m[1] + v[1] * m[4] + v[2] * m[7],
            v[0] * m[2] + v[1] * m[5] + v[2] * m[8],
        ]
    }

    /// `self` followed by `parent`: a child's local matrix composed into its parent's
    /// space (`child.then(&parent)` maps child-local points to the parent's parent).
    pub fn then(&self, parent: &Matrix34) -> Matrix34 {
        let mut out = [0.0f32; 12];
        for r in 0..3 {
            let a = parent.transform_vector(self.axis(r));
            out[r * 3..r * 3 + 3].copy_from_slice(&a);
        }
        let t = parent.transform_point(self.translation());
        out[9..12].copy_from_slice(&t);
        Matrix34 { m: out }
    }

    /// The determinant of the 3x3 part. Negative means the node mirrors, which flips
    /// the triangles' winding (a renderer must swap its cull mode for such a draw).
    pub fn determinant(&self) -> f32 {
        let m = &self.m;
        m[0] * (m[4] * m[8] - m[5] * m[7]) - m[1] * (m[3] * m[8] - m[5] * m[6])
            + m[2] * (m[3] * m[7] - m[4] * m[6])
    }

    pub fn is_finite(&self) -> bool {
        self.m.iter().all(|v| v.is_finite())
    }
}

impl Default for Matrix34 {
    fn default() -> Self {
        Matrix34::IDENTITY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose() {
        // Child translated by (1,0,0); parent rotates 90 degrees about z (x -> y) and
        // moves by (0,0,5).
        let mut child = Matrix34::IDENTITY;
        child.m[9] = 1.0;
        let parent = Matrix34 {
            m: [0.0, 1.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 5.0],
        };
        let w = child.then(&parent);
        assert_eq!(w.transform_point([0.0, 0.0, 0.0]), [0.0, 1.0, 5.0]);
        assert_eq!(w.transform_point([1.0, 0.0, 0.0]), [0.0, 2.0, 5.0]);
        assert_eq!(Matrix34::IDENTITY.determinant(), 1.0);
    }
}
