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
        floor_angle: if floor_depth != 0.0 { 45.0 } else { 0.0 },
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

    let room_features = vec![
        // Tunnel 1: Basic ascending tunnel
        make_line(vec![
            make_point(1000.0, 0.0, 0.0, 750.0, 1000.0, 0.0),
            make_point(2000.0, 0.0, 2000.0, 1250.0, 1750.0, -300.0),
        ]),
        // Tunnel 2: Parallel tunnel
        make_line(vec![
            make_point(0.0, 2000.0, 0.0, 750.0, 1000.0, 0.0),
            make_point(2000.0, 2000.0, 2000.0, 1250.0, 1750.0, -300.0),
        ]),
        // Tunnel 3: Another branch
        make_line(vec![
            make_point(0.0, 4000.0, 0.0, 750.0, 1000.0, 0.0),
            make_point(2000.0, 4000.0, 2000.0, 1250.0, 1750.0, 0.0),
        ]),
        // Tunnel 4: Rising tunnel
        make_line(vec![
            make_point(0.0, 6000.0, 0.0, 750.0, 1000.0, 0.0),
            make_point(2000.0, 6000.0, 2000.0, 1250.0, 1750.0, 300.0),
        ]),
        // Tunnel 5: Descending tunnel
        make_line(vec![
            make_point(1900.0, -2000.0, 0.0, 750.0, 1000.0, 0.0),
            make_point(2000.0, -2000.0, 2000.0, 1250.0, 1750.0, -1000.0),
        ]),
    ];

    let room = URoomGenerator {
        base: URoomGeneratorBase {
            bounds: 10000.0,
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
