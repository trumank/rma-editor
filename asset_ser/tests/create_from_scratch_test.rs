//! Test for creating assets from scratch

use asset_ser::{
    AssetVersionInfo,
    core::object_pool::{LoadedObject, ObjectHandle, ObjectPool, ObjectRef},
    object::UObject,
    parse_legacy_asset,
    saver::asset_saver,
    util::printer::ObjectPrinter,
};
use jmap::Jmap;
use std::fs;
use std::path::Path;

mod typed_properties;
use typed_properties::{
    FRandLinePoint, FRoomLinePoint, TypedArrayMut, TypedProperties,
    UDropPodCalldownLocationFeature, UFloodFillLine, UFloodFillPillar, URoomGenerator,
};

// Tuning constants for chaotic walk generation
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

// Vertical bias settings
const VERTICAL_BIAS_STRENGTH: f32 = 0.4; // Multiplier for distance-based Z offset

// Drop pod landing settings
const DROP_POD_DISTANCE: f32 = 2000.0; // Distance from center
const DROP_POD_CHAMBER_H_RANGE: f32 = 800.0;
const DROP_POD_CHAMBER_V_RANGE: f32 = 1000.0;
const DROP_POD_CEILING_HEIGHT: f32 = 1200.0;

/// Pseudo-random number generator using PCG (Permuted Congruential Generator)
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

/// Calculate vertical offset based on distance from center
fn calculate_vertical_bias(x: f64, y: f64, center_x: f64, center_y: f64) -> f64 {
    let dx = x - center_x;
    let dy = y - center_y;
    let distance = (dx * dx + dy * dy).sqrt();
    distance * VERTICAL_BIAS_STRENGTH as f64
}

/// Helper to create a FloodFillLine point using typed API
fn make_line_point(
    points: &mut TypedArrayMut<'_, FRoomLinePoint>,
    x: f32,
    y: f32,
    z: f32,
    h_range: f32,
    v_range: f32,
) {
    let mut point = points.push_default();
    point.location().x = x.into();
    point.location().y = y.into();
    point.location().z = z.into();
    *point.h_range() = h_range;
    *point.v_range() = v_range;
    *point.cieling_noise_range() = 0.0;
    *point.wall_noise_range() = 0.0;
    *point.floor_noise_range() = 0.0;
    *point.cieling_height() = v_range;
    *point.height_scale() = 1.0;
    *point.floor_depth() = 0.0;
    *point.floor_angle() = 0.0;
}

/// Helper to create a FloodFillLine point with full control over properties
fn make_line_point_full(
    points: &mut TypedArrayMut<'_, FRoomLinePoint>,
    x: f32,
    y: f32,
    z: f32,
    h_range: f32,
    v_range: f32,
    ceiling_height: f32,
    ceiling_noise: f32,
    wall_noise: f32,
    floor_noise: f32,
) {
    let mut point = points.push_default();
    point.location().x = x.into();
    point.location().y = y.into();
    point.location().z = z.into();
    *point.h_range() = h_range;
    *point.v_range() = v_range;
    *point.cieling_noise_range() = ceiling_noise;
    *point.wall_noise_range() = wall_noise;
    *point.floor_noise_range() = floor_noise;
    *point.cieling_height() = ceiling_height;
    *point.height_scale() = 1.0;
    *point.floor_depth() = 0.0;
    *point.floor_angle() = 0.0;
}

/// Helper to create a FloodFillPillar point using typed API
fn make_pillar_point(
    points: &mut TypedArrayMut<'_, FRandLinePoint>,
    x: f32,
    y: f32,
    z: f32,
    range: f32,
    noise: f32,
    fill: f32,
) {
    let mut point = points.push_default();
    point.location().x = x.into();
    point.location().y = y.into();
    point.location().z = z.into();

    point.range().min().0 = range;
    point.range().max().0 = range;

    point.noise_range().min().0 = noise;
    point.noise_range().max().0 = noise;

    point.skew_factor().min().0 = 0.0;
    point.skew_factor().max().0 = 0.0;

    point.fill_amount().min().0 = fill;
    point.fill_amount().max().0 = fill;
}

/// Helper to create a drop pod landing chamber with drop pod feature
fn create_drop_pod_landing(
    pool: &mut ObjectPool,
    root_handle: ObjectHandle,
    x: f32,
    y: f32,
    z: f32,
) -> Vec<ObjectRef> {
    let line_name = "DropPodLanding";

    let line_handle = pool.allocate(LoadedObject {
        name: line_name.into(),
        outer: Some(ObjectRef::Loaded(root_handle)),
        class: ObjectRef::Unloaded("/Script/FSD.FloodFillLine".into()),
        template: Some(ObjectRef::Unloaded(
            "/Script/FSD.Default__FloodFillLine".into(),
        )),
        object: Box::new(UObject::default()),
    });

    // Create the actual drop pod calldown location feature
    let drop_pod_feature_handle = pool.allocate(LoadedObject {
        name: "DropPodCalldownLocation".into(),
        outer: Some(ObjectRef::Loaded(line_handle)),
        class: ObjectRef::Unloaded("/Script/FSD.DropPodCalldownLocationFeature".into()),
        template: Some(ObjectRef::Unloaded(
            "/Script/FSD.Default__DropPodCalldownLocationFeature".into(),
        )),
        object: Box::new(UObject::default()),
    });

    // Configure the drop pod feature
    {
        let drop_pod_feature = pool.get_mut(drop_pod_feature_handle).unwrap();
        let mut typed_drop_pod =
            UDropPodCalldownLocationFeature::from_properties_mut(drop_pod_feature.properties_mut())
                .unwrap();

        // Set the location for the drop pod
        typed_drop_pod.location().x = x.into();
        typed_drop_pod.location().y = y.into();
        typed_drop_pod.location().z = z.into();

        // *typed_drop_pod.calldown_class() = ObjectRef::Unloaded(
        //     "/Game/LevelElements/Droppod/BP_SpawnDroppodLocationItem.BP_SpawnDroppodLocationItem_C"
        //         .into(),
        *typed_drop_pod.calldown_class() = ObjectRef::Unloaded(
            "/Game/LevelElements/Minehead/BP_Motherlode_MiningHeadDropLocation.BP_Motherlode_MiningHeadDropLocation_C"
                .into(),
        );
    }

    // Configure the drop pod landing chamber
    {
        let line = pool.get_mut(line_handle).unwrap();
        let mut typed_line = UFloodFillLine::from_properties_mut(line.properties_mut()).unwrap();
        let mut points = typed_line.points();

        // Create a single large chamber for the drop pod
        make_line_point_full(
            &mut points,
            x,
            y,
            z,
            DROP_POD_CHAMBER_H_RANGE,
            DROP_POD_CHAMBER_V_RANGE,
            DROP_POD_CEILING_HEIGHT,
            50.0, // ceiling_noise
            30.0, // wall_noise
            20.0, // floor_noise
        );
        make_line_point_full(
            &mut points,
            x + 100.0,
            y + 100.0,
            z,
            DROP_POD_CHAMBER_H_RANGE,
            DROP_POD_CHAMBER_V_RANGE,
            DROP_POD_CEILING_HEIGHT,
            50.0, // ceiling_noise
            30.0, // wall_noise
            20.0, // floor_noise
        );
    }

    vec![line_handle.into(), drop_pod_feature_handle.into()]
}

/// Helper to create a test line with pillar using typed API
fn create_test_line_with_pillar<F, G>(
    pool: &mut ObjectPool,
    root_handle: ObjectHandle,
    line_name: &str,
    configure_line: F,
    configure_pillar: G,
) -> ObjectHandle
where
    F: FnOnce(&mut TypedArrayMut<'_, FRoomLinePoint>),
    G: FnOnce(&mut TypedArrayMut<'_, FRandLinePoint>),
{
    // Allocate the line object first
    let line_handle = pool.allocate(LoadedObject {
        name: line_name.into(),
        outer: Some(ObjectRef::Loaded(root_handle)),
        class: ObjectRef::Unloaded("/Script/FSD.FloodFillLine".into()),
        template: Some(ObjectRef::Unloaded(
            "/Script/FSD.Default__FloodFillLine".into(),
        )),
        object: Box::new(UObject::default()),
    });

    // Create and allocate the pillar
    let pillar_handle = pool.allocate(LoadedObject {
        name: "Pillar".into(),
        outer: Some(ObjectRef::Loaded(line_handle)),
        class: ObjectRef::Unloaded("/Script/FSD.FloodFillPillar".into()),
        template: Some(ObjectRef::Unloaded(
            "/Script/FSD.Default__FloodFillPillar".into(),
        )),
        object: Box::new(UObject::default()),
    });

    // Configure pillar using typed API
    {
        let pillar = pool.get_mut(pillar_handle).unwrap();
        let mut typed_pillar =
            UFloodFillPillar::from_properties_mut(pillar.properties_mut()).unwrap();

        // Configure points
        let mut points = typed_pillar.points();
        configure_pillar(&mut points);

        // Set default range scales
        typed_pillar.range_scale().min().0 = 1.0;
        typed_pillar.range_scale().max().0 = 1.0;
        typed_pillar.noise_range_scale().min().0 = 1.0;
        typed_pillar.noise_range_scale().max().0 = 1.0;
        typed_pillar.endcap_scale().min().0 = 1.0;
        typed_pillar.endcap_scale().max().0 = 1.0;
    }

    // Configure line using typed API
    {
        let line = pool.get_mut(line_handle).unwrap();
        let mut typed_line = UFloodFillLine::from_properties_mut(line.properties_mut()).unwrap();

        // Configure points
        let mut points = typed_line.points();
        configure_line(&mut points);
    }

    // Add pillar to line's RoomFeatures
    {
        let line = pool.get_mut(line_handle).unwrap();
        let mut typed_line = UFloodFillLine::from_properties_mut(line.properties_mut()).unwrap();
        typed_line
            .room_features_objects()
            .push(ObjectRef::Loaded(pillar_handle));
    }

    line_handle
}

#[test]
fn test_create_from_scratch() -> anyhow::Result<()> {
    let jmap_path = Path::new("fsd.jmap");
    let jmap_data = fs::read_to_string(jmap_path)?;
    let jmap: Jmap = serde_json::from_str(&jmap_data)?;

    // Load a reference asset to get version info
    let reference_path = Path::new("RMA_Test.uasset");
    let header = parse_legacy_asset(reference_path)?;
    let version = AssetVersionInfo::from_package_header(&header);

    let mut pool = ObjectPool::new();

    // Create the root RoomGenerator object
    let root_name = "RMA_Test";

    let root_obj = LoadedObject {
        name: root_name.into(),
        outer: None,
        class: ObjectRef::Unloaded("/Script/FSD.RoomGenerator".into()),
        template: Some(ObjectRef::Unloaded(
            "/Script/FSD.Default__RoomGenerator".into(),
        )),
        object: Box::new(UObject::default()),
    };

    let root_handle = pool.allocate(root_obj);

    // Calculate drop pod position first
    let _drop_pod_seed = 987654321u64;
    let angle = std::f32::consts::PI; //rand_range(&mut drop_pod_seed, 0.0, 2.0 * std::f32::consts::PI);
    let drop_pod_x = DROP_POD_DISTANCE * angle.cos();
    let drop_pod_y = DROP_POD_DISTANCE * angle.sin();
    let drop_pod_z = 0.0;

    // Create a FloodFillLine object using typed API
    let line_name = "FloodFillLine_1";

    // Generate chaotic walk points starting from drop pod location
    let walk_points = generate_chaotic_walk_points(
        WALK_NUM_POINTS,
        (drop_pod_x as f64, drop_pod_y as f64, drop_pod_z as f64),
    );

    let line_handle = pool.allocate(LoadedObject {
        name: line_name.into(),
        outer: Some(ObjectRef::Loaded(root_handle)),
        class: ObjectRef::Unloaded("/Script/FSD.FloodFillLine".into()),
        template: Some(ObjectRef::Unloaded(
            "/Script/FSD.Default__FloodFillLine".into(),
        )),
        object: Box::new(UObject::default()),
    });

    // Build the FloodFillLine properties using typed API
    {
        let line = pool.get_mut(line_handle).unwrap();
        let mut typed_line = UFloodFillLine::from_properties_mut(line.properties_mut()).unwrap();
        let mut points = typed_line.points();

        let mut rng_seed = 123456789u64;

        for (x, y, z) in walk_points.into_iter() {
            // Chamber sizing
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

            let mut point = points.push_default();

            // Location
            point.location().x = (x as f32).into();
            point.location().y = (y as f32).into();
            point.location().z = (z as f32).into();

            // Ranges with chamber multipliers
            *point.h_range() = rand_range(&mut rng_seed, H_RANGE_MIN, H_RANGE_MAX) * h_mult;
            *point.v_range() = rand_range(&mut rng_seed, V_RANGE_MIN, V_RANGE_MAX) * v_mult;

            // Noise ranges
            *point.cieling_noise_range() = rand_range(&mut rng_seed, 0.0, NOISE_RANGE_MAX);
            *point.wall_noise_range() = rand_range(&mut rng_seed, 0.0, NOISE_RANGE_MAX);
            *point.floor_noise_range() = rand_range(&mut rng_seed, 0.0, NOISE_RANGE_MAX);

            // Ceiling and other properties
            *point.cieling_height() =
                rand_range(&mut rng_seed, CEILING_HEIGHT_MIN, CEILING_HEIGHT_MAX) * v_mult;
            *point.height_scale() = rand_range(&mut rng_seed, 0.5, 2.0);
            *point.floor_depth() = 0.0;
            *point.floor_angle() = rand_range(&mut rng_seed, -30.0, 30.0);
        }
    }

    // Create test FloodFillPillar objects, each with its own FloodFillLine
    let mut root_room_features = vec![ObjectRef::loaded(line_handle)];

    // Add drop pod landing chamber at the calculated position
    let drop_pod_handles =
        create_drop_pod_landing(&mut pool, root_handle, drop_pod_x, drop_pod_y, drop_pod_z);
    root_room_features.extend(drop_pod_handles);

    // test_shapes(&mut pool, root_handle, &mut root_room_features);

    // Add all features to the root's RoomFeatures array using typed API
    {
        let root = pool.get_mut(root_handle).unwrap();
        let mut typed_root = URoomGenerator::from_properties_mut(root.properties_mut()).unwrap();

        // Set bounds
        *typed_root.bounds() = 10000.0;

        // Add room features
        *typed_root.room_features_objects() = root_room_features;
    }

    println!("{}", ObjectPrinter::new(&pool).print_object(root_handle)?);

    // Save the asset
    let output_path =
        Path::new("new_mod_P/FSD/Content/_AssemblyStorm/SandboxUtilities/MapGen/RMA_Test.uasset");

    println!("Saving asset to {:?}", output_path);

    asset_saver::save_asset(
        output_path,
        &pool,
        vec![root_handle],
        version,
        "RMA_Test".to_string(),
        &jmap,
    )?;

    println!("Asset created successfully!");
    Ok(())
}

/// Helper to create a FloodFillLine point with floor depth control
fn make_line_point_with_floor_depth(
    points: &mut TypedArrayMut<'_, FRoomLinePoint>,
    x: f32,
    y: f32,
    z: f32,
    h_range: f32,
    v_range: f32,
    floor_depth: f32,
) {
    let mut point = points.push_default();
    point.location().x = x.into();
    point.location().y = y.into();
    point.location().z = z.into();
    *point.h_range() = h_range;
    *point.v_range() = v_range;
    *point.cieling_noise_range() = 0.0;
    *point.wall_noise_range() = 0.0;
    *point.floor_noise_range() = 0.0;
    *point.cieling_height() = v_range;
    *point.height_scale() = 1.0;
    *point.floor_depth() = floor_depth;
    *point.floor_angle() = 45.0;
}

fn test_shapes(
    pool: &mut ObjectPool,
    root_handle: ObjectHandle,
    root_room_features: &mut Vec<ObjectRef>,
) {
    // Test 1: Simple cylindrical pillar
    root_room_features.push(ObjectRef::loaded(create_test_line_with_pillar(
        pool,
        root_handle,
        "Test1_Cylinder",
        |points| {
            make_line_point(points, -1000.0, 20000.0, 0.0, 800.0, 600.0);
            make_line_point(points, 3000.0, 20000.0, 0.0, 800.0, 600.0);
        },
        |points| {
            make_pillar_point(points, 1000.0, 20000.0, -300.0, 200.0, 0.0, 100.0);
            make_pillar_point(points, 1000.0, 20000.0, 300.0, 200.0, 0.0, 100.0);
        },
    )));

    // Test 2: Cone (varying range)
    root_room_features.push(ObjectRef::loaded(create_test_line_with_pillar(
        pool,
        root_handle,
        "Test2_Cone",
        |points| {
            make_line_point(points, -1000.0, 24000.0, 0.0, 800.0, 600.0);
            make_line_point(points, 3000.0, 24000.0, 0.0, 800.0, 600.0);
        },
        |points| {
            make_pillar_point(points, 1000.0, 24000.0, -300.0, 350.0, 0.0, 100.0);
            make_pillar_point(points, 1000.0, 24000.0, 300.0, 100.0, 0.0, 100.0);
        },
    )));

    // Test 3: Noisy surface
    root_room_features.push(ObjectRef::loaded(create_test_line_with_pillar(
        pool,
        root_handle,
        "Test3_Noisy",
        |points| {
            make_line_point(points, -1000.0, 28000.0, 0.0, 800.0, 600.0);
            make_line_point(points, 3000.0, 28000.0, 0.0, 800.0, 600.0);
        },
        |points| {
            make_pillar_point(points, 1000.0, 28000.0, -300.0, 250.0, 150.0, 100.0);
            make_pillar_point(points, 1000.0, 28000.0, 300.0, 250.0, 150.0, 100.0);
        },
    )));

    // Test 4: Partial fill (50%)
    root_room_features.push(ObjectRef::loaded(create_test_line_with_pillar(
        pool,
        root_handle,
        "Test4_PartialFill",
        |points| {
            make_line_point(points, -1000.0, 32000.0, 0.0, 800.0, 600.0);
            make_line_point(points, 3000.0, 32000.0, 0.0, 800.0, 600.0);
        },
        |points| {
            make_pillar_point(points, 1000.0, 32000.0, -300.0, 250.0, 0.0, 50.0);
            make_pillar_point(points, 1000.0, 32000.0, 300.0, 250.0, 0.0, 50.0);
        },
    )));

    // Test 5: Multi-segment (3 points)
    root_room_features.push(ObjectRef::loaded(create_test_line_with_pillar(
        pool,
        root_handle,
        "Test5_MultiSegment",
        |points| {
            make_line_point(points, -1000.0, 36000.0, 0.0, 800.0, 800.0);
            make_line_point(points, 3000.0, 36000.0, 0.0, 800.0, 800.0);
        },
        |points| {
            make_pillar_point(points, 1000.0, 36000.0, -400.0, 300.0, 0.0, 100.0);
            make_pillar_point(points, 1000.0, 36000.0, 0.0, 150.0, 0.0, 100.0);
            make_pillar_point(points, 1000.0, 36000.0, 400.0, 250.0, 0.0, 100.0);
        },
    )));

    // Test 6: Horizontal bridge (pillar going sideways with multiple points)
    root_room_features.push(ObjectRef::loaded(create_test_line_with_pillar(
        pool,
        root_handle,
        "Test6_Bridge",
        |points| {
            make_line_point(points, -1000.0, 40000.0, 0.0, 800.0, 800.0);
            make_line_point(points, 3000.0, 40000.0, 0.0, 800.0, 800.0);
        },
        |points| {
            make_pillar_point(points, 0.0, 40000.0, 100.0, 200.0, 0.0, 100.0);
            make_pillar_point(points, 1000.0, 40000.0, 100.0, 200.0, 0.0, 100.0);
            make_pillar_point(points, 2000.0, 40000.0, 100.0, 200.0, 0.0, 100.0);
        },
    )));
}

fn generate_chaotic_walk_points(num_points: usize, start: (f64, f64, f64)) -> Vec<(f64, f64, f64)> {
    let mut rng_seed = 123456789u64;
    let mut points = Vec::new();
    let mut current = start;
    let mut current_angle_h = 0.0f32;
    let mut current_angle_v = 0.0f32;
    let mut anchor_points = vec![start];

    // Use start position as the center for vertical bias
    let center_x = start.0;
    let center_y = start.1;

    for i in 0..num_points {
        points.push((
            current.0,
            current.1,
            // Apply vertical bias based on distance from the drop pod (start position)
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
                    current.2 = spiral_start.2 + ((j as f32 * 25.0) as f64);
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

#[test]
fn test_create_simple_tunnel() -> anyhow::Result<()> {
    let jmap_path = Path::new("fsd.jmap");
    let jmap_data = fs::read_to_string(jmap_path)?;
    let jmap: Jmap = serde_json::from_str(&jmap_data)?;

    // Load a reference asset to get version info
    let reference_path = Path::new("RMA_Test.uasset");
    let header = parse_legacy_asset(reference_path)?;
    let version = AssetVersionInfo::from_package_header(&header);

    let mut pool = ObjectPool::new();

    // Create the root RoomGenerator object
    let root_name = "RMA_Test";

    let root_obj = LoadedObject {
        name: root_name.into(),
        outer: None,
        class: ObjectRef::Unloaded("/Script/FSD.RoomGenerator".into()),
        template: Some(ObjectRef::Unloaded(
            "/Script/FSD.Default__RoomGenerator".into(),
        )),
        object: Box::new(UObject::default()),
    };

    let root_handle = pool.allocate(root_obj);

    let mut lines = vec![];

    let mut n = 0;
    fn mk_line<'a>(
        pool: &'a mut ObjectPool,
        n: &mut i32,
        root_handle: &ObjectHandle,
        lines: &mut Vec<ObjectRef>,
    ) -> &'a mut LoadedObject {
        *n += 1;
        let l = LoadedObject {
            name: format!("SimpleTunnel_{n}").into(),
            outer: Some(ObjectRef::Loaded(*root_handle)),
            class: ObjectRef::Unloaded("/Script/FSD.FloodFillLine".into()),
            template: Some(ObjectRef::Unloaded(
                "/Script/FSD.Default__FloodFillLine".into(),
            )),
            object: Box::new(UObject::default()),
        };
        let l = pool.allocate(l);
        lines.push(l.into());
        pool.get_mut(l).unwrap()
    }

    // Build the FloodFillLine properties using typed API
    {
        let line = mk_line(&mut pool, &mut n, &root_handle, &mut lines);
        let mut typed_line = UFloodFillLine::from_properties_mut(line.properties_mut()).unwrap();
        let mut points = typed_line.points();

        #[rustfmt::skip]
        let mut buh = || {
            // make_line_point_with_floor_depth( &mut points, 0000.0, 0000.0, 0000.0, 0750.0, 1000.0, 0000.0, );
            // make_line_point_with_floor_depth( &mut points, 2500.0, 4000.0, 2500.0, 1250.0, 1750.0, -0750.0, );
            // make_line_point_with_floor_depth( &mut points, 6000.0, 6000.0, 5000.0, 1750.0, 2500.0, -1250.0, );
            // make_line_point_with_floor_depth( &mut points, 10000.0, 4000.0, 6500.0, 1500.0, 2000.0, -0500.0, );
            // make_line_point_with_floor_depth( &mut points, 12500.0, 0000.0, 7500.0, 1000.0, 1500.0, 0500.0, );
            // make_line_point_with_floor_depth( &mut points, 11500.0, -3500.0, 9000.0, 1250.0, 1750.0, -1000.0, );
            // make_line_point_with_floor_depth( &mut points, 7500.0, -5000.0, 11000.0, 1500.0, 2250.0, -0750.0, );
            // make_line_point_with_floor_depth( &mut points, 2500.0, -4000.0, 12500.0, 1000.0, 1500.0, 0000.0, );
            // make_line_point_with_floor_depth( &mut points, 0000.0, 0000.0, 14000.0, 0750.0, 1250.0, 0000.0, );

            make_line_point_with_floor_depth( &mut points, 1000.0, 0000.0, 0000.0, 0750.0, 1000.0, 0.0);
            make_line_point_with_floor_depth( &mut points, 2000.0, 0000.0, 2000.0, 1250.0, 1750.0, -300.0);
        };
        buh()
    }

    // Build the FloodFillLine properties using typed API
    {
        let line = mk_line(&mut pool, &mut n, &root_handle, &mut lines);
        let mut typed_line = UFloodFillLine::from_properties_mut(line.properties_mut()).unwrap();
        let mut points = typed_line.points();

        #[rustfmt::skip]
        let mut buh = || {
            make_line_point_with_floor_depth( &mut points, 0000.0, 2000.0, 0000.0, 0750.0, 1000.0, 0.0);
            make_line_point_with_floor_depth( &mut points, 2000.0, 2000.0, 2000.0, 1250.0, 1750.0, -300.0);
        };
        buh()
    }

    {
        let line = mk_line(&mut pool, &mut n, &root_handle, &mut lines);
        let mut typed_line = UFloodFillLine::from_properties_mut(line.properties_mut()).unwrap();
        let mut points = typed_line.points();

        #[rustfmt::skip]
        let mut buh = || {
            make_line_point_with_floor_depth( &mut points, 0000.0, 4000.0, 0000.0, 0750.0, 1000.0, 0.0);
            make_line_point_with_floor_depth( &mut points, 2000.0, 4000.0, 2000.0, 1250.0, 1750.0, 0.0);
        };
        buh()
    }

    {
        let line = mk_line(&mut pool, &mut n, &root_handle, &mut lines);
        let mut typed_line = UFloodFillLine::from_properties_mut(line.properties_mut()).unwrap();
        let mut points = typed_line.points();

        #[rustfmt::skip]
        let mut buh = || {
            make_line_point_with_floor_depth( &mut points, 0000.0, 6000.0, 0000.0, 0750.0, 1000.0, 0.0);
            make_line_point_with_floor_depth( &mut points, 2000.0, 6000.0, 2000.0, 1250.0, 1750.0, 300.0);
        };
        buh()
    }

    {
        let line = mk_line(&mut pool, &mut n, &root_handle, &mut lines);
        let mut typed_line = UFloodFillLine::from_properties_mut(line.properties_mut()).unwrap();
        let mut points = typed_line.points();

        #[rustfmt::skip]
        let mut buh = || {
            make_line_point_with_floor_depth( &mut points, 1900.0, -2000.0, 0000.0, 0750.0, 1000.0, 0.0);
            make_line_point_with_floor_depth( &mut points, 2000.0, -2000.0, 2000.0, 1250.0, 1750.0, -1000.0);
        };
        buh()
    }

    // Add line to root's RoomFeatures array using typed API
    {
        let root = pool.get_mut(root_handle).unwrap();
        let mut typed_root = URoomGenerator::from_properties_mut(root.properties_mut()).unwrap();

        // Set bounds
        *typed_root.bounds() = 10000.0;

        // Add room features
        *typed_root.room_features_objects() = lines;
    }

    println!("{}", ObjectPrinter::new(&pool).print_object(root_handle)?);

    // Save the asset
    let output_path =
        Path::new("new_mod_P/FSD/Content/_AssemblyStorm/SandboxUtilities/MapGen/RMA_Test.uasset");

    println!("Saving asset to {:?}", output_path);

    asset_saver::save_asset(
        output_path,
        &pool,
        vec![root_handle],
        version,
        "RMA_Test".to_string(),
        &jmap,
    )?;

    println!("Asset created successfully!");
    Ok(())
}
