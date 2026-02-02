//! Export table builder for asset serialization
//!
//! This module builds the export table for legacy assets, converting
//! LoadedObject instances from the pool into FObjectExport entries.

use crate::core::object_pool::{LoadedObject, ObjectHandle};
use crate::loader::dependency_collector::ExportDependencies;
use crate::saver::object_ref_resolver::ObjectRefResolver;
use anyhow::Result;
use retoc::legacy_asset::FObjectExport;
use retoc::legacy_asset::FPackageNameMap;
use retoc::zen::FPackageIndex;
use std::collections::HashMap;

/// Builder for constructing the export table
#[derive(Debug)]
pub struct ExportBuilder {
    /// List of exports in order
    exports: Vec<ExportEntry>,

    /// Mapping from ObjectHandle to export index
    handle_to_index: HashMap<ObjectHandle, u32>,
}

/// Internal representation of an export with metadata
#[derive(Debug)]
struct ExportEntry {
    /// The export metadata (will be finalized later with offsets)
    export: FObjectExport,

    /// Serialized property data for this export
    serial_data: Vec<u8>,
}

impl ExportBuilder {
    /// Create a new export builder
    pub fn new() -> Self {
        Self {
            exports: Vec::new(),
            handle_to_index: HashMap::new(),
        }
    }

    /// Add an export for an object
    ///
    /// Returns the export index.
    pub fn add_export(
        &mut self,
        handle: ObjectHandle,
        object: &LoadedObject,
        resolver: &ObjectRefResolver,
        name_map: &mut FPackageNameMap,
        serial_data: Vec<u8>,
    ) -> Result<u32> {
        // Check if already added
        if let Some(&index) = self.handle_to_index.get(&handle) {
            return Ok(index);
        }

        let export_index = self.exports.len() as u32;

        // Resolve object references to package indices
        let class_index = resolver.resolve(&object.class)?;

        let template_index = if let Some(ref template_ref) = object.template {
            resolver.resolve(template_ref)?
        } else {
            FPackageIndex { index: 0 }
        };

        let outer_index = if let Some(ref outer_ref) = object.outer {
            resolver.resolve(outer_ref)?
        } else {
            FPackageIndex { index: 0 }
        };

        // Add object name to name map
        // The object name is stored directly now
        let object_name_minimal = name_map.store(object.name.as_str());

        // Create the export (offsets will be filled in later during finalize)
        let export = FObjectExport {
            class_index,
            super_index: FPackageIndex { index: 0 }, // TODO: Add super support
            template_index,
            outer_index,
            object_name: object_name_minimal,
            object_flags: object.object.as_uobject().object_flags.bits(),
            serial_size: serial_data.len() as i64,
            serial_offset: 0, // Will be set during finalize
            is_not_for_client: false,
            is_not_for_server: false,
            is_inherited_instance: false,
            is_not_always_loaded_for_editor_game: true,
            is_asset: false,
            generate_public_hash: false,
            first_export_dependency_index: -1, // Will be set from dependencies
            serialize_before_serialize_dependencies: 0,
            create_before_serialize_dependencies: 0,
            serialize_before_create_dependencies: 0,
            create_before_create_dependencies: 0,
            script_serialization_start_offset: 0,
            script_serialization_end_offset: 0,
        };

        let entry = ExportEntry {
            export,
            serial_data,
        };

        self.exports.push(entry);
        self.handle_to_index.insert(handle, export_index);

        Ok(export_index)
    }

    /// Update dependency metadata for an export
    pub fn update_dependencies(
        &mut self,
        export_index: u32,
        dependencies: &ExportDependencies,
        first_dependency_index: i32,
    ) -> Result<()> {
        let entry = self
            .exports
            .get_mut(export_index as usize)
            .ok_or_else(|| anyhow::anyhow!("Invalid export index: {}", export_index))?;

        entry.export.first_export_dependency_index = first_dependency_index;
        entry.export.serialize_before_serialize_dependencies =
            dependencies.serialize_before_serialize.len() as i32;
        entry.export.create_before_serialize_dependencies =
            dependencies.create_before_serialize.len() as i32;
        entry.export.serialize_before_create_dependencies =
            dependencies.serialize_before_create.len() as i32;
        entry.export.create_before_create_dependencies =
            dependencies.create_before_create.len() as i32;

        Ok(())
    }

    /// Finalize the export table by computing serial offsets
    ///
    /// Returns (export table, concatenated serial data)
    pub fn finalize(self, uasset_size: usize) -> (Vec<FObjectExport>, Vec<u8>) {
        let mut uexp_data = Vec::new();
        let mut exports = Vec::with_capacity(self.exports.len());

        let mut current_offset = uasset_size;

        for entry in self.exports {
            let mut export = entry.export;

            // Set the serial offset (relative to start of concatenated file)
            export.serial_offset = current_offset as i64;

            // Append serial data to uexp
            uexp_data.extend_from_slice(&entry.serial_data);

            current_offset += entry.serial_data.len();

            exports.push(export);
        }

        (exports, uexp_data)
    }

    /// Get the total number of exports
    pub fn len(&self) -> usize {
        self.exports.len()
    }

    /// Check if there are no exports
    pub fn is_empty(&self) -> bool {
        self.exports.is_empty()
    }

    /// Get the export index for a handle
    pub fn get_export_index(&self, handle: ObjectHandle) -> Option<u32> {
        self.handle_to_index.get(&handle).copied()
    }
}

impl Default for ExportBuilder {
    fn default() -> Self {
        Self::new()
    }
}
