//! Test the FloodFillBox primitive with various sizes and orientations
//!
//! Run with: cargo run --example flood_fill_box -- output.json

use std::collections::BTreeSet;
use std::path::PathBuf;

use rma::objects::*;

fn make_box(
    x: f32,
    y: f32,
    z: f32,
    size_x: f32,
    size_y: f32,
    size_z: f32,
    pitch: f32,
    yaw: f32,
    roll: f32,
) -> URoomFeature {
    URoomFeature {
        children: vec![],
        feature_type: URoomFeatureType::FloodFillBox(UFloodFillBox {
            noise: None,
            position: FVector { x, y, z },
            extends: FVector {
                x: size_x,
                y: size_y,
                z: size_z,
            },
            rotation: FRotator { pitch, yaw, roll },
            is_carver: true,
            noise_range: 0.0,
        }),
    }
}

fn make_point(x: f32, y: f32, z: f32, h_range: f32, v_range: f32) -> FRoomLinePoint {
    FRoomLinePoint {
        location: FVector { x, y, z },
        h_range,
        v_range,
        cieling_noise_range: 0.0,
        wall_noise_range: 0.0,
        floor_noise_range: 0.0,
        cieling_height: v_range,
        height_scale: 1.0,
        floor_depth: 0.0,
        floor_angle: 0.0,
    }
}

fn make_line_with_children(
    points: Vec<FRoomLinePoint>,
    children: Vec<URoomFeature>,
) -> URoomFeature {
    URoomFeature {
        children,
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
        .unwrap_or_else(|| PathBuf::from("flood_fill_box.json"));

    let mut room_features = vec![];

    // ==========================================================================
    // Row 1 (X=0): Size tests - varying dimensions
    // ==========================================================================
    let size_tests = vec![
        // Small cube
        make_box(0.0, 0.0, 0.0, 500.0, 500.0, 500.0, 0.0, 0.0, 0.0),
        // Medium cube
        make_box(0.0, 3000.0, 0.0, 1000.0, 1000.0, 1000.0, 0.0, 0.0, 0.0),
        // Large cube
        make_box(0.0, 6000.0, 0.0, 1500.0, 1500.0, 1500.0, 0.0, 0.0, 0.0),
        // Wide/flat box (X-dominant)
        make_box(0.0, 9000.0, 0.0, 2000.0, 500.0, 500.0, 0.0, 0.0, 0.0),
        // Deep box (Y-dominant)
        make_box(0.0, 12000.0, 0.0, 500.0, 2000.0, 500.0, 0.0, 0.0, 0.0),
        // Tall box (Z-dominant)
        make_box(0.0, 15000.0, 0.0, 500.0, 500.0, 2000.0, 0.0, 0.0, 0.0),
    ];
    room_features.push(make_line_with_children(
        vec![
            make_point(0.0, 0.0, 0.0, 100.0, 100.0),
            make_point(0.0, 15000.0, 0.0, 100.0, 100.0),
        ],
        size_tests,
    ));

    // ==========================================================================
    // Row 2 (X=5000): Yaw rotation tests (rotation around Z axis)
    // ==========================================================================
    let yaw_angles = [0.0, 15.0, 30.0, 45.0, 60.0, 90.0];
    let yaw_tests: Vec<_> = yaw_angles
        .iter()
        .enumerate()
        .map(|(i, &yaw)| {
            let y = i as f32 * 3000.0;
            make_box(5000.0, y, 0.0, 1500.0, 500.0, 500.0, 0.0, yaw, 0.0)
        })
        .collect();
    room_features.push(make_line_with_children(
        vec![
            make_point(5000.0, 0.0, 0.0, 100.0, 100.0),
            make_point(5000.0, 15000.0, 0.0, 100.0, 100.0),
        ],
        yaw_tests,
    ));

    // ==========================================================================
    // Row 3 (X=10000): Pitch rotation tests (rotation around Y axis)
    // ==========================================================================
    let pitch_angles = [0.0, 15.0, 30.0, 45.0, 60.0, 90.0];
    let pitch_tests: Vec<_> = pitch_angles
        .iter()
        .enumerate()
        .map(|(i, &pitch)| {
            let y = i as f32 * 3000.0;
            make_box(10000.0, y, 0.0, 1500.0, 500.0, 500.0, pitch, 0.0, 0.0)
        })
        .collect();
    room_features.push(make_line_with_children(
        vec![
            make_point(10000.0, 0.0, 0.0, 100.0, 100.0),
            make_point(10000.0, 15000.0, 0.0, 100.0, 100.0),
        ],
        pitch_tests,
    ));

    // ==========================================================================
    // Row 4 (X=15000): Roll rotation tests (rotation around X axis)
    // ==========================================================================
    let roll_angles = [0.0, 15.0, 30.0, 45.0, 60.0, 90.0];
    let roll_tests: Vec<_> = roll_angles
        .iter()
        .enumerate()
        .map(|(i, &roll)| {
            let y = i as f32 * 3000.0;
            make_box(15000.0, y, 0.0, 1500.0, 500.0, 500.0, 0.0, 0.0, roll)
        })
        .collect();
    room_features.push(make_line_with_children(
        vec![
            make_point(15000.0, 0.0, 0.0, 100.0, 100.0),
            make_point(15000.0, 15000.0, 0.0, 100.0, 100.0),
        ],
        roll_tests,
    ));

    // ==========================================================================
    // Row 5 (X=20000): Combined rotation tests
    // ==========================================================================
    let combined_tests = vec![
        // Yaw + Pitch
        make_box(20000.0, 0.0, 0.0, 1500.0, 500.0, 500.0, 30.0, 45.0, 0.0),
        // Yaw + Roll
        make_box(20000.0, 3000.0, 0.0, 1500.0, 500.0, 500.0, 0.0, 45.0, 30.0),
        // Pitch + Roll
        make_box(20000.0, 6000.0, 0.0, 1500.0, 500.0, 500.0, 30.0, 0.0, 45.0),
        // All three rotations
        make_box(20000.0, 9000.0, 0.0, 1500.0, 500.0, 500.0, 30.0, 45.0, 60.0),
        // Extreme combined rotation
        make_box(
            20000.0, 12000.0, 0.0, 1500.0, 500.0, 500.0, 45.0, 45.0, 45.0,
        ),
        // Negative rotations combined
        make_box(
            20000.0, 15000.0, 0.0, 1500.0, 500.0, 500.0, -30.0, -45.0, -60.0,
        ),
    ];
    room_features.push(make_line_with_children(
        vec![
            make_point(20000.0, 0.0, 0.0, 100.0, 100.0),
            make_point(20000.0, 15000.0, 0.0, 100.0, 100.0),
        ],
        combined_tests,
    ));

    let room = URoomGenerator {
        base: URoomGeneratorBase {
            bounds: 25000.0,
            can_only_be_used_once: false,
            mirror_support: ERoomMirroringSupport::NotAllowed,
            room_tags: FGameplayTagContainer(BTreeSet::new()),
        },
        room_features,
    };

    rma::save_room(&output_path, &room)?;
    eprintln!("Saved to {}", output_path.display());

    Ok(())
}
