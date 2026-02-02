//! CSG mesh generation from room features
//!
//! This module converts mesh primitives (FloodFillLine, etc.) into actual CSG meshes
//! using the csgrs crate for proper cave visualization.

use std::collections::HashMap;

use csgrs::csg::CSG;
use csgrs::float_types::parry3d::na::Point3;
use three_d::{CpuMesh, Indices, Positions, Vector3, vec3};

use super::room_features::Aabb;
use crate::objects::{
    FRandLinePoint, FRoomLinePoint, UFloodFillLine, UFloodFillPillar, URoomFeature,
    URoomFeatureType, URoomGenerator,
};

/// Trait for checking feature visibility
pub trait VisibilityCheck {
    fn is_visible(&self, path: &[usize]) -> bool;
}

impl<T> VisibilityCheck for HashMap<Vec<usize>, T>
where
    T: HasVisible,
{
    fn is_visible(&self, path: &[usize]) -> bool {
        self.get(path).map(|s| s.visible()).unwrap_or(true)
    }
}

/// Trait for types that have a visible field
pub trait HasVisible {
    fn visible(&self) -> bool;
}

/// SDF for a FloodFillLine - creates an ellipsoid tunnel along a polyline
fn flood_fill_line_sdf(line: &UFloodFillLine) -> impl Fn(&Point3<f64>) -> f64 + '_ {
    move |p: &Point3<f64>| {
        if line.points.is_empty() {
            return f64::INFINITY;
        }

        // Helper function for ellipsoid segment between two points
        // with vertical cross-sections that only rotate around Z axis
        let sd_ellipsoid_segment = |p: &Point3<f64>, a: &FRoomLinePoint, b: &FRoomLinePoint| {
            let a_point = Point3::new(
                a.location.x as f64,
                a.location.y as f64,
                a.location.z as f64,
            );
            let b_point = Point3::new(
                b.location.x as f64,
                b.location.y as f64,
                b.location.z as f64,
            );
            let a_h_range = a.h_range as f64;
            let b_h_range = b.h_range as f64;
            let a_v_range = a.v_range as f64;
            let b_v_range = b.v_range as f64;
            let a_floor_depth = a.floor_depth as f64;
            let b_floor_depth = b.floor_depth as f64;

            // Calculate direction in XY plane only (keeping cross-section vertical)
            let horizontal_dir = Point3::new(b_point.x - a_point.x, b_point.y - a_point.y, 0.0);
            let horizontal_length = horizontal_dir.coords.norm();

            if horizontal_length < 1e-6 {
                // Points are vertically aligned, just use a simple vertical cylinder
                let offset = Point3::new(p.x - a_point.x, p.y - a_point.y, 0.0);
                let horizontal_dist = offset.coords.norm();
                let vertical_dist = (p.z - a_point.z.min(b_point.z).max(a_point.z.max(b_point.z)))
                    .abs()
                    .min((p.z - a_point.z).abs().min((p.z - b_point.z).abs()));
                let r_h = (a_h_range + b_h_range) * 0.5;
                let r_v = (a_v_range + b_v_range) * 0.5;
                let floor_d = (a_floor_depth + b_floor_depth) * 0.5;

                let k0 = ((horizontal_dist / r_h).powi(2) + (vertical_dist / r_v).powi(2)).sqrt();
                let k1 = horizontal_dist / (r_h * r_h) + vertical_dist / (r_v * r_v);
                let ellipsoid_dist = if k1.abs() > 1e-10 {
                    k0 * (k0 - 1.0) / k1
                } else {
                    f64::INFINITY
                };

                // Apply floor constraint
                let avg_z = (a_point.z + b_point.z) * 0.5;
                let floor_dist = avg_z - floor_d - p.z;
                return ellipsoid_dist.max(floor_dist);
            }

            let horizontal_dir_normalized = horizontal_dir / horizontal_length;

            // Project point onto the horizontal line segment
            let pa_horizontal = Point3::new(p.x - a_point.x, p.y - a_point.y, 0.0);
            let projection_dist = pa_horizontal
                .coords
                .dot(&horizontal_dir_normalized.coords)
                .clamp(0.0, horizontal_length);

            // Interpolation parameter along the segment
            let t = projection_dist / horizontal_length;

            // Interpolated Z position, radii, and floor depth
            let z_interp = a_point.z * (1.0 - t) + b_point.z * t;
            let r_h = a_h_range * (1.0 - t) + b_h_range * t;
            let r_v = a_v_range * (1.0 - t) + b_v_range * t;
            let floor_d = a_floor_depth * (1.0 - t) + b_floor_depth * t;

            // Point on the segment axis (interpolated in XY, linearly interpolated in Z)
            let segment_point = Point3::new(
                a_point.x + horizontal_dir_normalized.x * projection_dist,
                a_point.y + horizontal_dir_normalized.y * projection_dist,
                z_interp,
            );

            let offset = p - segment_point;

            // Calculate distance to ellipsoid with vertical cross-sections
            let horizontal_dist = (offset.x * offset.x + offset.y * offset.y).sqrt();
            let vertical_dist = offset.z.abs();

            // Approximate SDF for ellipsoid
            let k0 = ((horizontal_dist / r_h).powi(2) + (vertical_dist / r_v).powi(2)).sqrt();
            let k1 = horizontal_dist / (r_h * r_h) + vertical_dist / (r_v * r_v);
            let ellipsoid_dist = if k1.abs() > 1e-10 {
                k0 * (k0 - 1.0) / k1
            } else {
                f64::INFINITY
            };

            // Apply floor constraint (plane at z_interp - floor_depth)
            // SDF for plane: distance = plane_z - point_z (positive when point is below plane)
            let floor_dist = z_interp - floor_d - p.z;

            // Max combines the SDFs (intersection: point must be inside ellipsoid AND above floor)
            ellipsoid_dist.max(floor_dist)
        };

        let mut min = f64::INFINITY;

        for pair in line.points.windows(2) {
            min = min.min(sd_ellipsoid_segment(p, &pair[0], &pair[1]));
        }

        min
    }
}

/// SDF for a FloodFillPillar - creates a capsule/pill shape along a polyline
/// Pillars fill material (opposite of carving), so this SDF is used for subtraction
fn flood_fill_pillar_sdf(pillar: &UFloodFillPillar) -> impl Fn(&Point3<f64>) -> f64 + '_ {
    move |p: &Point3<f64>| {
        if pillar.points.is_empty() {
            return f64::INFINITY;
        }

        // SDF for a capsule segment (sphere-swept line) with varying radius
        let sd_capsule_segment = |p: &Point3<f64>, a: &FRandLinePoint, b: &FRandLinePoint| {
            let a_point = Point3::new(
                a.location.x as f64,
                a.location.y as f64,
                a.location.z as f64,
            );
            let b_point = Point3::new(
                b.location.x as f64,
                b.location.y as f64,
                b.location.z as f64,
            );
            // Use average of min/max for deterministic radius
            let a_radius = ((a.range.min + a.range.max) * 0.5) as f64;
            let b_radius = ((b.range.min + b.range.max) * 0.5) as f64;

            // Vector from a to b
            let ab = Point3::new(
                b_point.x - a_point.x,
                b_point.y - a_point.y,
                b_point.z - a_point.z,
            );
            // Vector from a to p
            let ap = Point3::new(p.x - a_point.x, p.y - a_point.y, p.z - a_point.z);

            let len_sq = ab.x * ab.x + ab.y * ab.y + ab.z * ab.z;

            if len_sq < 1e-10 {
                // Points are coincident, just use sphere at a
                return ap.coords.norm() - a_radius;
            }

            // Project point onto line segment, clamp to [0, 1]
            let dot = ap.x * ab.x + ap.y * ab.y + ap.z * ab.z;
            let t = (dot / len_sq).clamp(0.0, 1.0);

            // Closest point on segment
            let closest = Point3::new(
                a_point.x + ab.x * t,
                a_point.y + ab.y * t,
                a_point.z + ab.z * t,
            );

            // Interpolated radius
            let radius = a_radius * (1.0 - t) + b_radius * t;

            // Distance to capsule surface
            let diff = Point3::new(p.x - closest.x, p.y - closest.y, p.z - closest.z);
            diff.coords.norm() - radius
        };

        let mut min = f64::INFINITY;

        for pair in pillar.points.windows(2) {
            min = min.min(sd_capsule_segment(p, &pair[0], &pair[1]));
        }

        min
    }
}

/// Compute the bounding box needed for CSG generation from visible mesh features
fn compute_csg_bounds<V: VisibilityCheck>(room: &URoomGenerator, visibility: &V) -> Option<Aabb> {
    let mut aabb = Aabb::new();

    fn process_features<V: VisibilityCheck>(
        features: &[URoomFeature],
        aabb: &mut Aabb,
        visibility: &V,
        path: &mut Vec<usize>,
    ) {
        path.push(0);
        for (i, feature) in features.iter().enumerate() {
            *path.last_mut().unwrap() = i;

            // Skip invisible features
            if !visibility.is_visible(path) {
                continue;
            }

            match &feature.feature_type {
                URoomFeatureType::FloodFillBox(f) => {
                    let pos: Vector3<f32> = vec3(f.position.x, f.position.y, f.position.z);
                    let ext: Vector3<f32> = vec3(f.extends.x, f.extends.y, f.extends.z);
                    aabb.expand_point(pos - ext);
                    aabb.expand_point(pos + ext);
                }
                URoomFeatureType::FloodFillPillar(f) => {
                    for point in &f.points {
                        let loc: Vector3<f32> =
                            vec3(point.location.x, point.location.y, point.location.z);
                        let r = point.range.max.max(point.range.min);
                        aabb.expand_point(loc - vec3(r, r, r));
                        aabb.expand_point(loc + vec3(r, r, r));
                    }
                }
                URoomFeatureType::FloodFillLine(f) => {
                    for point in &f.points {
                        let loc: Vector3<f32> =
                            vec3(point.location.x, point.location.y, point.location.z);
                        let h = point.h_range;
                        let v = point.v_range;
                        aabb.expand_point(loc - vec3(h, h, v + point.floor_depth));
                        aabb.expand_point(loc + vec3(h, h, v));
                    }
                }
                _ => {}
            }

            // Process children recursively
            process_features(&feature.children, aabb, visibility, path);
        }
        path.pop();
    }

    let mut path = Vec::new();
    process_features(&room.room_features, &mut aabb, visibility, &mut path);

    if aabb.is_valid() {
        Some(aabb.padded(0.1))
    } else {
        None
    }
}

/// Collect all visible FloodFillLine features from the room
fn collect_flood_fill_lines<'a, V: VisibilityCheck>(
    features: &'a [URoomFeature],
    visibility: &V,
) -> Vec<&'a UFloodFillLine> {
    let mut lines = Vec::new();

    fn collect_recursive<'a, V: VisibilityCheck>(
        features: &'a [URoomFeature],
        lines: &mut Vec<&'a UFloodFillLine>,
        visibility: &V,
        path: &mut Vec<usize>,
    ) {
        path.push(0);
        for (i, feature) in features.iter().enumerate() {
            *path.last_mut().unwrap() = i;

            // Skip invisible features
            if !visibility.is_visible(path) {
                continue;
            }

            if let URoomFeatureType::FloodFillLine(line) = &feature.feature_type {
                lines.push(line);
            }
            collect_recursive(&feature.children, lines, visibility, path);
        }
        path.pop();
    }

    let mut path = Vec::new();
    collect_recursive(features, &mut lines, visibility, &mut path);
    lines
}

/// Collect all visible FloodFillPillar features from the room
fn collect_flood_fill_pillars<'a, V: VisibilityCheck>(
    features: &'a [URoomFeature],
    visibility: &V,
) -> Vec<&'a UFloodFillPillar> {
    let mut pillars = Vec::new();

    fn collect_recursive<'a, V: VisibilityCheck>(
        features: &'a [URoomFeature],
        pillars: &mut Vec<&'a UFloodFillPillar>,
        visibility: &V,
        path: &mut Vec<usize>,
    ) {
        path.push(0);
        for (i, feature) in features.iter().enumerate() {
            *path.last_mut().unwrap() = i;

            // Skip invisible features
            if !visibility.is_visible(path) {
                continue;
            }

            if let URoomFeatureType::FloodFillPillar(pillar) = &feature.feature_type {
                pillars.push(pillar);
            }
            collect_recursive(&feature.children, pillars, visibility, path);
        }
        path.pop();
    }

    let mut path = Vec::new();
    collect_recursive(features, &mut pillars, visibility, &mut path);
    pillars
}

/// Build CSG from all visible mesh features in the room
pub fn build_csg_from_features<V: VisibilityCheck>(
    room: &URoomGenerator,
    visibility: &V,
) -> Option<CSG<()>> {
    let bounds = compute_csg_bounds(room, visibility)?;
    let lines = collect_flood_fill_lines(&room.room_features, visibility);
    let pillars = collect_flood_fill_pillars(&room.room_features, visibility);

    if lines.is_empty() && pillars.is_empty() {
        return None;
    }

    // Create combined SDF:
    // - Lines carve (create cave space): union (min) of all line SDFs
    // - Pillars fill material: subtract from cave using max(cave_sdf, -pillar_sdf)
    let combined_sdf = move |p: &Point3<f64>| {
        // Cave SDF from lines (min = union)
        let mut cave_dist = f64::INFINITY;
        for line in &lines {
            let sdf = flood_fill_line_sdf(line);
            cave_dist = cave_dist.min(sdf(p));
        }

        // Pillar SDF (min = union of all pillars)
        let mut pillar_dist = f64::INFINITY;
        for pillar in &pillars {
            let sdf = flood_fill_pillar_sdf(pillar);
            pillar_dist = pillar_dist.min(sdf(p));
        }

        // Subtract pillars from cave: max(cave, -pillar)
        // This removes pillar regions from the carved space
        if pillar_dist < f64::INFINITY {
            cave_dist.max(-pillar_dist)
        } else {
            cave_dist
        }
    };

    // Calculate grid resolution based on bounds size
    // Use approximately 1 unit per cell, clamped to reasonable limits
    let size = bounds.size();
    let max_dim = size.x.max(size.y).max(size.z);
    let resolution = ((max_dim / 50.0) as usize).clamp(50, 200);

    let csg = CSG::<()>::sdf(
        combined_sdf,
        (resolution, resolution, resolution),
        Point3::new(
            bounds.min.x as f64,
            bounds.min.y as f64,
            bounds.min.z as f64,
        ),
        Point3::new(
            bounds.max.x as f64,
            bounds.max.y as f64,
            bounds.max.z as f64,
        ),
        0.0,
        None,
    );

    Some(csg.tessellate())
}

/// Convert a tessellated CSG to a three-d CpuMesh
/// Uses standard winding so cave interior is visible from outside (one-sided mesh)
pub fn csg_to_three_d_mesh(csg: &CSG<()>) -> CpuMesh {
    let polygons = &csg.polygons;

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();

    let mut index_start = 0u32;

    for poly in polygons {
        // Skip degenerate polygons
        if poly.vertices.len() != 3 {
            continue;
        }

        // Push positions and normals (standard orientation for viewing from outside)
        for v in &poly.vertices {
            positions.push(vec3(v.pos.x as f32, v.pos.y as f32, v.pos.z as f32));
            normals.push(vec3(
                v.normal.x as f32,
                v.normal.y as f32,
                v.normal.z as f32,
            ));
        }

        // Standard winding order
        indices.push(index_start);
        indices.push(index_start + 1);
        indices.push(index_start + 2);
        index_start += 3;
    }

    CpuMesh {
        positions: Positions::F32(positions),
        normals: Some(normals),
        indices: Indices::U32(indices),
        ..Default::default()
    }
}
