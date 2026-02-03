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

/// Reference to a segment within a primitive
#[derive(Clone, Copy)]
enum SegmentRef {
    Line { line_idx: usize, seg_idx: usize },
    Pillar { pillar_idx: usize, seg_idx: usize },
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

/// Build spatial bins by sampling each segment's SDF on a coarse grid
fn build_spatial_bins(
    bounds: &Aabb,
    lines: &[&UFloodFillLine],
    pillars: &[&UFloodFillPillar],
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

    bins
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

    // Build spatial acceleration structure
    let bins = build_spatial_bins(&bounds, &lines, &pillars);

    // Precompute pillar range scales
    let pillar_range_scales: Vec<f64> = pillars
        .iter()
        .map(|p| ((p.range_scale.min + p.range_scale.max) * 0.5) as f64)
        .collect();

    // Create combined SDF using spatial bins for acceleration:
    // - Lines carve (create cave space): union (min) of all line SDFs
    // - Pillars fill material: subtract from cave using max(cave_sdf, -pillar_sdf)
    let combined_sdf = |p: &Point3<f64>| {
        let bin_idx = bins.point_to_bin(p);
        let segments = &bins.bins[bin_idx];

        let mut cave_dist = f64::INFINITY;
        let mut pillar_dist = f64::INFINITY;

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
                    pillar_dist = pillar_dist.min(pillar_segment_sdf(a, b, range_scale, p));
                }
            }
        }

        // Subtract pillars from cave: max(cave, -pillar)
        if pillar_dist < f64::INFINITY {
            cave_dist.max(-pillar_dist)
        } else {
            cave_dist
        }
    };

    // Calculate grid resolution based on bounds size
    let size = bounds.size();
    let max_dim = size.x.max(size.y).max(size.z);
    let resolution = (max_dim / 50.0) as usize;

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
