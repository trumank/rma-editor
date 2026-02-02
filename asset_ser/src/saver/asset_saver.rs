//! Asset saving functionality
//!
//! This module provides a clean API for saving assets to disk from an object pool.

use crate::AssetVersionInfo;
use crate::core::object_pool::{ObjectHandle, ObjectPool};
use crate::saver::package_writer::PackageWriter;
use anyhow::{Context, Result};
use jmap::Jmap;
use retoc::legacy_asset::FLegacyPackageHeader;
use std::fs;
use std::io::Write;
use std::path::Path;

pub fn save_asset(
    output_path: &Path,
    pool: &ObjectPool,
    root_handles: Vec<ObjectHandle>,
    version: AssetVersionInfo,
    package_name: String,
    jmap: &Jmap,
) -> Result<()> {
    // Prepare the package components
    let writer = PackageWriter::new(pool, version.clone(), package_name.clone(), root_handles);
    let components = writer
        .prepare(jmap)
        .context("Failed to prepare package components")?;

    // Build the complete package header
    let header = FLegacyPackageHeader {
        summary: create_default_summary(&package_name, &version),
        name_map: components.name_map,
        imports: components.imports,
        exports: components.exports,
        preload_dependencies: components.preload_dependencies,
        data_resources: vec![],
        data_resource_version: None,
        cell_imports: vec![],
        cell_exports: vec![],
    };

    // Write .uasset file
    let uasset_file = fs::File::create(output_path)
        .with_context(|| format!("Failed to create .uasset file: {:?}", output_path))?;
    let mut uasset_writer = std::io::BufWriter::new(uasset_file);

    let log = retoc::logging::Log::new_stdout(false, false);
    header
        .serialize(&mut uasset_writer, None, &log)
        .context("Failed to serialize package header")?;

    uasset_writer
        .flush()
        .context("Failed to flush .uasset file")?;

    eprintln!("Wrote .uasset to: {:?}", output_path);

    // Write .uexp file
    let uexp_path = output_path.with_extension("uexp");
    fs::write(&uexp_path, &components.uexp_data)
        .with_context(|| format!("Failed to write .uexp file: {:?}", uexp_path))?;

    eprintln!("Wrote .uexp to: {:?}", uexp_path);

    Ok(())
}

/// Create a default package summary
fn create_default_summary(
    package_name: &str,
    version: &AssetVersionInfo,
) -> retoc::legacy_asset::FLegacyPackageFileSummary {
    retoc::legacy_asset::FLegacyPackageFileSummary {
        versioning_info: retoc::legacy_asset::FLegacyPackageVersioningInfo {
            legacy_file_version: -7,
            package_file_version: retoc::zen::FPackageFileVersion {
                file_version_ue4: version.package_file_version_ue4 as i32,
                file_version_ue5: version.package_file_version_ue5 as i32,
            },
            licensee_version: 0,
            saved_hash: Default::default(),
            custom_versions: Default::default(),
            total_header_size: 0, // Will be calculated during serialization
            is_unversioned: false,
        },
        package_name: package_name.to_string(),
        package_flags: 0x80000000, // PKG_FilterEditorOnly
        names: Default::default(), // Will be populated during serialization
        soft_object_paths: Default::default(),
        exports: Default::default(), // Will be populated during serialization
        imports: Default::default(), // Will be populated during serialization
        cell_exports: Default::default(),
        cell_imports: Default::default(),
        depends_offset: -1,
        package_guid: Default::default(),
        package_source: 0,
        world_tile_info_data_offset: 0,
        chunk_ids: vec![],
        preload_dependencies: Default::default(), // Will be populated during serialization
        names_referenced_from_export_data_count: 0,
        data_resource_offset: 0,
        asset_registry_data_offset: 0,
        bulk_data_start_offset: -1,
    }
}
