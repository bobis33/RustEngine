use super::pipeline::Vertex;

/// 8 sommets du cube, chaque face a une couleur différente.
#[rustfmt::skip]
pub const VERTICES: &[Vertex] = &[
    Vertex { position: [-0.5, -0.5, -0.5], color: [1.0, 0.0, 0.0] }, // 0
    Vertex { position: [ 0.5, -0.5, -0.5], color: [0.0, 1.0, 0.0] }, // 1
    Vertex { position: [ 0.5,  0.5, -0.5], color: [0.0, 0.0, 1.0] }, // 2
    Vertex { position: [-0.5,  0.5, -0.5], color: [1.0, 1.0, 0.0] }, // 3
    Vertex { position: [-0.5, -0.5,  0.5], color: [1.0, 0.0, 1.0] }, // 4
    Vertex { position: [ 0.5, -0.5,  0.5], color: [0.0, 1.0, 1.0] }, // 5
    Vertex { position: [ 0.5,  0.5,  0.5], color: [1.0, 1.0, 1.0] }, // 6
    Vertex { position: [-0.5,  0.5,  0.5], color: [0.5, 0.5, 0.5] }, // 7
];

/// Winding CCW vu de l'extérieur de chaque face.
/// Avec cull_mode = BACK et front_face = COUNTER_CLOCKWISE dans la pipeline.
///
/// Rappel des axes : X droite, Y bas (Vulkan), Z vers l'écran
///
/// ```text
///      3----2
///     /|   /|
///    7----6 |
///    | 0--|-1
///    |/   |/
///    4----5
/// ```

#[rustfmt::skip]
pub const INDICES: &[u16] = &[
    // face arrière (-Z) — vue de -Z, normale pointe vers -Z
    0, 3, 2,  2, 1, 0,
    // face avant  (+Z) — vue de +Z, normale pointe vers +Z
    4, 5, 6,  6, 7, 4,
    // face gauche (-X) — vue de -X, normale pointe vers -X
    0, 4, 7,  7, 3, 0,
    // face droite (+X) — vue de +X, normale pointe vers +X
    1, 2, 6,  6, 5, 1,
    // face bas    (-Y) — vue de -Y (haut en Vulkan), normale pointe vers -Y
    0, 1, 5,  5, 4, 0,
    // face haut   (+Y) — vue de +Y (bas en Vulkan),  normale pointe vers +Y
    3, 7, 6,  6, 2, 3,
];