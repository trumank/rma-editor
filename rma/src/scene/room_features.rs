//! Room feature visualization and editing UI
//!
//! This module provides 3D visualization and egui editors for RMA room features.

use three_d::{
    Angle, CpuMaterial, CpuMesh, Gm, InnerSpace, Mat4, Mesh, Object, PhysicalMaterial, Radians,
    Srgba, Vector3, egui, radians, vec2, vec3,
};
use transform_gizmo_egui::GizmoMode;

use super::debug_lines::{DebugLine, DebugLineMaterial, DebugLines};
use crate::RMAContext;
use crate::objects::*;

// Wireframe visualization constants
const CIRCLE_SEGMENTS: usize = 40;
const PILLAR_SEGMENTS: usize = 4;
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
        vec3(val.x, val.y, val.z)
    }
}

impl From<&FVector> for Vector3<f32> {
    fn from(val: &FVector) -> Self {
        vec3(val.x, val.y, val.z)
    }
}

/// Build 3D visualization for a room feature
pub fn build_feature(
    feature: &URoomFeature,
    ctx: &RMAContext,
    highlight_color: Option<Srgba>,
) -> Vec<Box<dyn Object>> {
    match &feature.feature_type {
        URoomFeatureType::FloodFillBox(f) => {
            let position: Vector3<f32> = (&f.position).into();
            let ext = &f.extends;
            let color = highlight_color.unwrap_or(Srgba::new_opaque(200, 0, 0));

            // Build rotation matrix from FRotator (pitch, yaw, roll in degrees)
            // Unreal uses left-handed coords, negate pitch to match
            let roll = radians(f.rotation.roll.to_radians());
            let pitch = radians((-f.rotation.pitch).to_radians());
            let yaw = radians(f.rotation.yaw.to_radians());
            let rot =
                Mat4::from_angle_z(yaw) * Mat4::from_angle_y(pitch) * Mat4::from_angle_x(roll);

            // 8 corners of the box in local space
            let corners_local = [
                vec3(-ext.x, -ext.y, -ext.z),
                vec3(ext.x, -ext.y, -ext.z),
                vec3(ext.x, ext.y, -ext.z),
                vec3(-ext.x, ext.y, -ext.z),
                vec3(-ext.x, -ext.y, ext.z),
                vec3(ext.x, -ext.y, ext.z),
                vec3(ext.x, ext.y, ext.z),
                vec3(-ext.x, ext.y, ext.z),
            ];

            // Transform corners to world space
            let corners: Vec<Vector3<f32>> = corners_local
                .iter()
                .map(|c| {
                    let rotated = rot * c.extend(1.0);
                    vec3(rotated.x, rotated.y, rotated.z) + position
                })
                .collect();

            // 12 edges of the box
            let edges = [
                // Bottom face
                (0, 1),
                (1, 2),
                (2, 3),
                (3, 0),
                // Top face
                (4, 5),
                (5, 6),
                (6, 7),
                (7, 4),
                // Vertical edges
                (0, 4),
                (1, 5),
                (2, 6),
                (3, 7),
            ];

            let lines: Vec<DebugLine> = edges
                .iter()
                .map(|&(a, b)| DebugLine {
                    start: corners[a],
                    end: corners[b],
                    color,
                })
                .collect();

            let mut debug_lines = DebugLines::new(ctx.context, 2.);
            debug_lines.set_lines(lines);
            vec![Box::new(Gm::new(debug_lines, DebugLineMaterial::new()))]
        }

        URoomFeatureType::FloodFillPillar(f) => {
            let mut lines = Vec::new();
            let color = highlight_color.unwrap_or(Srgba::new_opaque(0, 200, 0));

            let mut add_line = |p1: Vector3<f32>, p2: Vector3<f32>| {
                lines.push(DebugLine {
                    start: p1,
                    end: p2,
                    color,
                });
            };

            let range_scale = (f.range_scale.min + f.range_scale.max) / 2.;

            // Collect points data (location and max range as radius)
            let points_data: Vec<_> = f
                .points
                .iter()
                .map(|p| {
                    (
                        Vector3::from(&p.location),
                        p.range.max.max(p.range.min) * range_scale,
                    )
                })
                .collect();

            // Connector bands between points
            for pair in points_data.windows(2) {
                let (p1, r1) = pair[0];
                let (p2, r2) = pair[1];

                // Direction from p1 to p2
                let dir = p2 - p1;
                let len = dir.magnitude();
                if len < 0.001 {
                    continue;
                }
                let dir_norm = dir / len;

                // Build a perpendicular basis
                let up = if dir_norm.z.abs() < 0.9 {
                    vec3(0., 0., 1.)
                } else {
                    vec3(1., 0., 0.)
                };
                let perp1 = dir_norm.cross(up).normalize();
                let perp2 = dir_norm.cross(perp1).normalize();

                // Draw connector lines around the circumference
                let segments = PILLAR_SEGMENTS;
                for i in 0..segments {
                    let angle = 2.0 * std::f32::consts::PI * i as f32 / segments as f32;
                    let (cos_a, sin_a) = (angle.cos(), angle.sin());
                    let offset1 = perp1 * cos_a + perp2 * sin_a;

                    add_line(p1 + offset1 * r1, p2 + offset1 * r2);
                }
            }

            // Helper to draw a circle given center, radius, and two perpendicular axes
            let mut draw_circle =
                |center: Vector3<f32>, radius: f32, axis1: Vector3<f32>, axis2: Vector3<f32>| {
                    let segments = CIRCLE_SEGMENTS;
                    let mut iter = (0..segments + 1)
                        .map(|j| {
                            let angle = 2.0 * std::f32::consts::PI * j as f32 / segments as f32;
                            (angle.cos(), angle.sin())
                        })
                        .peekable();
                    while let (Some(a), Some(b)) = (iter.next(), iter.peek()) {
                        add_line(
                            center + (axis1 * a.0 + axis2 * a.1) * radius,
                            center + (axis1 * b.0 + axis2 * b.1) * radius,
                        );
                    }
                };

            let perp_basis = |dir: Vector3<f32>| -> (Vector3<f32>, Vector3<f32>) {
                let up = if dir.z.abs() < 0.9 {
                    vec3(0., 0., 1.)
                } else {
                    vec3(1., 0., 0.)
                };
                let p1 = dir.cross(up).normalize();
                let p2 = dir.cross(p1).normalize();
                (p1, p2)
            };

            // Each segment draws end cap circles at both ends
            for pair in points_data.windows(2) {
                let (p1, r1) = pair[0];
                let (p2, r2) = pair[1];
                let dir = (p2 - p1).normalize();
                let (perp1, perp2) = perp_basis(dir);
                draw_circle(p1, r1, perp1, perp2);
                draw_circle(p2, r2, perp1, perp2);
            }

            // Each point draws additional circles
            for (i, (loc, r)) in points_data.iter().enumerate() {
                let is_endpoint = points_data.len() == 1 || i == 0 || i == points_data.len() - 1;

                let dir = if points_data.len() == 1 {
                    vec3(0., 0., 1.)
                } else if i == 0 {
                    (points_data[1].0 - *loc).normalize()
                } else {
                    (*loc - points_data[i - 1].0).normalize()
                };

                let (perp1, perp2) = perp_basis(dir);

                if is_endpoint {
                    // Endpoints: two circles through the axis
                    draw_circle(*loc, *r, dir, perp1);
                    draw_circle(*loc, *r, dir, perp2);
                } else {
                    // Joint: one circle in the plane formed by both segments
                    let d2 = (points_data[i + 1].0 - *loc).normalize();
                    let normal = dir.cross(d2);
                    if normal.magnitude() > 0.001 {
                        draw_circle(*loc, *r, dir, normal.cross(dir).normalize());
                    } else {
                        draw_circle(*loc, *r, dir, perp1);
                    }
                }
            }

            let mut debug_lines = DebugLines::new(ctx.context, 2.);
            debug_lines.set_lines(lines);
            vec![Box::new(Gm::new(debug_lines, DebugLineMaterial::new()))]
        }

        URoomFeatureType::FloodFillLine(f) => {
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
            let points_data: Vec<_> = f
                .points
                .iter()
                .map(|p| (Vector3::from(&p.location), p.h_range, p.v_range))
                .collect();

            // Connector bands between points at each height level
            for pair in points_data.windows(2) {
                let (p1, h1, v1) = pair[0];
                let (p2, h2, v2) = pair[1];

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
                                location.x + h_r * a.0,
                                location.y + h_r * a.1,
                                location.z + z_off,
                            ),
                            vec3(
                                location.x + h_r * b.0,
                                location.y + h_r * b.1,
                                location.z + z_off,
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
                                location.x + h_range * a.0 * rot_cos,
                                location.y + h_range * a.0 * rot_sin,
                                location.z + v_range * a.1,
                            ),
                            vec3(
                                location.x + h_range * b.0 * rot_cos,
                                location.y + h_range * b.0 * rot_sin,
                                location.z + v_range * b.1,
                            ),
                        );
                    }
                }
            }

            let mut debug_lines = DebugLines::new(ctx.context, 2.);
            debug_lines.set_lines(lines);
            vec![Box::new(Gm::new(debug_lines, DebugLineMaterial::new()))]
        }

        URoomFeatureType::Entrance(f) => {
            let location: Vector3<f32> = (&f.location).into();

            let albedo = highlight_color.unwrap_or(match f.entrance_type {
                ECaveEntranceType::EntranceAndExit => Srgba {
                    r: 0,
                    g: 255,
                    b: 255,
                    a: 200,
                },
                ECaveEntranceType::Entrance => Srgba {
                    r: 255,
                    g: 100,
                    b: 0,
                    a: 200,
                },
                ECaveEntranceType::Exit => Srgba {
                    r: 255,
                    g: 0,
                    b: 100,
                    a: 200,
                },
                ECaveEntranceType::TreassureRoom => Srgba {
                    r: 255,
                    g: 200,
                    b: 0,
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
            sphere.set_transformation(Mat4::from_translation(location) * Mat4::from_scale(100.0));
            vec![Box::new(sphere)]
        }

        URoomFeatureType::SpawnActor(f) => {
            let location: Vector3<f32> = (&f.location).into();

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
                Mat4::from_translation(location)
                    * Mat4::from_nonuniform_scale(100.0, 100.0, 300.0)
                    * Mat4::from_angle_y(-Radians::turn_div_4()),
            );
            vec![Box::new(obj)]
        }

        URoomFeatureType::DropPodCalldownLocation(f) => {
            let location: Vector3<f32> = (&f.location).into();

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
                Mat4::from_translation(location)
                    * Mat4::from_nonuniform_scale(100.0, 100.0, 300.0)
                    * Mat4::from_angle_y(Radians::turn_div_4()),
            );
            vec![Box::new(obj)]
        }

        _ => Vec::new(),
    }
}

/// Check if a feature type is a mesh primitive (carves terrain)
pub fn is_mesh_primitive(feature: &URoomFeatureType) -> bool {
    matches!(
        feature,
        URoomFeatureType::FloodFillBox(_)
            | URoomFeatureType::FloodFillLine(_)
            | URoomFeatureType::FloodFillPillar(_)
            | URoomFeatureType::FloodFillProceduralPillar(_)
    )
}

/// Get the display name for a feature type
pub fn feature_type_name(feature: &URoomFeatureType) -> &'static str {
    match feature {
        URoomFeatureType::FloodFillBox(_) => "FloodFillBox",
        URoomFeatureType::FloodFillLine(_) => "FloodFillLine",
        URoomFeatureType::FloodFillPillar(_) => "FloodFillPillar",
        URoomFeatureType::FloodFillProceduralPillar(_) => "FloodFillProceduralPillar",
        URoomFeatureType::Entrance(_) => "Entrance",
        URoomFeatureType::RandomSelector(_) => "RandomSelector",
        URoomFeatureType::RandomSubRoom(_) => "RandomSubRoom",
        URoomFeatureType::SubRoom(_) => "SubRoom",
        URoomFeatureType::SpawnActor(_) => "SpawnActor",
        URoomFeatureType::SpawnTrigger(_) => "SpawnTrigger",
        URoomFeatureType::Resource(_) => "Resource",
        URoomFeatureType::DropPodCalldownLocation(_) => "DropPodCalldownLocation",
    }
}

/// Build editor UI for a room feature
pub fn edit_feature<'s>(
    feature: &mut URoomFeature,
    ui: &mut egui::Ui,
    _gizmos: &mut Gizmos<'s>,
) -> bool {
    use super::property_editors::*;

    ui.label(format!(
        "Feature: {}",
        feature_type_name(&feature.feature_type)
    ));
    ui.separator();

    let mut changed = false;

    match &mut feature.feature_type {
        URoomFeatureType::FloodFillBox(f) => {
            changed |= edit_fvector(ui, "Position", &mut f.position);
            changed |= edit_fvector(ui, "Extends", &mut f.extends);
            changed |= edit_frotator(ui, "Rotation", &mut f.rotation);
            changed |= edit_bool(ui, "Is Carver", &mut f.is_carver);
            changed |= edit_f32(ui, "Noise Range", &mut f.noise_range);
        }

        URoomFeatureType::FloodFillPillar(f) => {
            changed |= edit_frand_range(ui, "Range Scale", &mut f.range_scale);
            changed |= edit_frand_range(ui, "Noise Range Scale", &mut f.noise_range_scale);
            changed |= edit_frand_range(ui, "Endcap Scale", &mut f.endcap_scale);
            changed |= edit_vec(
                ui,
                "Points",
                &mut f.points,
                || FRandLinePoint {
                    location: FVector::default(),
                    range: FRandRange {
                        min: 100.0,
                        max: 200.0,
                    },
                    noise_range: FRandRange {
                        min: 0.0,
                        max: 50.0,
                    },
                    skew_factor: FRandRange { min: 0.0, max: 0.0 },
                    fill_amount: FRandRange { min: 1.0, max: 1.0 },
                },
                edit_rand_line_point,
            );
        }

        URoomFeatureType::FloodFillLine(f) => {
            changed |= edit_bool(ui, "Use Detail Noise", &mut f.use_detail_noise);
            changed |= edit_vec(
                ui,
                "Points",
                &mut f.points,
                || FRoomLinePoint {
                    location: FVector::default(),
                    h_range: 200.0,
                    v_range: 200.0,
                    cieling_noise_range: 50.0,
                    wall_noise_range: 50.0,
                    floor_noise_range: 50.0,
                    cieling_height: 100.0,
                    height_scale: 1.0,
                    floor_depth: 0.0,
                    floor_angle: 0.0,
                },
                edit_room_line_point,
            );
        }

        URoomFeatureType::FloodFillProceduralPillar(f) => {
            changed |= edit_vec(
                ui,
                "Points",
                &mut f.points,
                FVector::default,
                edit_fvector_point,
            );
        }

        URoomFeatureType::Entrance(f) => {
            changed |= edit_fvector(ui, "Location", &mut f.location);
            changed |= edit_frotator(ui, "Direction", &mut f.direction);
            changed |= edit_enum(ui, "Entrance Type", &mut f.entrance_type);
            changed |= edit_enum(ui, "Priority", &mut f.priority);
        }

        URoomFeatureType::RandomSelector(f) => {
            changed |= edit_i32(ui, "Min", &mut f.min);
            changed |= edit_i32(ui, "Max", &mut f.max);
        }

        URoomFeatureType::RandomSubRoom(f) => {
            changed |= edit_fvector(ui, "Location", &mut f.location);
            changed |= edit_frotator(ui, "Rotation", &mut f.rotation);
            changed |= edit_f32(ui, "Scale", &mut f.scale);
            changed |= edit_i32(ui, "Layer", &mut f.layer);
        }

        URoomFeatureType::SubRoom(f) => {
            changed |= edit_fvector(ui, "Location", &mut f.location);
            changed |= edit_frotator(ui, "Rotation", &mut f.rotation);
            changed |= edit_f32(ui, "Scale", &mut f.scale);
        }

        URoomFeatureType::SpawnActor(f) => {
            changed |= edit_fvector(ui, "Location", &mut f.location);
            changed |= edit_fvector(ui, "Adjustment Direction", &mut f.adjustment_direction);
            changed |= edit_enum(ui, "Adjustment", &mut f.adjustment);
            changed |= edit_fvector(ui, "Scale Min", &mut f.scale_min);
            changed |= edit_fvector(ui, "Scale Max", &mut f.scale_max);
            changed |= edit_frotator(ui, "Rotation Delta", &mut f.rotation_delta);
        }

        URoomFeatureType::SpawnTrigger(f) => {
            changed |= edit_string(ui, "Message", &mut f.message);
            changed |= edit_fvector(ui, "Translation", &mut f.transform.translation);
            let mut rotator: FRotator = f.transform.rotation.into();
            if edit_frotator(ui, "Rotation", &mut rotator) {
                f.transform.rotation = rotator.into();
                changed = true;
            }
            changed |= edit_fvector(ui, "Scale", &mut f.transform.Scale3D);
        }

        URoomFeatureType::Resource(f) => {
            changed |= edit_fvector(ui, "Location", &mut f.location);
            changed |= edit_f32(ui, "Base Amount", &mut f.base_amount);
        }

        URoomFeatureType::DropPodCalldownLocation(f) => {
            changed |= edit_fvector(ui, "Location", &mut f.location);
        }
    }

    changed
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
pub fn compute_room_bounds(room: &URoomGenerator) -> Aabb {
    let mut aabb = Aabb::new();

    fn process_features(features: &[URoomFeature], aabb: &mut Aabb) {
        for feature in features {
            match &feature.feature_type {
                URoomFeatureType::FloodFillBox(f) => {
                    let pos: Vector3<f32> = (&f.position).into();
                    let ext: Vector3<f32> = (&f.extends).into();
                    aabb.expand_point(pos - ext);
                    aabb.expand_point(pos + ext);
                }
                URoomFeatureType::FloodFillPillar(f) => {
                    for point in &f.points {
                        let loc: Vector3<f32> = (&point.location).into();
                        let r = point.range.max.max(point.range.min);
                        aabb.expand_point(loc - vec3(r, r, r));
                        aabb.expand_point(loc + vec3(r, r, r));
                    }
                }
                URoomFeatureType::FloodFillLine(f) => {
                    for point in &f.points {
                        let loc: Vector3<f32> = (&point.location).into();
                        let h = point.h_range;
                        let v = point.v_range;
                        aabb.expand_point(loc - vec3(h, h, v));
                        aabb.expand_point(loc + vec3(h, h, v));
                    }
                }
                URoomFeatureType::Entrance(f) => {
                    let loc: Vector3<f32> = (&f.location).into();
                    aabb.expand_point(loc);
                }
                URoomFeatureType::SpawnActor(f) => {
                    let loc: Vector3<f32> = (&f.location).into();
                    aabb.expand_point(loc);
                }
                URoomFeatureType::DropPodCalldownLocation(f) => {
                    let loc: Vector3<f32> = (&f.location).into();
                    aabb.expand_point(loc);
                }
                _ => {}
            }

            // Process children recursively
            process_features(&feature.children, aabb);
        }
    }

    process_features(&room.room_features, &mut aabb);
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
