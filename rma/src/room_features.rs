//! Room feature visualization and editing UI
//!
//! This module provides 3D visualization and egui editors for RMA room features.

use three_d::{
    Angle, BoundingBox, CpuMaterial, CpuMesh, Gm, InnerSpace, Mat4, Mesh, Object, PhysicalMaterial,
    Radians, Srgba, Vector3, egui, vec2, vec3,
};
use transform_gizmo_egui::GizmoMode;

use crate::debug_lines::{DebugLine, DebugLineMaterial, DebugLines};

use asset_ser::core::object_pool::{ObjectHandle, ObjectPool};

use crate::{
    RMAContext,
    rma::{
        FTransform, FVector, RoomFeature, TypedProperties, UDropPodCalldownLocationFeature,
        UEntranceFeature, UFloodFillBox, UFloodFillLine, UFloodFillPillar, USpawnActorFeature,
    },
};

// Wireframe visualization constants
const CIRCLE_SEGMENTS: usize = 40;
const ELLIPSOID_H_BANDS: usize = 2;
const ELLIPSOID_V_BANDS: usize = 2;
const CONNECTOR_BANDS: usize = 2;

pub type Gizmos<'s> = Vec<(
    enumset::EnumSet<GizmoMode>,
    FTransform,
    Box<dyn FnOnce(FTransform) + 's>,
)>;

impl From<FVector> for Vector3<f32> {
    fn from(val: FVector) -> Self {
        vec3(*val.x, *val.y, *val.z)
    }
}

/// Build 3D visualization for a room feature
/// If `highlight_color` is Some, uses that color instead of the default feature color
pub fn build_feature(
    pool: &ObjectPool,
    handle: ObjectHandle,
    feature: &RoomFeature,
    ctx: &RMAContext,
    highlight_color: Option<Srgba>,
) -> Vec<Box<dyn Object>> {
    let obj = pool.get(handle).expect("Invalid handle");
    let props = obj.properties();

    match feature {
        RoomFeature::FloodFillBox(_) => {
            let typed = UFloodFillBox::from_properties(props).unwrap();
            let position = typed.position();
            let extends = typed.extends();

            let mut mesh = BoundingBox::new(ctx.context, CpuMesh::cube().compute_aabb());
            mesh.set_transformation(
                Mat4::from_translation(position.into())
                    * Mat4::from_nonuniform_scale(*extends.x, *extends.y, *extends.z),
            );

            let material = if let Some(color) = highlight_color {
                PhysicalMaterial::new_opaque(
                    ctx.context,
                    &CpuMaterial {
                        albedo: color,
                        ..Default::default()
                    },
                )
            } else {
                ctx.wireframe_material.clone()
            };
            vec![Box::new(Gm::new(mesh, material))]
        }

        RoomFeature::FloodFillPillar(_) => {
            let typed = UFloodFillPillar::from_properties(props).unwrap();
            let mut lines = Vec::new();

            let points: Vec<FVector> = typed.points().iter().map(|p| p.location()).collect();
            let color = highlight_color.unwrap_or(Srgba::new_opaque(0, 200, 0));

            for pair in points.windows(2) {
                lines.push(DebugLine {
                    start: pair[0].into(),
                    end: pair[1].into(),
                    color,
                });
            }

            let mut debug_lines = DebugLines::new(ctx.context, 2.);
            debug_lines.set_lines(lines);
            vec![Box::new(Gm::new(debug_lines, DebugLineMaterial::new()))]
        }

        RoomFeature::FloodFillLine(_) => {
            let typed = UFloodFillLine::from_properties(props).unwrap();
            let mut lines = Vec::new();
            let color = highlight_color.unwrap_or(Srgba::new_opaque(200, 0, 0));

            let mut add_line = |p1: Vector3<f32>, p2: Vector3<f32>| {
                lines.push(DebugLine {
                    start: p1,
                    end: p2,
                    color,
                });
            };

            // Collect points data
            let points_data: Vec<_> = typed
                .points()
                .iter()
                .map(|p| (p.location(), p.h_range(), p.v_range()))
                .collect();

            // Connector bands between points at each height level
            for pair in points_data.windows(2) {
                let (loc1, h1, v1) = pair[0];
                let (loc2, h2, v2) = pair[1];
                let p1: Vector3<f32> = loc1.into();
                let p2: Vector3<f32> = loc2.into();

                let d = (p1.truncate() - p2.truncate()).normalize();
                let perp = vec2(-d.y, d.x);

                for band in 0..CONNECTOR_BANDS {
                    let t = band as f32 / (CONNECTOR_BANDS - 1) as f32;
                    let z1 = t * v1;
                    let z2 = t * v2;
                    // Horizontal radius shrinks following ellipsoid profile
                    let h_scale = (1.0 - t * t).sqrt();
                    let h1_scaled = h1 * h_scale;
                    let h2_scaled = h2 * h_scale;

                    // Left side connector
                    add_line(
                        p1 + vec3(perp.x * h1_scaled, perp.y * h1_scaled, z1),
                        p2 + vec3(perp.x * h2_scaled, perp.y * h2_scaled, z2),
                    );
                    // Right side connector
                    add_line(
                        p1 + vec3(-perp.x * h1_scaled, -perp.y * h1_scaled, z1),
                        p2 + vec3(-perp.x * h2_scaled, -perp.y * h2_scaled, z2),
                    );
                }
                // Connect the tops of the semi-ellipsoids
                add_line(p1 + vec3(0., 0., v1), p2 + vec3(0., 0., v2));
            }

            // horizontal bands at different heights (semi-ellipsoid, top half only)
            for (location, h_range, v_range) in &points_data {
                let segments = CIRCLE_SEGMENTS;
                for band in 0..ELLIPSOID_H_BANDS {
                    // z offset from 0 to +v_range (top half only)
                    let t = band as f32 / (ELLIPSOID_H_BANDS - 1) as f32;
                    let z_off = t * v_range;
                    // horizontal radius shrinks as we move up (ellipsoid profile)
                    let h_scale = (1.0 - t * t).sqrt();
                    let h_r = h_range * h_scale;

                    if h_r < 1.0 {
                        continue;
                    }

                    let mut iter = (0..segments + 1)
                        .map(|i| {
                            let angle = 2.0 * std::f32::consts::PI * i as f32 / segments as f32;
                            (angle.cos(), angle.sin())
                        })
                        .peekable();
                    while let (Some(a), Some(b)) = (iter.next(), iter.peek()) {
                        add_line(
                            vec3(
                                *location.x + h_r * a.0,
                                *location.y + h_r * a.1,
                                *location.z + z_off,
                            ),
                            vec3(
                                *location.x + h_r * b.0,
                                *location.y + h_r * b.1,
                                *location.z + z_off,
                            ),
                        );
                    }
                }
            }

            // vertical bands at multiple rotation angles
            for (location, h_range, v_range) in &points_data {
                let segments = CIRCLE_SEGMENTS;
                for rot in 0..ELLIPSOID_V_BANDS {
                    let rot_angle = std::f32::consts::PI * rot as f32 / ELLIPSOID_V_BANDS as f32;
                    let (rot_cos, rot_sin) = (rot_angle.cos(), rot_angle.sin());

                    let mut iter = (0..segments / 2 + 1)
                        .map(|i| {
                            let angle = 2.0 * std::f32::consts::PI * i as f32 / segments as f32;
                            (angle.cos(), angle.sin())
                        })
                        .peekable();
                    while let (Some(a), Some(b)) = (iter.next(), iter.peek()) {
                        // Rotate the vertical circle around the Z axis
                        add_line(
                            vec3(
                                *location.x + h_range * a.0 * rot_cos,
                                *location.y + h_range * a.0 * rot_sin,
                                *location.z + v_range * a.1,
                            ),
                            vec3(
                                *location.x + h_range * b.0 * rot_cos,
                                *location.y + h_range * b.0 * rot_sin,
                                *location.z + v_range * b.1,
                            ),
                        );
                    }
                }
            }

            let mut debug_lines = DebugLines::new(ctx.context, 2.);
            debug_lines.set_lines(lines);
            vec![Box::new(Gm::new(debug_lines, DebugLineMaterial::new()))]
        }

        RoomFeature::EntranceFeature(_) => {
            let typed = UEntranceFeature::from_properties(props).unwrap();
            let location = typed.location();
            let entrance_type = typed.entrance_type();

            let albedo = highlight_color.unwrap_or(match entrance_type {
                "ECaveEntranceType::EntranceAndExit" => Srgba {
                    r: 0,
                    g: 255,
                    b: 255,
                    a: 200,
                },
                "ECaveEntranceType::Entrance" => Srgba {
                    r: 255,
                    g: 100,
                    b: 0,
                    a: 200,
                },
                "ECaveEntranceType::Exit" => Srgba {
                    r: 255,
                    g: 0,
                    b: 100,
                    a: 200,
                },
                "ECaveEntranceType::TreassureRoom" => Srgba {
                    r: 255,
                    g: 200,
                    b: 0,
                    a: 200,
                },
                _ => Srgba {
                    r: 128,
                    g: 128,
                    b: 128,
                    a: 200,
                },
            });

            let mut sphere = Gm::new(
                Mesh::new(ctx.context, &CpuMesh::sphere(16)),
                PhysicalMaterial::new_opaque(
                    ctx.context,
                    &CpuMaterial {
                        albedo,
                        ..Default::default()
                    },
                ),
            );
            sphere.set_transformation(
                Mat4::from_translation(location.into()) * Mat4::from_scale(100.0),
            );
            vec![Box::new(sphere)]
        }

        RoomFeature::SpawnActorFeature(_) => {
            let typed = USpawnActorFeature::from_properties(props).unwrap();
            let location = typed.location();

            let albedo = highlight_color.unwrap_or(Srgba {
                r: 255,
                g: 200,
                b: 0,
                a: 200,
            });

            let mut obj = Gm::new(
                Mesh::new(ctx.context, &CpuMesh::cone(16)),
                PhysicalMaterial::new_opaque(
                    ctx.context,
                    &CpuMaterial {
                        albedo,
                        ..Default::default()
                    },
                ),
            );
            obj.set_transformation(
                Mat4::from_translation(location.into())
                    * Mat4::from_nonuniform_scale(100.0, 100.0, 300.0)
                    * Mat4::from_angle_y(-Radians::turn_div_4()),
            );
            vec![Box::new(obj)]
        }

        RoomFeature::DropPodCalldownLocationFeature(_) => {
            let typed = UDropPodCalldownLocationFeature::from_properties(props).unwrap();
            let location = typed.location();

            let albedo = highlight_color.unwrap_or(Srgba {
                r: 0,
                g: 255,
                b: 0,
                a: 200,
            });

            let mut obj = Gm::new(
                Mesh::new(ctx.context, &CpuMesh::cylinder(16)),
                PhysicalMaterial::new_opaque(
                    ctx.context,
                    &CpuMaterial {
                        albedo,
                        ..Default::default()
                    },
                ),
            );
            obj.set_transformation(
                Mat4::from_translation(location.into())
                    * Mat4::from_nonuniform_scale(100.0, 100.0, 300.0)
                    * Mat4::from_angle_y(Radians::turn_div_4()),
            );
            vec![Box::new(obj)]
        }

        _ => Vec::new(),
    }
}

/// Build editor UI for a room feature
pub fn edit_feature<'s>(
    pool: &'s mut ObjectPool,
    handle: ObjectHandle,
    feature: &RoomFeature,
    ui: &mut egui::Ui,
    _gizmos: &mut Gizmos<'s>,
) -> bool {
    // For now, just show the feature name
    // Full editing support requires more work to handle mutable property access
    ui.label(format!("Feature: {}", feature.name()));

    let obj = pool.get(handle).expect("Invalid handle");
    let props = obj.properties();

    match feature {
        RoomFeature::FloodFillBox(_) => {
            let typed = UFloodFillBox::from_properties(props).unwrap();
            ui.label(format!("Position: {:?}", typed.position()));
            ui.label(format!("Extends: {:?}", typed.extends()));
            false
        }

        RoomFeature::FloodFillPillar(_) => {
            let typed = UFloodFillPillar::from_properties(props).unwrap();
            ui.label(format!("Points: {}", typed.points().len()));
            false
        }

        RoomFeature::FloodFillLine(_) => {
            let typed = UFloodFillLine::from_properties(props).unwrap();
            ui.label(format!("Points: {}", typed.points().len()));
            false
        }

        RoomFeature::EntranceFeature(_) => {
            let typed = UEntranceFeature::from_properties(props).unwrap();
            ui.label(format!("Location: {:?}", typed.location()));
            ui.label(format!("Type: {}", typed.entrance_type()));
            false
        }

        RoomFeature::SpawnActorFeature(_) => {
            let typed = USpawnActorFeature::from_properties(props).unwrap();
            ui.label(format!("Location: {:?}", typed.location()));
            false
        }

        RoomFeature::DropPodCalldownLocationFeature(_) => {
            let typed = UDropPodCalldownLocationFeature::from_properties(props).unwrap();
            ui.label(format!("Location: {:?}", typed.location()));
            false
        }

        _ => {
            ui.label("(no editor available)");
            false
        }
    }
}

/// Axis-aligned bounding box with min/max corners
#[derive(Debug, Clone, Copy)]
pub struct Aabb {
    pub min: Vector3<f32>,
    pub max: Vector3<f32>,
}

impl Aabb {
    pub fn new() -> Self {
        Self {
            min: vec3(f32::MAX, f32::MAX, f32::MAX),
            max: vec3(f32::MIN, f32::MIN, f32::MIN),
        }
    }

    pub fn expand_point(&mut self, p: Vector3<f32>) {
        self.min.x = self.min.x.min(p.x);
        self.min.y = self.min.y.min(p.y);
        self.min.z = self.min.z.min(p.z);
        self.max.x = self.max.x.max(p.x);
        self.max.y = self.max.y.max(p.y);
        self.max.z = self.max.z.max(p.z);
    }

    pub fn is_valid(&self) -> bool {
        self.min.x <= self.max.x && self.min.y <= self.max.y && self.min.z <= self.max.z
    }

    pub fn size(&self) -> Vector3<f32> {
        self.max - self.min
    }

    pub fn center(&self) -> Vector3<f32> {
        (self.min + self.max) * 0.5
    }

    /// Pad the bounding box by a percentage on each side
    pub fn padded(&self, percent: f32) -> Self {
        let size = self.size();
        let padding = size * percent;
        Self {
            min: self.min - padding,
            max: self.max + padding,
        }
    }
}

impl Default for Aabb {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute the bounding box of all features in the room
pub fn compute_room_bounds(pool: &ObjectPool, root_handle: ObjectHandle) -> Aabb {
    let mut aabb = Aabb::new();
    let features = crate::rma::load_room_features(pool, root_handle);

    fn process_features(pool: &ObjectPool, features: &[RoomFeature], aabb: &mut Aabb) {
        for feature in features {
            let obj = pool.get(feature.handle()).expect("Invalid handle");
            let props = obj.properties();

            match feature {
                RoomFeature::FloodFillBox(_) => {
                    let typed = UFloodFillBox::from_properties(props).unwrap();
                    let pos: Vector3<f32> = typed.position().into();
                    let ext: Vector3<f32> = typed.extends().into();
                    // Box corners
                    aabb.expand_point(pos - ext);
                    aabb.expand_point(pos + ext);
                }
                RoomFeature::FloodFillPillar(_) => {
                    let typed = UFloodFillPillar::from_properties(props).unwrap();
                    for point in typed.points().iter() {
                        let loc: Vector3<f32> = point.location().into();
                        let range = point.range();
                        let r = range.max().max(range.min());
                        aabb.expand_point(loc - vec3(r, r, r));
                        aabb.expand_point(loc + vec3(r, r, r));
                    }
                }
                RoomFeature::FloodFillLine(_) => {
                    let typed = UFloodFillLine::from_properties(props).unwrap();
                    for point in typed.points().iter() {
                        let loc: Vector3<f32> = point.location().into();
                        let h = point.h_range();
                        let v = point.v_range();
                        aabb.expand_point(loc - vec3(h, h, v));
                        aabb.expand_point(loc + vec3(h, h, v));
                    }
                }
                RoomFeature::EntranceFeature(_) => {
                    let typed = UEntranceFeature::from_properties(props).unwrap();
                    let loc: Vector3<f32> = typed.location().into();
                    aabb.expand_point(loc);
                }
                RoomFeature::SpawnActorFeature(_) => {
                    let typed = USpawnActorFeature::from_properties(props).unwrap();
                    let loc: Vector3<f32> = typed.location().into();
                    aabb.expand_point(loc);
                }
                RoomFeature::DropPodCalldownLocationFeature(_) => {
                    let typed = UDropPodCalldownLocationFeature::from_properties(props).unwrap();
                    let loc: Vector3<f32> = typed.location().into();
                    aabb.expand_point(loc);
                }
                _ => {}
            }

            // Process children recursively
            let children = feature.get_child_features(pool);
            process_features(pool, &children, aabb);
        }
    }

    process_features(pool, &features, &mut aabb);
    aabb
}

/// Build grid planes for the room based on bounding box
/// Creates grid lines on XY, XZ, and YZ planes, each plane is square (max dimension), centered on room
pub fn build_grid_planes(ctx: &RMAContext, bounds: &Aabb) -> Vec<Box<dyn Object>> {
    if !bounds.is_valid() {
        return Vec::new();
    }

    let padded = bounds.padded(0.2); // 20% padding
    let size = padded.size();
    let center = padded.center();

    // Use the maximum dimension so each plane is the same size (square)
    let max_size = size.x.max(size.y).max(size.z);
    let half = max_size / 2.0;
    let grid_spacing = max_size / 10.0;

    // Grid color - subtle gray
    let grid_color = Srgba {
        r: 80,
        g: 80,
        b: 80,
        a: 255,
    };

    let mut lines = Vec::new();
    let mut add_line = |start: Vector3<f32>, end: Vector3<f32>| {
        lines.push(DebugLine {
            start,
            end,
            color: grid_color,
        });
    };

    // Cube bounds centered on the room
    let min_x = center.x - half;
    let min_y = center.y - half;
    let min_z = center.z - half;

    // XY plane (at min Z) - square grid centered
    let z = min_z;
    let mut t = 0.0;
    while t <= max_size {
        add_line(
            vec3(min_x + t, min_y, z),
            vec3(min_x + t, min_y + max_size, z),
        );
        add_line(
            vec3(min_x, min_y + t, z),
            vec3(min_x + max_size, min_y + t, z),
        );
        t += grid_spacing;
    }

    // XZ plane (at min Y) - square grid centered
    let y = min_y;
    let mut t = 0.0;
    while t <= max_size {
        add_line(
            vec3(min_x + t, y, min_z),
            vec3(min_x + t, y, min_z + max_size),
        );
        add_line(
            vec3(min_x, y, min_z + t),
            vec3(min_x + max_size, y, min_z + t),
        );
        t += grid_spacing;
    }

    // YZ plane (at min X) - square grid centered
    let x = min_x;
    let mut t = 0.0;
    while t <= max_size {
        add_line(
            vec3(x, min_y + t, min_z),
            vec3(x, min_y + t, min_z + max_size),
        );
        add_line(
            vec3(x, min_y, min_z + t),
            vec3(x, min_y + max_size, min_z + t),
        );
        t += grid_spacing;
    }

    let mut debug_lines = DebugLines::new(ctx.context, 2.5);
    debug_lines.set_lines(lines);

    vec![Box::new(Gm::new(debug_lines, DebugLineMaterial::new()))]
}
