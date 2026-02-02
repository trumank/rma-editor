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

fn make_pillar_point(x: f32, y: f32, z: f32, radius: f32, skew: f32) -> FRandLinePoint {
    FRandLinePoint {
        location: FVector { x, y, z },
        range: FRandRange {
            min: radius,
            max: radius,
        },
        noise_range: FRandRange { min: 0.0, max: 0.0 },
        skew_factor: FRandRange {
            min: skew,
            max: skew,
        },
        fill_amount: FRandRange {
            min: 100.0,
            max: 100.0,
        },
    }
}

fn make_pillar(points: Vec<FRandLinePoint>, range_scale: f32, endcap_scale: f32) -> URoomFeature {
    URoomFeature {
        children: vec![],
        feature_type: URoomFeatureType::FloodFillPillar(UFloodFillPillar {
            noise_override: None,
            points,
            range_scale: FRandRange {
                min: range_scale,
                max: range_scale,
            },
            noise_range_scale: FRandRange { min: 1.0, max: 1.0 },
            endcap_scale: FRandRange {
                min: endcap_scale,
                max: endcap_scale,
            },
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

    // ==========================================================================
    // Row 5 (X=20000): Pillar skew_factor tests
    // ==========================================================================
    // Large room to contain the pillars
    room_features.push(make_line(vec![
        make_point(20000.0, 0.0, 1500.0, 2000.0, 1500.0, -1500.0, 0.0),
        make_point(20000.0, 24000.0, 1500.0, 2000.0, 1500.0, -1500.0, 0.0),
    ]));

    // Baseline: skew = 0
    room_features.push(make_pillar(
        vec![
            make_pillar_point(20000.0, 1000.0, 0.0, 300.0, 0.0),
            make_pillar_point(20000.0, 2500.0, 1500.0, 300.0, 0.0),
        ],
        1.0,
        1.0,
    ));

    // skew = 0.5
    room_features.push(make_pillar(
        vec![
            make_pillar_point(20000.0, 4000.0, 0.0, 300.0, 0.5),
            make_pillar_point(20000.0, 5500.0, 1500.0, 300.0, 0.5),
        ],
        1.0,
        1.0,
    ));

    // skew = 1.0
    room_features.push(make_pillar(
        vec![
            make_pillar_point(20000.0, 7000.0, 0.0, 300.0, 1.0),
            make_pillar_point(20000.0, 8500.0, 1500.0, 300.0, 1.0),
        ],
        1.0,
        1.0,
    ));

    // skew = 2.0
    room_features.push(make_pillar(
        vec![
            make_pillar_point(20000.0, 10000.0, 0.0, 300.0, 2.0),
            make_pillar_point(20000.0, 11500.0, 1500.0, 300.0, 2.0),
        ],
        1.0,
        1.0,
    ));

    // skew = -1.0
    room_features.push(make_pillar(
        vec![
            make_pillar_point(20000.0, 13000.0, 0.0, 300.0, -1.0),
            make_pillar_point(20000.0, 14500.0, 1500.0, 300.0, -1.0),
        ],
        1.0,
        1.0,
    ));

    // skew = 100.0
    room_features.push(make_pillar(
        vec![
            make_pillar_point(20000.0, 16000.0, 0.0, 300.0, 100.0),
            make_pillar_point(20000.0, 17500.0, 1500.0, 300.0, 100.0),
        ],
        1.0,
        1.0,
    ));

    // Opposing skew: bottom=1.0, top=-1.0
    room_features.push(make_pillar(
        vec![
            make_pillar_point(20000.0, 19000.0, 0.0, 300.0, 1.0),
            make_pillar_point(20000.0, 20500.0, 1500.0, 300.0, -1.0),
        ],
        1.0,
        1.0,
    ));

    // Opposing skew: bottom=-1.0, top=1.0
    room_features.push(make_pillar(
        vec![
            make_pillar_point(20000.0, 22000.0, 0.0, 300.0, -1.0),
            make_pillar_point(20000.0, 23500.0, 1500.0, 300.0, 1.0),
        ],
        1.0,
        1.0,
    ));

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
