//! Test for editing and saving assets

use asset_ser::{
    AssetVersionInfo,
    core::object_pool::{AssetArchiveType, ObjectHandle, ObjectPool},
    loader::asset_loader,
    parse_legacy_asset,
    saver::asset_saver,
};
use jmap::Jmap;
use std::fs;
use std::path::Path;
use uesave::{Property, StructValue, ValueVec};

/// Generate points in a circle formation
///
/// # Arguments
/// * `center` - Center point of the circle
/// * `radius` - Radius of the circle
/// * `num_points` - Number of points to generate
/// * `z_offset` - Z offset for each point (can be used to create a spiral)
///
/// # Returns
/// Vector of (x, y, z) tuples representing points on the circle
fn generate_circle_points(
    center: (f64, f64, f64),
    radius: f64,
    num_points: usize,
    z_offset: f64,
) -> Vec<(f64, f64, f64)> {
    let mut points = Vec::new();
    let angle_step = 2.0 * std::f64::consts::PI / num_points as f64;

    for i in 0..num_points {
        let angle = i as f64 * angle_step;
        let x = center.0 + radius * angle.cos();
        let y = center.1 + radius * angle.sin();
        let z = center.2 + (i as f64 * z_offset);
        points.push((x, y, z));
    }

    points
}

#[test]
fn test_edit_location_and_save() -> anyhow::Result<()> {
    let asset_path = Path::new("RMA_Test.uasset");
    let jmap_path = Path::new("fsd.jmap");

    // Load jmap
    let jmap_data = fs::read_to_string(jmap_path)?;
    let jmap: Jmap = serde_json::from_str(&jmap_data)?;

    // Load the original asset
    let header = parse_legacy_asset(asset_path)?;
    println!("Original package: {}", header.summary.package_name);

    // Load asset into pool
    let mut pool = ObjectPool::new();
    let _root = asset_loader::load_asset(asset_path, &mut pool)
        .map_err(|e| anyhow::anyhow!("Failed to load asset: {}", e))?;

    println!("Loaded {} objects into pool", pool.len());

    // Find the FloodFillLine_12 object
    let target_path = "RMA_Test.FloodFillLine_12";
    let target_handle = pool
        .find_by_path(&target_path.into())
        .ok_or_else(|| anyhow::anyhow!("Could not find object: {}", target_path))?;

    println!("\nFound target object: {}", target_path);

    // Get mutable reference to the object
    let object = pool
        .get_mut(target_handle)
        .ok_or_else(|| anyhow::anyhow!("Could not get mutable reference to object"))?;

    // Print original location
    println!("\n=== Original Properties ===");
    if let Some((_, points_prop)) = object.properties().0.iter().find(|(k, _)| k.1 == "Points") {
        println!("Points property found");
        if let Property::Array(uesave::ValueVec::Struct(structs)) = points_prop {
            println!("Points array has {} elements", structs.len());
            if let Some(StructValue::Struct(first_point)) = structs.first()
                && let Some((_, Property::Struct(StructValue::Vector(vec)))) =
                    first_point.0.iter().find(|(k, _)| k.1 == "Location")
            {
                println!("Original Location[0]: ({}, {}, {})", vec.x, vec.y, vec.z);
            }
        }
    }

    // Modify the location of the first point
    // println!("\n=== Modifying Location ===");
    // if let Some((_, points_prop)) = object
    //     .properties
    //     .0
    //     .iter_mut()
    //     .find(|(k, _)| k.1 == "Points")
    // {
    //     if let Property::Array(array) = points_prop {
    //         if let uesave::ValueVec::Struct(structs) = array {
    //             if let Some(StructValue::Struct(first_point)) = structs.first_mut() {
    //                 if let Some((_, location_prop)) =
    //                     first_point.0.iter_mut().find(|(k, _)| k.1 == "Location")
    //                 {
    //                     if let Property::Struct(StructValue::Vector(vec)) = location_prop {
    //                         // Change the location
    //                         let old_x = vec.x;
    //                         let old_y = vec.y;
    //                         let old_z = vec.z;

    //                         vec.x.0 = -30000.0;
    //                         vec.y.0 = -5000.0;
    //                         vec.z.0 = 2000.0;

    //                         println!(
    //                             "Changed Location[0] from ({}, {}, {}) to ({}, {}, {})",
    //                             old_x, old_y, old_z, vec.x, vec.y, vec.z
    //                         );
    //                     }
    //                 }
    //             }
    //         }
    //     }
    // }

    // make_circle(&mut pool, target_handle)?;
    // make_drunk_dwarf(&mut pool, target_handle)?;
    make_chaotic_walk(&mut pool, target_handle)?;

    // Save the modified asset
    println!("\n=== Saving Modified Asset ===");
    let output_path =
        Path::new("new_mod_P/FSD/Content/_AssemblyStorm/SandboxUtilities/MapGen/RMA_Test.uasset");
    let version = AssetVersionInfo::from_package_header(&header);

    // Get all root objects (objects with no outer in the pool)
    let mut root_handles = vec![];
    for (handle, obj) in pool.iter() {
        if obj.outer.is_none() {
            root_handles.push(handle);
        }
    }

    println!("Found {} root objects", root_handles.len());

    asset_saver::save_asset(
        output_path,
        &pool,
        root_handles,
        version,
        header.summary.package_name.clone(),
        &jmap,
    )?;

    Ok(())
}

/// Apply pulsating effect to HRange/VRange (breathing tunnel)
fn apply_pulsating_size(index: usize, total: usize, base_size: f64, amplitude: f64) -> (f64, f64) {
    let t = (index as f64 / total as f64) * 4.0 * std::f64::consts::PI;
    let size = base_size + amplitude * t.sin();
    (size, size)
}

/// Apply funnel effect (wide -> narrow -> wide)
fn apply_funnel_size(index: usize, total: usize, max_size: f64, min_size: f64) -> (f64, f64) {
    let t = index as f64 / total as f64;
    // Parabola shape: small at edges, large in middle
    let size = min_size + (max_size - min_size) * (1.0 - (2.0 * t - 1.0).powi(2));
    (size, size)
}

/// Apply elliptical variation (alternating tall/wide sections)
fn apply_elliptical_variation(index: usize, total: usize, base_size: f64) -> (f64, f64) {
    let t = (index as f64 / total as f64) * 2.0 * std::f64::consts::PI;
    let h_range = base_size * (1.0 + 0.5 * t.cos());
    let v_range = base_size * (1.0 + 0.5 * t.sin());
    (h_range, v_range)
}

/// Apply random organic variation
fn apply_random_variation(base_size: f64, variation: f64) -> (f64, f64) {
    // Simple deterministic "random" based on current time as seed
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    let h_rand = ((seed % 1000) as f64 / 1000.0) * variation;
    let v_rand = (((seed / 1000) % 1000) as f64 / 1000.0) * variation;

    (
        base_size + h_rand - variation / 2.0,
        base_size + v_rand - variation / 2.0,
    )
}

/// Generate a chaotic "drunk dwarf" path - random walk with wild size variations
fn generate_drunk_dwarf_path(
    start: (f64, f64, f64),
    num_points: usize,
    step_size: f64,
) -> Vec<(f64, f64, f64)> {
    let mut points = Vec::new();
    let mut current = start;

    // Use a simple pseudo-random generator based on index for determinism
    for i in 0..num_points {
        points.push(current);

        // Generate "random" angles using simple math on index
        let angle_h = ((i * 7919) % 360) as f64 * std::f64::consts::PI / 180.0;
        let angle_v =
            ((i * 5179) % 120) as f64 * std::f64::consts::PI / 180.0 - std::f64::consts::PI / 3.0;

        // Random step in 3D space
        let step_variation = 0.5 + ((i * 3571) % 100) as f64 / 100.0;
        let actual_step = step_size * step_variation;

        current.0 += actual_step * angle_h.cos() * angle_v.cos();
        current.1 += actual_step * angle_h.sin() * angle_v.cos();
        current.2 += actual_step * angle_v.sin();
    }

    points
}

/// Pseudo-random number generator using PCG (Permuted Congruential Generator)
/// Much better quality than simple LCG
fn pseudo_rand(seed: &mut u64) -> f32 {
    let oldstate = *seed;
    // LCG step
    *seed = oldstate
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    // PCG output function (XSH-RR)
    let xorshifted = (((oldstate >> 18) ^ oldstate) >> 27) as u32;
    let rot = (oldstate >> 59) as u32;
    let result = xorshifted.rotate_right(rot);
    result as f32 / u32::MAX as f32
}

/// Random float in range [min, max]
fn rand_range(seed: &mut u64, min: f32, max: f32) -> f32 {
    min + pseudo_rand(seed) * (max - min)
}

fn make_chaotic_walk(pool: &mut ObjectPool, target: ObjectHandle) -> anyhow::Result<()> {
    let num_points = 100;
    let start = (0.0, 30000.0, 0.0);

    // Seed based on a prime number for variety
    let mut rng_seed = 123456789u64;

    let mut points = Vec::new();
    let mut current = start;

    // Track current direction for smoother but still chaotic turning
    let mut current_angle_h = 0.0f32;
    let mut current_angle_v = 0.0f32;

    // Track "anchor points" to encourage backtracking/clustering
    let mut anchor_points = vec![start];

    println!("\n=== Generating chaotic random walk ===");

    for i in 0..num_points {
        points.push(current);

        // Every so often, add current position as a potential anchor point
        if i % 25 == 0 && i > 0 {
            anchor_points.push(current);
        }

        // Occasionally pull towards a random anchor point (creates clustering/loops)
        let pull_to_anchor = rand_range(&mut rng_seed, 0.0, 1.0);
        if pull_to_anchor > 0.75 && !anchor_points.is_empty() {
            // Pick a random anchor point to head towards
            let anchor_idx =
                (rand_range(&mut rng_seed, 0.0, 1.0) * anchor_points.len() as f32) as usize;
            let anchor_idx = anchor_idx.min(anchor_points.len() - 1);
            let anchor = anchor_points[anchor_idx];

            // Calculate angle towards anchor
            let dx = anchor.0 - current.0;
            let dy = anchor.1 - current.1;
            let target_angle = (dy as f32).atan2(dx as f32);

            // Blend current direction with anchor direction
            current_angle_h = current_angle_h * 0.3 + target_angle * 0.7;
        } else {
            // Normal random walk behavior
            let turn_intensity = rand_range(&mut rng_seed, 0.0, 1.0);

            if turn_intensity > 0.85 {
                // Sharp turn! Completely random new direction (mostly horizontal)
                current_angle_h = rand_range(&mut rng_seed, 0.0, 2.0 * std::f64::consts::PI as f32);
                // Much smaller vertical angles - keep it mostly horizontal
                current_angle_v = rand_range(
                    &mut rng_seed,
                    -std::f64::consts::PI as f32 / 12.0,
                    std::f64::consts::PI as f32 / 12.0,
                );
            } else {
                // Gentle turn - adjust current direction
                let turn_h = rand_range(&mut rng_seed, -0.6, 0.6);
                // Much gentler vertical changes
                let turn_v = rand_range(&mut rng_seed, -0.05, 0.05);
                current_angle_h += turn_h;
                // Tighter vertical angle limits - keep it mostly flat
                current_angle_v = (current_angle_v + turn_v).clamp(
                    -std::f64::consts::PI as f32 / 12.0,
                    std::f64::consts::PI as f32 / 12.0,
                );
            }
        }

        // Add some spiral sections
        if i % 30 == 0 && i > 0 {
            // Create a tight spiral or loop
            let spiral_length = 15;
            if i + spiral_length < num_points {
                let spiral_radius = rand_range(&mut rng_seed, 1000.0, 2000.0);
                let angle_step = 2.0 * std::f64::consts::PI as f32 / spiral_length as f32;

                // Save current position for spiral
                let spiral_start = current;
                let base_angle = current_angle_h;

                for j in 0..spiral_length.min(num_points - i - 1) {
                    let angle = base_angle + (j as f32 * angle_step);
                    current.0 = spiral_start.0 + (spiral_radius * angle.cos()) as f64;
                    current.1 = spiral_start.1 + (spiral_radius * angle.sin()) as f64;
                    // Gentle z variation in spiral
                    current.2 = spiral_start.2 + ((j as f32 * 25.0) as f64);
                    points.push(current);
                }
                continue;
            }
        }

        // More frequent 180 turns for backtracking
        if rand_range(&mut rng_seed, 0.0, 1.0) > 0.92 {
            current_angle_h += std::f64::consts::PI as f32 * rand_range(&mut rng_seed, 0.8, 1.2);
        }

        // Variable step size - more variation for clustering
        let step_type = rand_range(&mut rng_seed, 0.0, 1.0);
        let step = if step_type > 0.85 {
            // Big leap!
            rand_range(&mut rng_seed, 1000.0, 2000.0)
        } else if step_type < 0.25 {
            // Tiny step (increased frequency for more dense areas)
            rand_range(&mut rng_seed, 100.0, 300.0)
        } else {
            // Normal step
            rand_range(&mut rng_seed, 400.0, 750.0)
        };

        current.0 += step as f64 * (current_angle_h as f64).cos() * (current_angle_v as f64).cos();
        current.1 += step as f64 * (current_angle_h as f64).sin() * (current_angle_v as f64).cos();
        // Scale down vertical movement significantly
        current.2 += step as f64 * (current_angle_v as f64).sin() * 0.3;
    }

    let object = pool
        .get_mut(target)
        .ok_or_else(|| anyhow::anyhow!("Could not get mutable reference to object"))?;

    let points_prop = prop_mut(object.properties_mut(), "Points");

    if let Property::Array(ValueVec::Struct(structs)) = points_prop
        && let Some(StructValue::Struct(template_point)) = structs.first().cloned()
    {
        structs.clear();

        for (x, y, z) in points {
            let mut new_point = template_point.clone();

            // Set random location
            if let Property::Struct(StructValue::Vector(vec)) = prop_mut(&mut new_point, "Location")
            {
                vec.x.0 = x;
                vec.y.0 = y;
                vec.z.0 = z;
            }

            // Occasionally create large chamber nodes
            let is_chamber = rand_range(&mut rng_seed, 0.0, 1.0) > 0.88; // ~12% chance
            let (h_mult, v_mult) = if is_chamber {
                let chamber_type = rand_range(&mut rng_seed, 0.0, 1.0);
                if chamber_type < 0.33 {
                    // Wide chamber
                    (rand_range(&mut rng_seed, 2.0, 3.5), 1.0)
                } else if chamber_type < 0.66 {
                    // Tall chamber
                    (1.0, rand_range(&mut rng_seed, 2.0, 3.5))
                } else {
                    // Large chamber (both wide and tall)
                    (
                        rand_range(&mut rng_seed, 1.8, 2.8),
                        rand_range(&mut rng_seed, 1.8, 2.8),
                    )
                }
            } else {
                (1.0, 1.0)
            };

            // Randomize HRange (horizontal tunnel width)
            if let Property::Float(v) = prop_mut(&mut new_point, "HRange") {
                v.0 = rand_range(&mut rng_seed, 150.0, 750.0) * h_mult;
            }

            // Randomize VRange (vertical tunnel height)
            if let Property::Float(v) = prop_mut(&mut new_point, "VRange") {
                v.0 = rand_range(&mut rng_seed, 150.0, 600.0) * v_mult;
            }

            // Randomize CeilingNoiseRange (adds variation to ceiling)
            if let Property::Float(v) = prop_mut(&mut new_point, "CielingNoiseRange") {
                v.0 = rand_range(&mut rng_seed, 0.0, 200.0);
            }

            // Randomize WallNoiseRange (adds variation to walls)
            if let Property::Float(v) = prop_mut(&mut new_point, "WallNoiseRange") {
                v.0 = rand_range(&mut rng_seed, 0.0, 200.0);
            }

            // Randomize FloorNoiseRange (adds variation to floor)
            if let Property::Float(v) = prop_mut(&mut new_point, "FloorNoiseRange") {
                v.0 = rand_range(&mut rng_seed, 0.0, 200.0);
            }

            // Randomize Cielingheight (how tall the ceiling is)
            if let Property::Float(v) = prop_mut(&mut new_point, "Cielingheight") {
                v.0 = rand_range(&mut rng_seed, 200.0, 750.0) * v_mult;
            }

            // Randomize HeightScale (vertical scaling factor)
            if let Property::Float(v) = prop_mut(&mut new_point, "HeightScale") {
                v.0 = rand_range(&mut rng_seed, 0.5, 2.0);
            }

            // Randomize FloorDepth (how deep the floor goes, can be negative)
            if let Property::Float(v) = prop_mut(&mut new_point, "FloorDepth") {
                v.0 = rand_range(&mut rng_seed, 0.0, 0.0);
            }

            // Randomize FloorAngle (tilt of the floor)
            if let Property::Float(v) = prop_mut(&mut new_point, "FloorAngle") {
                v.0 = rand_range(&mut rng_seed, -30.0, 30.0);
            }

            structs.push(StructValue::Struct(new_point));
        }

        println!(
            "Created chaotic random walk with {} points, all parameters randomized",
            structs.len()
        );
    }

    Ok(())
}

fn make_drunk_dwarf(pool: &mut ObjectPool, target: ObjectHandle) -> anyhow::Result<()> {
    // Generate chaotic drunk dwarf path
    let drunk_points = generate_drunk_dwarf_path(
        (0.0, 30000.0, 0.0), // start
        150,                 // num_points
        800.0,               // step_size (how far each stagger)
    );

    println!(
        "\n=== Generated {} drunk dwarf points ===",
        drunk_points.len()
    );

    let object = pool
        .get_mut(target)
        .ok_or_else(|| anyhow::anyhow!("Could not get mutable reference to object"))?;

    let points_prop = prop_mut(object.properties_mut(), "Points");

    if let Property::Array(ValueVec::Struct(structs)) = points_prop
        && let Some(StructValue::Struct(template_point)) = structs.first().cloned()
    {
        structs.clear();
        let _total_points = drunk_points.len();

        for (i, (x, y, z)) in drunk_points.into_iter().enumerate() {
            let mut new_point = template_point.clone();

            // Set location
            if let Property::Struct(StructValue::Vector(vec)) = prop_mut(&mut new_point, "Location")
            {
                vec.x.0 = x;
                vec.y.0 = y;
                vec.z.0 = z;
            }

            // Wild random size variations - the dwarf is very drunk!
            let size_seed = i * 9973;
            let base_h = 400.0 + ((size_seed % 800) as f32);
            let base_v = 400.0 + (((size_seed / 3) % 800) as f32);

            // Occasional "hiccup" - random huge rooms
            let is_hiccup = (i * 7) % 13 == 0;
            let h_range = if is_hiccup { base_h * 3.0 } else { base_h };
            let v_range = if is_hiccup { base_v * 3.0 } else { base_v };

            if let Property::Float(v) = prop_mut(&mut new_point, "HRange") {
                v.0 = h_range;
            }
            if let Property::Float(v) = prop_mut(&mut new_point, "VRange") {
                v.0 = v_range;
            }
            if let Property::Float(v) = prop_mut(&mut new_point, "Cielingheight") {
                v.0 = v_range * 1.2;
            }
            if let Property::Float(v) = prop_mut(&mut new_point, "FloorDepth") {
                v.0 = 0.0;
            }

            structs.push(StructValue::Struct(new_point));
        }

        println!(
            "Created chaotic drunk dwarf tunnel with {} points",
            structs.len()
        );
    }

    Ok(())
}

fn make_circle(pool: &mut ObjectPool, target: ObjectHandle) -> anyhow::Result<()> {
    // Generate circle points
    let circle_points = generate_circle_points(
        (0.0, 30000.0, 0.0), // center
        5000.0,              // radius
        100,                 // num_points
        100.0,               // z_offset (creates slight spiral)
    );

    println!("\n=== Generated {} circle points ===", circle_points.len());

    // Get mutable reference to the object
    let object = pool
        .get_mut(target)
        .ok_or_else(|| anyhow::anyhow!("Could not get mutable reference to object"))?;

    // Replace the Points array with new circle points
    println!("\n=== Replacing Points Array ===");
    let points_prop = prop_mut(object.properties_mut(), "Points");

    if let Property::Array(ValueVec::Struct(structs)) = points_prop
        && let Some(StructValue::Struct(template_point)) = structs.first().cloned()
    {
        structs.clear();
        let total_points = circle_points.len();

        for (i, (x, y, z)) in circle_points.into_iter().enumerate() {
            let mut new_point = template_point.clone();

            // Set location
            if let Property::Struct(StructValue::Vector(vec)) = prop_mut(&mut new_point, "Location")
            {
                vec.x.0 = x;
                vec.y.0 = y;
                vec.z.0 = z;
            }

            // Apply pulsating size effect - breathing cave!
            let (h_range, v_range) = apply_pulsating_size(i, total_points, 800.0, 400.0);

            if let Property::Float(v) = prop_mut(&mut new_point, "HRange") {
                v.0 = h_range as f32;
            }
            if let Property::Float(v) = prop_mut(&mut new_point, "VRange") {
                v.0 = v_range as f32;
            }
            if let Property::Float(v) = prop_mut(&mut new_point, "Cielingheight") {
                v.0 = v_range as f32 * 1.5; // Ceiling height proportional to vertical range
            }

            structs.push(StructValue::Struct(new_point));
        }

        println!(
            "Created {} points with pulsating size effect",
            structs.len()
        );
    }

    Ok(())
}

fn prop_mut<'a>(
    properties: &'a mut uesave::Properties<AssetArchiveType>,
    name: &str,
) -> &'a mut Property<AssetArchiveType> {
    properties
        .0
        .iter_mut()
        .find(|(k, _)| k.1 == name)
        .unwrap()
        .1
}
