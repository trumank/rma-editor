//! Integration tests for round-trip asset serialization
//!
//! These tests load assets into a pool, then rebuild the import/export tables
//! and compare against the original asset structure.

use asset_ser::{
    AssetVersionInfo,
    core::object_pool::{ObjectPool, ObjectRef},
    loader::asset_loader,
    parse_legacy_asset,
    saver::{asset_saver, package_writer::PackageWriter},
};
use jmap::Jmap;
use retoc::legacy_asset::FPackageNameMap;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Test full round-trip: load asset, rebuild components, verify structure
#[test]
fn test_full_round_trip_structure() -> anyhow::Result<()> {
    let name = "RMA_WallPlatforms.uasset";
    // let name= "RMA_2PArcsSPAWNER.uasset";

    let asset_path = Path::new("test_assets").join(name);
    let jmap_path = Path::new("fsd.jmap");

    // Load jmap
    let jmap_data = fs::read_to_string(jmap_path)?;
    let jmap: Jmap = serde_json::from_str(&jmap_data)?;

    // Load the original asset
    let header = parse_legacy_asset(&asset_path)?;

    println!("=== Original Asset ===");
    println!("Imports: {}", header.imports.len());
    println!("Exports: {}", header.exports.len());
    println!("Preload deps: {}", header.preload_dependencies.len());
    println!("Names: {}", header.name_map.num_names());

    // Load asset into pool
    let mut pool = ObjectPool::new();
    let root = asset_loader::load_asset(&asset_path, &mut pool)
        .map_err(|e| anyhow::anyhow!("Failed to load asset: {}", e))?;

    println!("\n=== Loaded Pool ===");
    println!("Objects: {}", pool.len());

    // Rebuild using PackageWriter
    let version = AssetVersionInfo::from_package_header(&header);
    let writer = PackageWriter::new(
        &pool,
        version,
        header.summary.package_name.clone(),
        vec![root],
    );

    let components = writer.prepare(&jmap)?;

    println!("\n=== Rebuilt Components ===");
    println!("Imports: {}", components.imports.len());
    println!("Exports: {}", components.exports.len());
    println!("Preload deps: {}", components.preload_dependencies.len());
    println!("Names: {}", components.name_map.num_names());
    println!("UEXP size: {} bytes", components.uexp_data.len());

    // Verify import contents
    println!("\n=== Verifying Import Contents ===");
    println!("Original imports: {}", header.imports.len());
    println!("Rebuilt imports: {}", components.imports.len());

    // Print all original imports
    println!("\n--- Original Imports ---");
    let mut import_path_set = std::collections::HashSet::new();
    for (i, import) in header.imports.iter().enumerate() {
        let path = asset_ser::get_package_index_path(
            &header,
            retoc::zen::FPackageIndex::create_import(i as u32),
        )?;
        let class_name = header.name_map.get(import.class_name)?;

        println!(
            "Original Import {}: {} (class: {}, outer_idx: {})",
            -1 - i as isize,
            path,
            class_name,
            import.outer_index.index
        );
        import_path_set.insert(path.clone());
    }

    // Print all rebuilt imports
    // Build temporary header structure for path resolution
    let temp_header = retoc::legacy_asset::FLegacyPackageHeader {
        imports: components.imports.clone(),
        exports: components.exports.clone(),
        name_map: components.name_map.clone(),
        ..header.clone()
    };

    println!("\n--- Rebuilt Imports ---");
    for (i, _import) in components.imports.iter().enumerate() {
        let path = asset_ser::get_package_index_path(
            &temp_header,
            retoc::zen::FPackageIndex::create_import(i as u32),
        )?;
        let class_name = components.name_map.get(_import.class_name)?;

        println!(
            "Rebuilt Import {}: {} (class: {}, outer_idx: {})",
            -1 - i as isize,
            path,
            class_name,
            _import.outer_index.index
        );
    }

    // Verify export count matches
    assert_eq!(
        components.exports.len(),
        header.exports.len(),
        "Export count should match"
    );

    // Verify export contents
    println!("\n=== Verifying Export Contents ===");
    println!("Original exports: {}", header.exports.len());
    println!("Rebuilt exports: {}", components.exports.len());

    // Print all exports first
    for (i, (original, rebuilt)) in header.exports.iter().zip(&components.exports).enumerate() {
        let original_name = header.name_map.get(original.object_name)?;
        let rebuilt_name = components.name_map.get(rebuilt.object_name)?;
        let original_class = asset_ser::get_package_index_path(&header, original.class_index)?;

        let name_match = if original_name == rebuilt_name {
            "✓"
        } else {
            "✗"
        };
        let class_match = if original.class_index == rebuilt.class_index {
            "✓"
        } else {
            "✗"
        };

        println!(
            "Export {}: Original='{}' ({}) class_idx={} | Rebuilt='{}' class_idx={} | Name:{} Class:{}",
            1 + i,
            original_name,
            original_class,
            original.class_index.index,
            rebuilt_name,
            rebuilt.class_index.index,
            name_match,
            class_match
        );
    }

    // Verify preload dependencies structure
    println!("\n=== Verifying Preload Dependencies ===");
    println!(
        "Original preload deps: {}",
        header.preload_dependencies.len()
    );
    println!(
        "Rebuilt preload deps: {}",
        components.preload_dependencies.len()
    );

    let mut matching_exports = 0;
    let mut mismatching_exports = 0;

    // Print all dependency information first
    for (export_idx, (original_export, rebuilt_export)) in
        header.exports.iter().zip(&components.exports).enumerate()
    {
        let object_name = header.name_map.get(original_export.object_name)?;

        let original_sbs = original_export.serialize_before_serialize_dependencies as usize;
        let original_cbs = original_export.create_before_serialize_dependencies as usize;
        let original_sbc = original_export.serialize_before_create_dependencies as usize;
        let original_cbc = original_export.create_before_create_dependencies as usize;

        let rebuilt_sbs = rebuilt_export.serialize_before_serialize_dependencies as usize;
        let rebuilt_cbs = rebuilt_export.create_before_serialize_dependencies as usize;
        let rebuilt_sbc = rebuilt_export.serialize_before_create_dependencies as usize;
        let rebuilt_cbc = rebuilt_export.create_before_create_dependencies as usize;

        let has_orig_deps = original_export.first_export_dependency_index >= 0;
        let has_rebuilt_deps = rebuilt_export.first_export_dependency_index >= 0;

        // Check if dependency contents match (not just counts)
        let deps_match = if !has_orig_deps && !has_rebuilt_deps {
            true
        } else if has_orig_deps != has_rebuilt_deps {
            false
        } else {
            let orig_first_idx = original_export.first_export_dependency_index as usize;
            let rebuilt_first_idx = rebuilt_export.first_export_dependency_index as usize;

            let total_orig = original_sbs + original_cbs + original_sbc + original_cbc;
            let total_rebuilt = rebuilt_sbs + rebuilt_cbs + rebuilt_sbc + rebuilt_cbc;

            if total_orig != total_rebuilt {
                false
            } else {
                // Compare actual dependency arrays
                let orig_deps =
                    &header.preload_dependencies[orig_first_idx..orig_first_idx + total_orig];
                let rebuilt_deps = &components.preload_dependencies
                    [rebuilt_first_idx..rebuilt_first_idx + total_rebuilt];

                orig_deps == rebuilt_deps
                    && original_sbs == rebuilt_sbs
                    && original_cbs == rebuilt_cbs
                    && original_sbc == rebuilt_sbc
                    && original_cbc == rebuilt_cbc
            }
        };

        let match_status = if !has_orig_deps && !has_rebuilt_deps {
            "No deps"
        } else if has_orig_deps != has_rebuilt_deps {
            "✗ PRESENCE MISMATCH"
        } else if deps_match {
            "✓"
        } else {
            "✗ CONTENT MISMATCH"
        };

        println!(
            "Export {} ({}): Original(first_idx={}, SbS:{}, CbS:{}, SbC:{}, CbC:{}) | Rebuilt(first_idx={}, SbS:{}, CbS:{}, SbC:{}, CbC:{}) | {}",
            export_idx,
            object_name,
            original_export.first_export_dependency_index,
            original_sbs,
            original_cbs,
            original_sbc,
            original_cbc,
            rebuilt_export.first_export_dependency_index,
            rebuilt_sbs,
            rebuilt_cbs,
            rebuilt_sbc,
            rebuilt_cbc,
            match_status
        );

        // Track statistics and show detailed mismatch info
        if has_orig_deps && has_rebuilt_deps {
            if deps_match {
                matching_exports += 1;
            } else {
                mismatching_exports += 1;

                // Show detailed dependency differences
                let orig_first_idx = original_export.first_export_dependency_index as usize;
                let rebuilt_first_idx = rebuilt_export.first_export_dependency_index as usize;

                // Helper to print dependency array
                let print_deps =
                    |label: &str,
                     deps: &[retoc::zen::FPackageIndex],
                     header: &retoc::legacy_asset::FLegacyPackageHeader| {
                        print!("    {} ({}): [", label, deps.len());
                        for (i, dep) in deps.iter().enumerate() {
                            if i > 0 {
                                print!(", ");
                            }
                            let path = asset_ser::get_package_index_path(header, *dep)
                                .unwrap_or_else(|_| "INVALID".into());
                            print!("{} ({})", dep.index, path);
                        }
                        println!("]");
                    };

                // Helper to check and print a specific dependency category
                let check_category = |name: &str,
                                      orig_offset: usize,
                                      orig_count: usize,
                                      rebuilt_offset: usize,
                                      rebuilt_count: usize| {
                    let orig_slice =
                        &header.preload_dependencies[orig_offset..orig_offset + orig_count];
                    let rebuilt_slice = &components.preload_dependencies
                        [rebuilt_offset..rebuilt_offset + rebuilt_count];

                    if orig_count != rebuilt_count || orig_slice != rebuilt_slice {
                        println!("  {} mismatch:", name);
                        print_deps("Original", orig_slice, &header);
                        print_deps("Rebuilt", rebuilt_slice, &temp_header);
                    }
                };

                // Check each dependency category
                check_category(
                    "SerializeBeforeSerialize",
                    orig_first_idx,
                    original_sbs,
                    rebuilt_first_idx,
                    rebuilt_sbs,
                );
                check_category(
                    "CreateBeforeSerialization",
                    orig_first_idx + original_sbs,
                    original_cbs,
                    rebuilt_first_idx + rebuilt_sbs,
                    rebuilt_cbs,
                );
                check_category(
                    "SerializeBeforeCreate",
                    orig_first_idx + original_sbs + original_cbs,
                    original_sbc,
                    rebuilt_first_idx + rebuilt_sbs + rebuilt_cbs,
                    rebuilt_sbc,
                );
                check_category(
                    "CreateBeforeCreate",
                    orig_first_idx + original_sbs + original_cbs + original_sbc,
                    original_cbc,
                    rebuilt_first_idx + rebuilt_sbs + rebuilt_cbs + rebuilt_sbc,
                    rebuilt_cbc,
                );
            }
        }
    }

    println!("\n--- Dependency Summary ---");
    println!("Matching dependency structures: {}", matching_exports);
    println!("Mismatching dependency structures: {}", mismatching_exports);

    if matching_exports + mismatching_exports > 0 {
        println!(
            "Dependency match rate: {:.1}%",
            (matching_exports as f64 / (matching_exports + mismatching_exports) as f64) * 100.0
        );
    }

    // Now do assertions
    for (i, (original, rebuilt)) in header.exports.iter().zip(&components.exports).enumerate() {
        let original_name = header.name_map.get(original.object_name)?;
        let rebuilt_name = components.name_map.get(rebuilt.object_name)?;

        assert_eq!(
            original_name,
            rebuilt_name,
            "Export {} name mismatch: '{}' vs '{}'",
            1 + i,
            original_name,
            rebuilt_name
        );

        assert_eq!(
            original.class_index,
            rebuilt.class_index,
            "Export {} class_index mismatch",
            1 + i
        );
    }

    // Verify name map contents
    println!("\n=== Verifying Name Map Contents ===");
    println!("Original name count: {}", header.name_map.num_names());
    println!("Rebuilt name count: {}", components.name_map.num_names());

    // Verify that common names can be looked up
    // Sample some names from exports to verify they exist in both
    let mut successfully_verified = 0;
    for export in header.exports.iter().take(10) {
        let name_in_original = header.name_map.get(export.object_name)?;
        if let Ok(name_in_rebuilt) = components.name_map.get(export.object_name)
            && name_in_original == name_in_rebuilt
        {
            successfully_verified += 1;
        }
    }

    println!(
        "Successfully verified {} common export names",
        successfully_verified
    );

    // Verify that the rebuilt name map has a reasonable size
    assert!(
        components.name_map.num_names() > 0,
        "Rebuilt name map should not be empty"
    );
    assert!(
        components.name_map.num_names() < header.name_map.num_names() * 3,
        "Rebuilt name map should not be excessively large"
    );

    // Now do assertions
    for (export_idx, (original_export, rebuilt_export)) in
        header.exports.iter().zip(&components.exports).enumerate()
    {
        if original_export.first_export_dependency_index < 0 {
            assert!(
                rebuilt_export.first_export_dependency_index < 0,
                "Export {} dependency presence mismatch",
                export_idx
            );
        }
    }

    // Verify UEXP data was generated
    assert!(
        !components.uexp_data.is_empty(),
        "Should have serialized export data"
    );

    println!("\n✓ Round-trip content verification passed");

    // Write the rebuilt asset to disk using the clean API
    println!("\n=== Writing Rebuilt Asset ===");

    // First, we need to get the pool back from the writer
    // Since we already consumed it, let's reload the asset
    let mut pool2 = ObjectPool::new();
    let root2 = asset_loader::load_asset(&asset_path, &mut pool2)
        .map_err(|e| anyhow::anyhow!("Failed to reload asset: {}", e))?;

    let output_path = Path::new("tmp").join(name);
    let version2 = AssetVersionInfo::from_package_header(&header);

    asset_saver::save_asset(
        &output_path,
        &pool2,
        vec![root2],
        version2,
        header.summary.package_name.clone(),
        &jmap,
    )?;

    println!("\n✓ Rebuilt asset written successfully");

    Ok(())
}

/// Test that object references are properly resolved
#[test]
fn test_object_reference_resolution() -> anyhow::Result<()> {
    let asset_path = Path::new("test_assets/RMA_WallPlatforms.uasset");

    // Load asset into pool
    let mut pool = ObjectPool::new();
    asset_loader::load_asset(asset_path, &mut pool)
        .map_err(|e| anyhow::anyhow!("Failed to load asset: {}", e))?;

    // Build a map of paths to handles
    let mut path_to_handle = HashMap::new();
    for (handle, _object) in pool.iter() {
        let path = pool.build_path(handle);
        path_to_handle.insert(path.to_string(), handle);
    }

    // Verify that objects with outer references can be resolved
    let mut resolved_count = 0;
    let mut unresolved_count = 0;

    for (_handle, object) in pool.iter() {
        // Check if outer can be resolved
        if let Some(ref outer_ref) = object.outer {
            match outer_ref {
                ObjectRef::Loaded(_) => {
                    resolved_count += 1;
                }
                ObjectRef::Unloaded(outer_path) => {
                    if pool.find_by_path(outer_path).is_some() {
                        resolved_count += 1;
                    } else {
                        // Outer is probably an import, not in the pool
                        unresolved_count += 1;
                    }
                }
            }
        }

        // Check if class can be resolved
        match &object.class {
            ObjectRef::Loaded(_) => {
                resolved_count += 1;
            }
            ObjectRef::Unloaded(class_path) => {
                if pool.find_by_path(class_path).is_some() {
                    resolved_count += 1;
                } else {
                    // Class is probably an import
                    unresolved_count += 1;
                }
            }
        }
    }

    println!("Resolved references: {}", resolved_count);
    println!("Unresolved references (imports): {}", unresolved_count);

    // We expect most class/outer references to be imports
    assert!(unresolved_count > 0, "Should have some import references");

    Ok(())
}

/// Test name map deduplication
#[test]
fn test_name_map_deduplication() -> anyhow::Result<()> {
    let asset_path = Path::new("test_assets/RMA_WallPlatforms.uasset");

    // Load the original asset
    let header = parse_legacy_asset(asset_path)?;
    let original_name_count = header.name_map.num_names();

    println!("Original name count: {}", original_name_count);

    // Load asset into pool and rebuild name map
    let mut pool = ObjectPool::new();
    asset_loader::load_asset(asset_path, &mut pool)
        .map_err(|e| anyhow::anyhow!("Failed to load asset: {}", e))?;

    let mut name_map = FPackageNameMap::create();

    // Add all object names
    for (handle, object) in pool.iter() {
        // Add the object's own name
        let path = pool.build_path(handle);
        let obj_name = path
            .as_str()
            .split('.')
            .next_back()
            .unwrap_or(path.as_str());
        name_map.store(obj_name);

        // Add metadata names from class
        match &object.class {
            ObjectRef::Loaded(_) => {
                // Skip loaded refs for now
            }
            ObjectRef::Unloaded(class_path) => {
                for component in class_path.as_str().split('.') {
                    name_map.store(component);
                }
            }
        }

        // Add metadata names from template
        if let Some(template_ref) = &object.template {
            match template_ref {
                ObjectRef::Loaded(_) => {
                    // Skip loaded refs for now
                }
                ObjectRef::Unloaded(template_path) => {
                    for component in template_path.as_str().split('.') {
                        name_map.store(component);
                    }
                }
            }
        }

        // Add metadata names from outer
        if let Some(outer_ref) = &object.outer {
            match outer_ref {
                ObjectRef::Loaded(_) => {
                    // Skip loaded refs for now
                }
                ObjectRef::Unloaded(outer_path) => {
                    for component in outer_path.as_str().split('.') {
                        name_map.store(component);
                    }
                }
            }
        }
    }

    let rebuilt_name_count = name_map.num_names();
    println!("Rebuilt name count: {}", rebuilt_name_count);

    // We might have different counts due to different collection methods,
    // but should have a reasonable number of names
    assert!(rebuilt_name_count > 0, "Should have collected some names");
    assert!(
        rebuilt_name_count < original_name_count * 2,
        "Shouldn't have excessive name duplication"
    );

    println!(
        "Name count ratio: {:.2}",
        rebuilt_name_count as f64 / original_name_count as f64
    );

    Ok(())
}
