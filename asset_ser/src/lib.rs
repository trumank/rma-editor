use anyhow::{Context as _, Result};
use retoc::legacy_asset::FLegacyPackageHeader;
use retoc::zen::FPackageIndex;
use std::fs;
use std::io::Cursor;
use std::path::Path;
use uesave::VersionInfo;

pub mod archive;
pub mod core;
pub mod loader;
pub mod object;
pub mod saver;
pub mod util;

/// Get the full path name for an FPackageIndex
///
/// Traverses the outer chain to build the complete object path.
/// For example: "Package.OuterClass.InnerObject"
pub fn get_package_index_path(
    header: &FLegacyPackageHeader,
    mut package_index: FPackageIndex,
) -> Result<core::object_path::ObjectPath> {
    if package_index.is_null() {
        return Ok("None".into());
    }

    let mut path_components = Vec::new();

    // Start with the initial object
    loop {
        if package_index.is_null() {
            break;
        } else if package_index.is_export() {
            // Export reference
            let export_idx = package_index.to_export_index() as usize;
            if export_idx >= header.exports.len() {
                path_components.push("INVALID_EXPORT".to_string());
                break;
            }
            let export = &header.exports[export_idx];
            path_components.push(header.name_map.get(export.object_name)?.to_string());
            package_index = export.outer_index;
        } else {
            // Import reference
            let import_idx = package_index.to_import_index() as usize;
            if import_idx >= header.imports.len() {
                path_components.push("INVALID_IMPORT".to_string());
                break;
            }
            let import = &header.imports[import_idx];
            path_components.push(header.name_map.get(import.object_name)?.to_string());
            package_index = import.outer_index;
        }
    }

    path_components.reverse();
    Ok(path_components.join(".").into())
}

/// Parse a UE4/UE5 legacy asset (.uasset + .uexp)
pub fn parse_legacy_asset(uasset_path: &Path) -> Result<FLegacyPackageHeader> {
    // Read the .uasset file
    let uasset_data = fs::read(uasset_path)
        .with_context(|| format!("Failed to read .uasset file: {:?}", uasset_path))?;

    // Try to read the .uexp file (exports data)
    let uexp_path = uasset_path.with_extension("uexp");
    let _uexp_data = fs::read(&uexp_path)
        .with_context(|| format!("Failed to read .uexp file: {:?}", uexp_path))?;

    // Parse the header from the .uasset file
    let mut cursor = Cursor::new(&uasset_data);

    // For UE 4.27 legacy assets, we need to provide a package version fallback
    // since the asset might be unversioned
    let package_version_fallback = Some(retoc::zen::FPackageFileVersion {
        file_version_ue4: 522, // UE 4.27
        file_version_ue5: 0,
    });

    let header = FLegacyPackageHeader::deserialize(&mut cursor, package_version_fallback)
        .context("Failed to deserialize legacy package header")?;

    Ok(header)
}

#[derive(Debug, Clone)]
pub struct AssetVersionInfo {
    pub package_file_version_ue4: u32,
    pub package_file_version_ue5: u32,
    pub engine_version_major: u16,
    pub engine_version_minor: u16,
    pub engine_version_patch: u16,
}

impl AssetVersionInfo {
    pub fn from_package_header(header: &FLegacyPackageHeader) -> Self {
        let ver = &header.summary.versioning_info.package_file_version;
        Self {
            package_file_version_ue4: ver.file_version_ue4 as u32,
            package_file_version_ue5: ver.file_version_ue5 as u32,
            // TODO deduce engine version from package file version and/or make configurable
            engine_version_major: 4,
            engine_version_minor: 27,
            engine_version_patch: 0,
        }
    }
}

impl VersionInfo for AssetVersionInfo {
    fn engine_version_major(&self) -> u16 {
        self.engine_version_major
    }
    fn engine_version_minor(&self) -> u16 {
        self.engine_version_minor
    }
    fn engine_version_patch(&self) -> u16 {
        self.engine_version_patch
    }
    fn package_file_version_ue4(&self) -> u32 {
        self.package_file_version_ue4
    }
    fn package_file_version_ue5(&self) -> u32 {
        self.package_file_version_ue5
    }
}
