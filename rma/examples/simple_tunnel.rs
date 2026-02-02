//! Generate a simple tunnel system using native Rust objects
//!
//! Run with: cargo run --example simple_tunnel -- output.json

use std::collections::BTreeSet;
use std::path::PathBuf;

use rma::objects::*;

fn make_point(
    x: f32,
    y: f32,
    z: f32,
    h_range: f32,
    v_range: f32,
    floor_depth: f32,
    floor_angle: f32,
) -> FRoomLinePoint {
    FRoomLinePoint {
        location: FVector { x, y, z },
        h_range,
        v_range,
        cieling_noise_range: 0.0,
        wall_noise_range: 0.0,
        floor_noise_range: 0.0,
        cieling_height: v_range,
        height_scale: 1.0,
        floor_depth,
        floor_angle,
    }
}

fn make_line(points: Vec<FRoomLinePoint>) -> URoomFeature {
    URoomFeature {
        children: vec![],
        feature_type: URoomFeatureType::FloodFillLine(UFloodFillLine {
            wall_noise_override: None,
            ceiling_noise_override: None,
            floor_noise_override: None,
            use_detail_noise: false,
            points,
        }),
    }
}

fn main() -> anyhow::Result<()> {
    let output_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("simple_tunnel.json"));

    let mut room_features = vec![];

    // ==========================================================================
    // Row 1 (X=0): Floor depth tests - 45° slope, varying floor_depth
    // ==========================================================================
    let floor_depths = [0.0, -300.0, -600.0, -900.0];
    for (i, &fd) in floor_depths.iter().enumerate() {
        let y = i as f32 * 3000.0;
        room_features.push(make_line(vec![
            make_point(0.0, y, 0.0, 1000.0, 1000.0, fd, 0.0),
            make_point(2000.0, y, 2000.0, 1000.0, 1000.0, fd, 0.0),
        ]));
    }

    // ==========================================================================
    // Row 2 (X=5000): Segment slope tests - no floor, varying vertical angle
    // ==========================================================================
    let slopes = [
        (2000.0, 0.0),    // Horizontal
        (2000.0, 500.0),  // Gentle (~14°)
        (2000.0, 2000.0), // 45°
        (1000.0, 2000.0), // Steep (~63°)
        (500.0, 2000.0),  // Very steep (~76°)
        (0.0, 2000.0),    // Vertical
    ];
    for (i, &(dx, dz)) in slopes.iter().enumerate() {
        let y = i as f32 * 3000.0;
        room_features.push(make_line(vec![
            make_point(5000.0, y, 0.0, 1000.0, 1000.0, -2000.0, 0.0),
            make_point(5000.0 + dx, y, dz, 1000.0, 1000.0, -2000.0, 0.0),
        ]));
    }

    // ==========================================================================
    // Row 3 (X=10000): Floor angle tests - 45° slope, varying floor_angle
    // ==========================================================================
    let floor_angles = [0.0, 15.0, 30.0, 45.0, -15.0, -30.0];
    for (i, &fa) in floor_angles.iter().enumerate() {
        let y = i as f32 * 3000.0;
        room_features.push(make_line(vec![
            make_point(10000.0, y, 0.0, 1000.0, 1000.0, 0.0, fa),
            make_point(12000.0, y, 2000.0, 1000.0, 1000.0, 0.0, fa),
        ]));
    }

    // ==========================================================================
    // Row 4 (X=15000): Opposing floor angle tests - angles transition between points
    // ==========================================================================
    let opposing_angles = [
        (30.0, -30.0), // Twist from +30 to -30
        (45.0, -45.0), // Twist from +45 to -45
        (-30.0, 30.0), // Twist from -30 to +30
        (45.0, 0.0),   // From tilted to flat
        (0.0, 45.0),   // From flat to tilted
    ];
    for (i, &(fa_start, fa_end)) in opposing_angles.iter().enumerate() {
        let y = i as f32 * 3000.0;
        room_features.push(make_line(vec![
            make_point(15000.0, y, 0.0, 1000.0, 1000.0, 0.0, fa_start),
            make_point(17000.0, y, 2000.0, 1000.0, 1000.0, 0.0, fa_end),
        ]));
    }

    let room = URoomGenerator {
        base: URoomGeneratorBase {
            bounds: 20000.0,
            can_only_be_used_once: false,
            mirror_support: ERoomMirroringSupport::MirrorAroundX,
            room_tags: FGameplayTagContainer(BTreeSet::new()),
        },
        room_features,
    };

    rma::save_room(&output_path, &room)?;
    eprintln!("Saved to {}", output_path.display());

    Ok(())
}
