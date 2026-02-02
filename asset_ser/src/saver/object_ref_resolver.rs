//! Object reference resolution for asset serialization
//!
//! This module converts ObjectRef (used in the pool) back to FPackageIndex
//! (used in serialized assets), classifying references as imports or exports.

use crate::core::object_path::ObjectPath;
use crate::core::object_pool::{ObjectHandle, ObjectRef};
use anyhow::Result;
use retoc::zen::FPackageIndex;
use std::collections::HashMap;

/// Resolver for converting ObjectRef to FPackageIndex
///
/// Maintains bidirectional mappings between ObjectRef and FPackageIndex,
/// classifying references as either exports (same package) or imports (external).
#[derive(Debug)]
pub struct ObjectRefResolver {
    /// Mapping from ObjectRef to FPackageIndex
    ref_to_index: HashMap<ObjectRef, FPackageIndex>,

    /// Mapping from ObjectHandle to export index
    handle_to_export_index: HashMap<ObjectHandle, u32>,

    /// Mapping from import path to import index
    path_to_import_index: HashMap<String, u32>,
}

impl Default for ObjectRefResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ObjectRefResolver {
    /// Create a new empty resolver
    pub fn new() -> Self {
        Self {
            ref_to_index: HashMap::new(),
            handle_to_export_index: HashMap::new(),
            path_to_import_index: HashMap::new(),
        }
    }

    /// Register an export mapping with its path
    ///
    /// Associates both an ObjectHandle and its path with an export index.
    /// This allows the export to be resolved by either handle or path.
    pub fn register_export_with_path(
        &mut self,
        handle: ObjectHandle,
        path: impl Into<ObjectPath>,
        export_index: u32,
    ) {
        self.handle_to_export_index.insert(handle, export_index);

        // Create the FPackageIndex for this export
        let pkg_index = FPackageIndex::create_export(export_index);
        let path = path.into();

        // Register both the handle and path variants
        self.ref_to_index
            .insert(ObjectRef::Loaded(handle), pkg_index);
        self.path_to_import_index
            .insert(path.as_str().to_string(), export_index);
        self.ref_to_index
            .insert(ObjectRef::Unloaded(path), pkg_index);
    }

    /// Register an import mapping
    ///
    /// Associates an object path with an import index.
    pub fn register_import(&mut self, path: impl Into<ObjectPath>, import_index: u32) {
        let path = path.into();
        self.path_to_import_index
            .insert(path.as_str().to_string(), import_index);

        // Create the FPackageIndex for this import
        let pkg_index = FPackageIndex::create_import(import_index);
        self.ref_to_index
            .insert(ObjectRef::Unloaded(path), pkg_index);
    }

    /// Resolve an ObjectRef to FPackageIndex
    ///
    /// Returns an error if the reference hasn't been registered.
    pub fn resolve(&self, object_ref: &ObjectRef) -> Result<FPackageIndex> {
        self.ref_to_index.get(object_ref).copied().ok_or_else(|| {
            anyhow::anyhow!("ObjectRef not registered in resolver: {:?}", object_ref)
        })
    }

    /// Get the import index for a path
    pub fn get_import_index(&self, path: &str) -> Option<u32> {
        self.path_to_import_index.get(path).copied()
    }
}
