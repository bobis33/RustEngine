/// Simple camera with position, target, and projection parameters.
/// Produces view and projection matrices in Vulkan's coordinate system
/// (Y-down, depth [0, 1]).
pub struct Camera {
    /// Camera position in world space
    pub position: [f32; 3],
    /// Point the camera looks at
    pub target:   [f32; 3],
    /// Up vector (usually [0, -1, 0] in Vulkan — Y points down)
    pub up:       [f32; 3],
    /// Vertical field of view in radians
    pub fov_y:    f32,
    /// Near clipping plane
    pub near:     f32,
    /// Far clipping plane
    pub far:      f32,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            position: [0.0, -1.0, -3.0], // behind and slightly above the origin
            target:   [0.0,  0.0,  0.0],
            up:       [0.0, -1.0,  0.0], // Y-up in Vulkan space = [0, -1, 0]
            fov_y:    std::f32::consts::FRAC_PI_4, // 45°
            near:     0.1,
            far:      100.0,
        }
    }

    /// Build the view matrix (world → camera space).
    /// Equivalent to glm::lookAt.
    pub fn view(&self) -> [[f32; 4]; 4] {
        let eye = self.position;
        let ctr = self.target;
        let up  = self.up;

        // Forward vector (from eye to target), normalized
        let f = normalize(sub(ctr, eye));
        // Right vector
        let r = normalize(cross(f, normalize(up)));
        // True up (recomputed to be orthogonal)
        let u = cross(r, f);

        [
            [ r[0],  u[0], -f[0], 0.0],
            [ r[1],  u[1], -f[1], 0.0],
            [ r[2],  u[2], -f[2], 0.0],
            [-dot(r, eye), -dot(u, eye), dot(f, eye), 1.0],
        ]
    }

    /// Build the perspective projection matrix for Vulkan.
    /// - Y axis is flipped (Vulkan Y-down)
    /// - Depth range [0, 1]
    pub fn projection(&self, aspect: f32) -> [[f32; 4]; 4] {
        let f    = 1.0 / (self.fov_y / 2.0).tan();
        let near = self.near;
        let far  = self.far;

        [
            [f / aspect, 0.0,  0.0,                          0.0],
            [0.0,       -f,    0.0,                          0.0], // flip Y
            [0.0,        0.0,  far / (near - far),          -1.0],
            [0.0,        0.0, (near * far) / (near - far),   0.0],
        ]
    }
}

impl Default for Camera {
    fn default() -> Self { Self::new() }
}

// ---------------------------------------------------------------------------
// Vec3 helpers (avoiding a math crate dependency for now)
// ---------------------------------------------------------------------------

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0]-b[0], a[1]-b[1], a[2]-b[2]]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0]*b[0] + a[1]*b[1] + a[2]*b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1]*b[2] - a[2]*b[1],
        a[2]*b[0] - a[0]*b[2],
        a[0]*b[1] - a[1]*b[0],
    ]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0]*v[0] + v[1]*v[1] + v[2]*v[2]).sqrt();
    [v[0]/len, v[1]/len, v[2]/len]
}