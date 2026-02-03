//! Test height_scale and cieling_height parameters
//!
//! Run with: cargo run --example height_params -- output.json

use std::collections::BTreeSet;
use std::path::PathBuf;

use rma::objects::*;

fn make_point_full(
    x: f32,
    y: f32,
    z: f32,
    h_range: f32,
    v_range: f32,
    cieling_height: f32,
    height_scale: f32,
    floor_depth: f32,
) -> FRoomLinePoint {
    FRoomLinePoint {
        location: FVector { x, y, z },
        h_range,
        v_range,
        cieling_noise_range: 0.0,
        wall_noise_range: 0.0,
        floor_noise_range: 0.0,
        cieling_height,
        height_scale,
        floor_depth,
        floor_angle: 0.0,
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
        .unwrap_or_else(|| PathBuf::from("height_params.json"));

    let mut room_features = vec![];

    // All tunnels: h_range=500, v_range=500, floor_depth=-500, cieling_height=500
    // This gives a symmetric baseline where floor and ceiling are at the ellipsoid boundary

    // ==========================================================================
    // Row 1 (X=0): BASELINE - height_scale=100.0, identical tunnels for reference
    // ==========================================================================
    for i in 0..6 {
        let y = i as f32 * 2000.0;
        room_features.push(make_line(vec![
            make_point_full(0.0, y, 0.0, 500.0, 500.0, 500.0, 100.0, -500.0),
            make_point_full(1500.0, y, 0.0, 500.0, 500.0, 500.0, 100.0, -500.0),
        ]));
    }

    // ==========================================================================
    // Row 2 (X=3000): height_scale varying: 0.0, 0.5, 1.0, 1.5, 2.0, 3.0
    // ==========================================================================
    let height_scales = [0.0, 50.0, 100.0, 150.0, 200.0, 300.0];
    for (i, &hs) in height_scales.iter().enumerate() {
        let y = i as f32 * 2000.0;
        room_features.push(make_line(vec![
            make_point_full(3000.0, y, 0.0, 500.0, 500.0, 500.0, hs, -500.0),
            make_point_full(4500.0, y, 0.0, 500.0, 500.0, 500.0, hs, -500.0),
        ]));
    }

    // ==========================================================================
    // Row 3 (X=6000): height_scale varying, NO floor (floor_depth=0)
    // ==========================================================================
    for (i, &hs) in height_scales.iter().enumerate() {
        let y = i as f32 * 2000.0;
        room_features.push(make_line(vec![
            make_point_full(6000.0, y, 0.0, 500.0, 500.0, 500.0, hs, 0.0),
            make_point_full(7500.0, y, 0.0, 500.0, 500.0, 500.0, hs, 0.0),
        ]));
    }

    // ==========================================================================
    // Row 4 (X=9000): height_scale varying, NO ceiling (cieling_height=0)
    // ==========================================================================
    for (i, &hs) in height_scales.iter().enumerate() {
        let y = i as f32 * 2000.0;
        room_features.push(make_line(vec![
            make_point_full(9000.0, y, 0.0, 500.0, 500.0, 0.0, hs, -500.0),
            make_point_full(10500.0, y, 0.0, 500.0, 500.0, 0.0, hs, -500.0),
        ]));
    }

    // ==========================================================================
    // Row 5 (X=12000): height_scale varying, large v_range=1000
    // ==========================================================================
    for (i, &hs) in height_scales.iter().enumerate() {
        let y = i as f32 * 2000.0;
        room_features.push(make_line(vec![
            make_point_full(12000.0, y, 0.0, 500.0, 1000.0, 1000.0, hs, -1000.0),
            make_point_full(13500.0, y, 0.0, 500.0, 1000.0, 1000.0, hs, -1000.0),
        ]));
    }

    // ==========================================================================
    // Row 6 (X=15000): height_scale varying, small v_range=250
    // ==========================================================================
    for (i, &hs) in height_scales.iter().enumerate() {
        let y = i as f32 * 2000.0;
        room_features.push(make_line(vec![
            make_point_full(15000.0, y, 0.0, 500.0, 250.0, 250.0, hs, -250.0),
            make_point_full(16500.0, y, 0.0, 500.0, 250.0, 250.0, hs, -250.0),
        ]));
    }

    // ==========================================================================
    // Row 7 (X=18000): Transition height_scale along segment
    // ==========================================================================
    let hs_transitions = [
        (100.0, 100.0), // No change (baseline)
        (50.0, 150.0),  // Low to high
        (150.0, 50.0),  // High to low
        (0.0, 200.0),   // Zero to double
        (200.0, 0.0),   // Double to zero
        (100.0, 0.0),   // Normal to zero
    ];
    for (i, &(hs_start, hs_end)) in hs_transitions.iter().enumerate() {
        let y = i as f32 * 2000.0;
        room_features.push(make_line(vec![
            make_point_full(18000.0, y, 0.0, 500.0, 500.0, 500.0, hs_start, -500.0),
            make_point_full(19500.0, y, 0.0, 500.0, 500.0, 500.0, hs_end, -500.0),
        ]));
    }

    let room = URoomGenerator {
        base: URoomGeneratorBase {
            bounds: 22000.0,
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
