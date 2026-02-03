//! Generate a chaotic cave system using native Rust objects
//!
//! Run with: cargo run --example generate_chaotic_walk -- output.json

use std::collections::BTreeSet;
use std::path::PathBuf;

use rma::objects::*;

// Tuning constants
const WALK_NUM_POINTS: usize = 150;

// Chamber sizing ranges
const H_RANGE_MIN: f32 = 300.0;
const H_RANGE_MAX: f32 = 800.0;
const V_RANGE_MIN: f32 = 300.0;
const V_RANGE_MAX: f32 = 800.0;
const CEILING_HEIGHT_MIN: f32 = 400.0;
const CEILING_HEIGHT_MAX: f32 = 800.0;

// Noise ranges
const NOISE_RANGE_MAX: f32 = 200.0;

// Chamber multipliers
const CHAMBER_PROBABILITY: f32 = 0.88;
const CHAMBER_H_MULT_MIN: f32 = 2.0;
const CHAMBER_H_MULT_MAX: f32 = 3.5;
const CHAMBER_V_MULT_MIN: f32 = 2.0;
const CHAMBER_V_MULT_MAX: f32 = 3.5;
const CHAMBER_BOTH_MULT_MIN: f32 = 1.8;
const CHAMBER_BOTH_MULT_MAX: f32 = 2.8;

// Step distances
const STEP_LARGE_MIN: f32 = 2000.0;
const STEP_LARGE_MAX: f32 = 3000.0;
const STEP_SMALL_MIN: f32 = 200.0;
const STEP_SMALL_MAX: f32 = 600.0;
const STEP_REGULAR_MIN: f32 = 100.0;
const STEP_REGULAR_MAX: f32 = 2000.0;

// Spiral settings
const SPIRAL_RADIUS_MIN: f32 = 2000.0;
const SPIRAL_RADIUS_MAX: f32 = 4000.0;
const SPIRAL_INTERVAL: usize = 30;
const SPIRAL_LENGTH: usize = 15;

// Vertical bias
const VERTICAL_BIAS_STRENGTH: f32 = 0.4;

// Drop pod settings
const DROP_POD_DISTANCE: f32 = 2000.0;
const DROP_POD_CHAMBER_H_RANGE: f32 = 800.0;
const DROP_POD_CHAMBER_V_RANGE: f32 = 1000.0;
const DROP_POD_CEILING_HEIGHT: f32 = 1200.0;

/// PCG random number generator
fn pseudo_rand(seed: &mut u64) -> f32 {
    let oldstate = *seed;
    *seed = oldstate
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    let xorshifted = (((oldstate >> 18) ^ oldstate) >> 27) as u32;
    let rot = (oldstate >> 59) as u32;
    let result = xorshifted.rotate_right(rot);
    result as f32 / u32::MAX as f32
}

fn rand_range(seed: &mut u64, min: f32, max: f32) -> f32 {
    min + pseudo_rand(seed) * (max - min)
}

fn calculate_vertical_bias(x: f64, y: f64, center_x: f64, center_y: f64) -> f64 {
    let dx = x - center_x;
    let dy = y - center_y;
    let distance = (dx * dx + dy * dy).sqrt();
    distance * VERTICAL_BIAS_STRENGTH as f64
}

fn make_line_point(
    x: f32,
    y: f32,
    z: f32,
    h_range: f32,
    v_range: f32,
    ceiling_height: f32,
) -> FRoomLinePoint {
    FRoomLinePoint {
        location: FVector { x, y, z },
        h_range,
        v_range,
        cieling_noise_range: 50.0,
        wall_noise_range: 30.0,
        floor_noise_range: 20.0,
        cieling_height: ceiling_height,
        height_scale: 1.0,
        floor_depth: 0.0,
        floor_angle: 0.0,
    }
}

fn create_drop_pod_landing(x: f32, y: f32, z: f32) -> Vec<URoomFeature> {
    let landing_line = URoomFeature {
        children: vec![],
        feature_type: URoomFeatureType::FloodFillLine(UFloodFillLine {
            wall_noise_override: None,
            ceiling_noise_override: None,
            floor_noise_override: None,
            use_detail_noise: false,
            points: vec![
                make_line_point(
                    x,
                    y,
                    z,
                    DROP_POD_CHAMBER_H_RANGE,
                    DROP_POD_CHAMBER_V_RANGE,
                    DROP_POD_CEILING_HEIGHT,
                ),
                make_line_point(
                    x + 100.0,
                    y + 100.0,
                    z,
                    DROP_POD_CHAMBER_H_RANGE,
                    DROP_POD_CHAMBER_V_RANGE,
                    DROP_POD_CEILING_HEIGHT,
                ),
            ],
        }),
    };

    let drop_pod = URoomFeature {
        children: vec![],
        feature_type: URoomFeatureType::DropPodCalldownLocation(UDropPodCalldownLocationFeature {
            location: FVector { x, y, z },
            calldown_class: Some("/Game/LevelElements/Minehead/BP_Motherlode_MiningHeadDropLocation.BP_Motherlode_MiningHeadDropLocation_C".into()),
        }),
    };

    vec![landing_line, drop_pod]
}

fn generate_chaotic_walk_points(num_points: usize, start: (f64, f64, f64)) -> Vec<(f64, f64, f64)> {
    let mut rng_seed = 123456789u64;
    let mut points = Vec::new();
    let mut current = start;
    let mut current_angle_h = 0.0f32;
    let mut current_angle_v = 0.0f32;
    let mut anchor_points = vec![start];

    let center_x = start.0;
    let center_y = start.1;

    for i in 0..num_points {
        points.push((
            current.0,
            current.1,
            current.2 + calculate_vertical_bias(current.0, current.1, center_x, center_y),
        ));

        if i % 25 == 0 && i > 0 {
            anchor_points.push(current);
        }

        let pull_to_anchor = rand_range(&mut rng_seed, 0.0, 1.0);
        if pull_to_anchor > 0.75 && !anchor_points.is_empty() {
            let anchor_idx =
                (rand_range(&mut rng_seed, 0.0, 1.0) * anchor_points.len() as f32) as usize;
            let anchor_idx = anchor_idx.min(anchor_points.len() - 1);
            let anchor = anchor_points[anchor_idx];

            let dx = anchor.0 - current.0;
            let dy = anchor.1 - current.1;
            let target_angle = (dy as f32).atan2(dx as f32);

            current_angle_h = current_angle_h * 0.3 + target_angle * 0.7;
        } else {
            let turn_intensity = rand_range(&mut rng_seed, 0.0, 1.0);

            if turn_intensity > 0.85 {
                current_angle_h = rand_range(&mut rng_seed, 0.0, 2.0 * std::f64::consts::PI as f32);
                current_angle_v = rand_range(
                    &mut rng_seed,
                    -std::f64::consts::PI as f32 / 12.0,
                    std::f64::consts::PI as f32 / 12.0,
                );
            } else {
                let turn_h = rand_range(&mut rng_seed, -0.6, 0.6);
                let turn_v = rand_range(&mut rng_seed, -0.05, 0.05);
                current_angle_h += turn_h;
                current_angle_v = (current_angle_v + turn_v).clamp(
                    -std::f64::consts::PI as f32 / 12.0,
                    std::f64::consts::PI as f32 / 12.0,
                );
            }
        }

        if i % SPIRAL_INTERVAL == 0 && i > 0 {
            let spiral_length = SPIRAL_LENGTH;
            if i + spiral_length < num_points {
                let spiral_radius = rand_range(&mut rng_seed, SPIRAL_RADIUS_MIN, SPIRAL_RADIUS_MAX);
                let angle_step = 2.0 * std::f64::consts::PI as f32 / spiral_length as f32;

                let spiral_start = current;
                let base_angle = current_angle_h;

                for j in 0..spiral_length.min(num_points - i - 1) {
                    let angle = base_angle + (j as f32 * angle_step);
                    current.0 = spiral_start.0 + (spiral_radius * angle.cos()) as f64;
                    current.1 = spiral_start.1 + (spiral_radius * angle.sin()) as f64;
                    current.2 = spiral_start.2 + (j as f32 * 25.0) as f64;
                    points.push(current);
                }
                continue;
            }
        }

        if rand_range(&mut rng_seed, 0.0, 1.0) > 0.92 {
            current_angle_h += std::f64::consts::PI as f32 * rand_range(&mut rng_seed, 0.8, 1.2);
        }

        let step_type = rand_range(&mut rng_seed, 0.0, 1.0);
        let step = if step_type > 0.85 {
            rand_range(&mut rng_seed, STEP_LARGE_MIN, STEP_LARGE_MAX)
        } else if step_type < 0.25 {
            rand_range(&mut rng_seed, STEP_SMALL_MIN, STEP_SMALL_MAX)
        } else {
            rand_range(&mut rng_seed, STEP_REGULAR_MIN, STEP_REGULAR_MAX)
        };

        current.0 += step as f64 * (current_angle_h as f64).cos() * (current_angle_v as f64).cos();
        current.1 += step as f64 * (current_angle_h as f64).sin() * (current_angle_v as f64).cos();
        current.2 += step as f64 * (current_angle_v as f64).sin() * 0.3;
    }

    points
}

fn main() -> anyhow::Result<()> {
    let output_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("chaotic_walk.json"));

    // Calculate drop pod position
    let angle = std::f32::consts::PI;
    let drop_pod_x = DROP_POD_DISTANCE * angle.cos();
    let drop_pod_y = DROP_POD_DISTANCE * angle.sin();
    let drop_pod_z = 0.0;

    // Generate chaotic walk points
    let walk_points = generate_chaotic_walk_points(
        WALK_NUM_POINTS,
        (drop_pod_x as f64, drop_pod_y as f64, drop_pod_z as f64),
    );

    // Convert walk points to FloodFillLine points
    let mut rng_seed = 123456789u64;
    let line_points: Vec<FRoomLinePoint> = walk_points
        .into_iter()
        .map(|(x, y, z)| {
            let is_chamber = rand_range(&mut rng_seed, 0.0, 1.0) > CHAMBER_PROBABILITY;
            let (h_mult, v_mult) = if is_chamber {
                let chamber_type = rand_range(&mut rng_seed, 0.0, 1.0);
                if chamber_type < 0.33 {
                    (
                        rand_range(&mut rng_seed, CHAMBER_H_MULT_MIN, CHAMBER_H_MULT_MAX),
                        1.0,
                    )
                } else if chamber_type < 0.66 {
                    (
                        1.0,
                        rand_range(&mut rng_seed, CHAMBER_V_MULT_MIN, CHAMBER_V_MULT_MAX),
                    )
                } else {
                    (
                        rand_range(&mut rng_seed, CHAMBER_BOTH_MULT_MIN, CHAMBER_BOTH_MULT_MAX),
                        rand_range(&mut rng_seed, CHAMBER_BOTH_MULT_MIN, CHAMBER_BOTH_MULT_MAX),
                    )
                }
            } else {
                (1.0, 1.0)
            };

            FRoomLinePoint {
                location: FVector {
                    x: x as f32,
                    y: y as f32,
                    z: z as f32,
                },
                h_range: rand_range(&mut rng_seed, H_RANGE_MIN, H_RANGE_MAX) * h_mult,
                v_range: rand_range(&mut rng_seed, V_RANGE_MIN, V_RANGE_MAX) * v_mult,
                cieling_noise_range: rand_range(&mut rng_seed, 0.0, NOISE_RANGE_MAX),
                wall_noise_range: rand_range(&mut rng_seed, 0.0, NOISE_RANGE_MAX),
                floor_noise_range: rand_range(&mut rng_seed, 0.0, NOISE_RANGE_MAX),
                cieling_height: rand_range(&mut rng_seed, CEILING_HEIGHT_MIN, CEILING_HEIGHT_MAX)
                    * v_mult,
                height_scale: rand_range(&mut rng_seed, 0.5, 2.0),
                floor_depth: 0.0,
                floor_angle: rand_range(&mut rng_seed, -30.0, 30.0),
            }
        })
        .collect();

    // Create main tunnel feature
    let main_tunnel = URoomFeature {
        children: vec![],
        feature_type: URoomFeatureType::FloodFillLine(UFloodFillLine {
            wall_noise_override: None,
            ceiling_noise_override: None,
            floor_noise_override: None,
            use_detail_noise: false,
            points: line_points,
        }),
    };

    // Create drop pod landing
    let drop_pod_features = create_drop_pod_landing(drop_pod_x, drop_pod_y, drop_pod_z);

    // Combine all features
    let mut room_features = vec![main_tunnel];
    room_features.extend(drop_pod_features);

    // Create the room generator
    let room = URoomGenerator {
        base: URoomGeneratorBase {
            bounds: 10000.0,
            can_only_be_used_once: false,
            mirror_support: ERoomMirroringSupport::NotAllowed,
            room_tags: FGameplayTagContainer(BTreeSet::new()),
        },
        room_features,
    };

    // Save as JSON
    rma::save_room(&output_path, &room)?;
    eprintln!("Saved to {}", output_path.display());

    Ok(())
}
