//! CSG mesh generation from room features
//!
//! This module converts mesh primitives (FloodFillLine, etc.) into actual CSG meshes
//! using the csgrs crate for proper cave visualization.

use std::collections::HashMap;

use csgrs::float_types::parry3d::na::Point3;
use csgrs::mesh::Mesh;
use three_d::{CpuMesh, Indices, Positions, Vector3, vec3};

use super::room_features::Aabb;
use crate::objects::{
    FRandLinePoint, FRoomLinePoint, UFloodFillBox, UFloodFillLine, UFloodFillPillar, URoomFeature,
    URoomFeatureType, URoomGenerator,
};

/// Reference to a segment within a primitive
#[derive(Clone, Copy)]
enum SegmentRef {
    Line { line_idx: usize, seg_idx: usize },
    Pillar { pillar_idx: usize, seg_idx: usize },
    Box { box_idx: usize },
}

/// Spatial bins for accelerating SDF evaluation
struct SpatialBins {
    cell_size: f64,
    origin: Point3<f64>,
    dims: (usize, usize, usize),
    bins: Vec<Vec<SegmentRef>>,
}

impl SpatialBins {
    fn new(bounds: &Aabb, bin_count: usize) -> Self {
        let size = bounds.size();
        let max_dim = size.x.max(size.y).max(size.z) as f64;
        let cell_size = max_dim / bin_count as f64;

        let dims = (
            ((size.x as f64 / cell_size).ceil() as usize).max(1),
            ((size.y as f64 / cell_size).ceil() as usize).max(1),
            ((size.z as f64 / cell_size).ceil() as usize).max(1),
        );

        let total_bins = dims.0 * dims.1 * dims.2;

        SpatialBins {
            cell_size,
            origin: Point3::new(
                bounds.min.x as f64,
                bounds.min.y as f64,
                bounds.min.z as f64,
            ),
            dims,
            bins: vec![Vec::new(); total_bins],
        }
    }

    fn point_to_bin(&self, p: &Point3<f64>) -> usize {
        let x = ((p.x - self.origin.x) / self.cell_size) as usize;
        let y = ((p.y - self.origin.y) / self.cell_size) as usize;
        let z = ((p.z - self.origin.z) / self.cell_size) as usize;

        let x = x.min(self.dims.0 - 1);
        let y = y.min(self.dims.1 - 1);
        let z = z.min(self.dims.2 - 1);

        x + y * self.dims.0 + z * self.dims.0 * self.dims.1
    }

    fn bin_center(&self, bin_idx: usize) -> Point3<f64> {
        let x = bin_idx % self.dims.0;
        let y = (bin_idx / self.dims.0) % self.dims.1;
        let z = bin_idx / (self.dims.0 * self.dims.1);

        Point3::new(
            self.origin.x + (x as f64 + 0.5) * self.cell_size,
            self.origin.y + (y as f64 + 0.5) * self.cell_size,
            self.origin.z + (z as f64 + 0.5) * self.cell_size,
        )
    }

    fn total_bins(&self) -> usize {
        self.dims.0 * self.dims.1 * self.dims.2
    }

    /// Threshold for considering a segment as affecting a bin
    fn threshold(&self) -> f64 {
        // Cell diagonal ensures we catch segments that partially overlap
        self.cell_size * 1.8 // ~sqrt(3) ≈ 1.73
    }
}

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

/// SDF for a single line segment (ellipsoid tunnel)
fn line_segment_sdf(a: &FRoomLinePoint, b: &FRoomLinePoint, p: &Point3<f64>) -> f64 {
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

    // Scale space for projection to account for anisotropic ellipsoid
    let avg_r_h = ((a.h_range + b.h_range) as f64 / 2.0).max(0.01);
    let avg_r_v = ((a.v_range + b.v_range) as f64 / 2.0).max(0.01);

    // Transform into normalized space where ellipsoid is more spherical
    let a_scaled = Point3::new(
        a_point.x / avg_r_h,
        a_point.y / avg_r_h,
        a_point.z / avg_r_v,
    );
    let b_scaled = Point3::new(
        b_point.x / avg_r_h,
        b_point.y / avg_r_h,
        b_point.z / avg_r_v,
    );
    let p_scaled = Point3::new(p.x / avg_r_h, p.y / avg_r_h, p.z / avg_r_v);

    // Project in scaled space for correct anisotropic handling
    let ba_scaled = b_scaled - a_scaled;
    let pa_scaled = p_scaled - a_scaled;
    let ba_len_sq = ba_scaled.dot(&ba_scaled);

    let h = if ba_len_sq > 1e-12 {
        pa_scaled.dot(&ba_scaled) / ba_len_sq
    } else {
        0.5
    };
    let t = h.clamp(0.0, 1.0);

    // Segment direction in original space (needed for floor angle calculation)
    let ba = b_point - a_point;

    // Interpolate all parameters
    let r_h = (a.h_range as f64 * (1.0 - t) + b.h_range as f64 * t).max(0.01);
    let r_v = (a.v_range as f64 * (1.0 - t) + b.v_range as f64 * t).max(0.01);
    let floor_d = a.floor_depth as f64 * (1.0 - t) + b.floor_depth as f64 * t;
    let floor_angle = a.floor_angle as f64 * (1.0 - t) + b.floor_angle as f64 * t;

    // Point on the segment axis
    let segment_point = a_point + ba * t;
    let offset = p - segment_point;

    // Calculate perpendicular direction (right vector) for floor angle
    let horizontal_dir_len = (ba.x * ba.x + ba.y * ba.y).sqrt();
    let (right_x, right_y) = if horizontal_dir_len > 1e-6 {
        (-ba.y / horizontal_dir_len, ba.x / horizontal_dir_len)
    } else {
        (1.0, 0.0)
    };
    let perp_dist = offset.x * right_x + offset.y * right_y;

    // Ellipsoid SDF with vertically-oriented radii: (r_h, r_h, r_v)
    let qx = offset.x / r_h;
    let qy = offset.y / r_h;
    let qz = offset.z / r_v;
    let k0 = (qx * qx + qy * qy + qz * qz).sqrt();

    let q2x = offset.x / (r_h * r_h);
    let q2y = offset.y / (r_h * r_h);
    let q2z = offset.z / (r_v * r_v);
    let k1 = (q2x * q2x + q2y * q2y + q2z * q2z).sqrt();

    let ellipsoid_dist = if k1 > 1e-10 {
        k0 * (k0 - 1.0) / k1
    } else {
        f64::INFINITY
    };

    // Apply floor constraint with angle (tilted plane)
    let angle_rad = floor_angle.to_radians();
    let angle_offset = -angle_rad.sin() * perp_dist;
    let floor_z = segment_point.z + floor_d + angle_offset;
    let floor_dist = floor_z - p.z;

    // Max combines the SDFs (intersection: point must be inside ellipsoid AND above floor)
    ellipsoid_dist.max(floor_dist)
}

/// SDF for a single pillar segment (capsule/sphere-swept line)
fn pillar_segment_sdf(
    a: &FRandLinePoint,
    b: &FRandLinePoint,
    range_scale: f64,
    p: &Point3<f64>,
) -> f64 {
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
    // Use average of min/max for deterministic radius, apply range_scale
    let a_radius = ((a.range.min + a.range.max) * 0.5) as f64 * range_scale;
    let b_radius = ((b.range.min + b.range.max) * 0.5) as f64 * range_scale;

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
}

/// SDF for a rotated box
fn box_sdf(b: &UFloodFillBox, p: &Point3<f64>) -> f64 {
    let center = Point3::new(
        b.position.x as f64,
        b.position.y as f64,
        b.position.z as f64,
    );
    let half_extents = Point3::new(b.extends.x as f64, b.extends.y as f64, b.extends.z as f64);

    // Build rotation matrix from FRotator (pitch, yaw, roll in degrees)
    // Unreal uses left-handed coords, negate pitch to match
    let roll = (b.rotation.roll as f64).to_radians();
    let pitch = (-b.rotation.pitch as f64).to_radians();
    let yaw = (b.rotation.yaw as f64).to_radians();

    let (sr, cr) = roll.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    let (sy, cy) = yaw.sin_cos();

    // Rotation matrix: Rz(yaw) * Ry(pitch) * Rx(roll)
    // We need the inverse (transpose) to transform the point into local space
    let r00 = cy * cp;
    let r01 = cy * sp * sr - sy * cr;
    let r02 = cy * sp * cr + sy * sr;
    let r10 = sy * cp;
    let r11 = sy * sp * sr + cy * cr;
    let r12 = sy * sp * cr - cy * sr;
    let r20 = -sp;
    let r21 = cp * sr;
    let r22 = cp * cr;

    // Transform point to local box space (translate then rotate by inverse)
    let local_x = p.x - center.x;
    let local_y = p.y - center.y;
    let local_z = p.z - center.z;

    // Apply inverse rotation (transpose of rotation matrix)
    let qx = r00 * local_x + r10 * local_y + r20 * local_z;
    let qy = r01 * local_x + r11 * local_y + r21 * local_z;
    let qz = r02 * local_x + r12 * local_y + r22 * local_z;

    // Standard axis-aligned box SDF in local space
    let dx = qx.abs() - half_extents.x;
    let dy = qy.abs() - half_extents.y;
    let dz = qz.abs() - half_extents.z;

    let outside = (dx.max(0.0).powi(2) + dy.max(0.0).powi(2) + dz.max(0.0).powi(2)).sqrt();
    let inside = dx.max(dy).max(dz).min(0.0);

    outside + inside
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
                    let ext = &f.extends;
                    // For rotated box, use the diagonal as a conservative bounding radius
                    let diag = (ext.x * ext.x + ext.y * ext.y + ext.z * ext.z).sqrt();
                    aabb.expand_point(pos - vec3(diag, diag, diag));
                    aabb.expand_point(pos + vec3(diag, diag, diag));
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
                        aabb.expand_point(loc - vec3(h, h, v - point.floor_depth));
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

/// Collect all visible FloodFillBox features from the room
fn collect_flood_fill_boxes<'a, V: VisibilityCheck>(
    features: &'a [URoomFeature],
    visibility: &V,
) -> Vec<&'a UFloodFillBox> {
    let mut boxes = Vec::new();

    fn collect_recursive<'a, V: VisibilityCheck>(
        features: &'a [URoomFeature],
        boxes: &mut Vec<&'a UFloodFillBox>,
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

            if let URoomFeatureType::FloodFillBox(b) = &feature.feature_type {
                boxes.push(b);
            }
            collect_recursive(&feature.children, boxes, visibility, path);
        }
        path.pop();
    }

    let mut path = Vec::new();
    collect_recursive(features, &mut boxes, visibility, &mut path);
    boxes
}

/// Build spatial bins by sampling each segment's SDF on a coarse grid
fn build_spatial_bins(
    bounds: &Aabb,
    lines: &[&UFloodFillLine],
    pillars: &[&UFloodFillPillar],
    boxes: &[&UFloodFillBox],
) -> SpatialBins {
    const BIN_COUNT: usize = 20;
    let mut bins = SpatialBins::new(bounds, BIN_COUNT);
    let threshold = bins.threshold();
    let total_bins = bins.total_bins();

    // Sample each line segment
    for (line_idx, line) in lines.iter().enumerate() {
        for seg_idx in 0..line.points.len().saturating_sub(1) {
            let a = &line.points[seg_idx];
            let b = &line.points[seg_idx + 1];

            for bin_idx in 0..total_bins {
                let center = bins.bin_center(bin_idx);
                if line_segment_sdf(a, b, &center) < threshold {
                    bins.bins[bin_idx].push(SegmentRef::Line { line_idx, seg_idx });
                }
            }
        }
    }

    // Sample each pillar segment
    for (pillar_idx, pillar) in pillars.iter().enumerate() {
        let range_scale = ((pillar.range_scale.min + pillar.range_scale.max) * 0.5) as f64;

        for seg_idx in 0..pillar.points.len().saturating_sub(1) {
            let a = &pillar.points[seg_idx];
            let b = &pillar.points[seg_idx + 1];

            for bin_idx in 0..total_bins {
                let center = bins.bin_center(bin_idx);
                if pillar_segment_sdf(a, b, range_scale, &center) < threshold {
                    bins.bins[bin_idx].push(SegmentRef::Pillar {
                        pillar_idx,
                        seg_idx,
                    });
                }
            }
        }
    }

    // Sample each box
    for (box_idx, b) in boxes.iter().enumerate() {
        for bin_idx in 0..total_bins {
            let center = bins.bin_center(bin_idx);
            if box_sdf(b, &center).abs() < threshold {
                bins.bins[bin_idx].push(SegmentRef::Box { box_idx });
            }
        }
    }

    bins
}

/// Build CSG from all visible mesh features in the room
pub fn build_csg_from_features<V: VisibilityCheck>(
    room: &URoomGenerator,
    visibility: &V,
) -> Option<Mesh<()>> {
    let bounds = compute_csg_bounds(room, visibility)?;
    let lines = collect_flood_fill_lines(&room.room_features, visibility);
    let pillars = collect_flood_fill_pillars(&room.room_features, visibility);
    let boxes = collect_flood_fill_boxes(&room.room_features, visibility);

    if lines.is_empty() && pillars.is_empty() && boxes.is_empty() {
        return None;
    }

    // Build spatial acceleration structure
    let bins = build_spatial_bins(&bounds, &lines, &pillars, &boxes);

    // Precompute pillar range scales
    let pillar_range_scales: Vec<f64> = pillars
        .iter()
        .map(|p| ((p.range_scale.min + p.range_scale.max) * 0.5) as f64)
        .collect();

    // Create combined SDF using spatial bins for acceleration:
    // - Lines carve (create cave space): union (min) of all line SDFs
    // - Pillars fill material: subtract from cave using max(cave_sdf, -pillar_sdf)
    // - Boxes can either carve or fill depending on is_carver flag
    let combined_sdf = |p: &Point3<f64>| {
        let bin_idx = bins.point_to_bin(p);
        let segments = &bins.bins[bin_idx];

        let mut cave_dist = f64::INFINITY;
        let mut fill_dist = f64::INFINITY;

        for seg in segments {
            match *seg {
                SegmentRef::Line { line_idx, seg_idx } => {
                    let line = &lines[line_idx];
                    let a = &line.points[seg_idx];
                    let b = &line.points[seg_idx + 1];
                    cave_dist = cave_dist.min(line_segment_sdf(a, b, p));
                }
                SegmentRef::Pillar {
                    pillar_idx,
                    seg_idx,
                } => {
                    let pillar = &pillars[pillar_idx];
                    let a = &pillar.points[seg_idx];
                    let b = &pillar.points[seg_idx + 1];
                    let range_scale = pillar_range_scales[pillar_idx];
                    fill_dist = fill_dist.min(pillar_segment_sdf(a, b, range_scale, p));
                }
                SegmentRef::Box { box_idx } => {
                    let b = boxes[box_idx];
                    let dist = box_sdf(b, p);
                    if b.is_carver {
                        cave_dist = cave_dist.min(dist);
                    } else {
                        fill_dist = fill_dist.min(dist);
                    }
                }
            }
        }

        // Subtract fill from cave: max(cave, -fill)
        if fill_dist < f64::INFINITY {
            cave_dist.max(-fill_dist)
        } else {
            cave_dist
        }
    };

    // Calculate grid resolution proportional to each dimension
    let size = bounds.size();
    let cell_size = 50.0;
    let res_x = (size.x / cell_size).ceil().max(1.0) as usize;
    let res_y = (size.y / cell_size).ceil().max(1.0) as usize;
    let res_z = (size.z / cell_size).ceil().max(1.0) as usize;

    let csg = Mesh::<()>::sdf(
        combined_sdf,
        (res_x, res_y, res_z),
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

    Some(csg.triangulate())
}

/// Convert a tessellated CSG to a three-d CpuMesh
/// Uses standard winding so cave interior is visible from outside (one-sided mesh)
pub fn csg_to_three_d_mesh(csg: &Mesh<()>) -> CpuMesh {
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

        // Use plane normal for flat shading
        let n = poly.plane.normal();
        let face_normal = vec3(n.x as f32, n.y as f32, n.z as f32);

        // Push positions with flat face normal for all vertices
        for v in &poly.vertices {
            positions.push(vec3(v.pos.x as f32, v.pos.y as f32, v.pos.z as f32));
            normals.push(face_normal);
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
