//! Asset loading with dependency resolution
//!
//! This module provides functionality to load Unreal Engine assets
//! and resolve their dependency chains, loading exports in the correct order.

use crate::archive::reader::AssetArchiveReader;
use crate::core::name::Name;
use crate::core::object_path::ObjectPath;
use crate::core::object_pool::{LoadedObject, ObjectHandle, ObjectPool, ObjectRef};
use crate::loader::dependency_collector::ExportDependencies;
use crate::object::{Error, ObjectType, Result, UClass, UFunction, UObject, UStruct};
use crate::parse_legacy_asset;
use retoc::legacy_asset::FLegacyPackageHeader;
use retoc::zen::FPackageIndex;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::io::Cursor;
use std::path::Path;

/// Context for loading an asset file
pub struct AssetLoadContext<'a> {
    /// The package header
    pub header: FLegacyPackageHeader,

    /// The .uexp file data
    pub uexp_data: Vec<u8>,

    /// Size of the .uasset file (for offset calculations)
    pub uasset_size: usize,

    /// Object pool for loaded objects
    pub pool: &'a mut ObjectPool,

    /// Mapping from export index to object handle
    export_to_handle: HashMap<usize, ObjectHandle>,
}

impl<'a> AssetLoadContext<'a> {
    /// Create a new asset load context
    pub fn new(asset_path: &Path, pool: &'a mut ObjectPool) -> Result<Self> {
        // Read the .uasset file
        let uasset_data = fs::read(asset_path)
            .map_err(|e| Error::Other(format!("Failed to read .uasset: {}", e)))?;
        let uasset_size = uasset_data.len();

        // Parse the asset header
        let header = parse_legacy_asset(asset_path)
            .map_err(|e| Error::Other(format!("Failed to parse asset header: {}", e)))?;

        // Read the .uexp file
        let uexp_path = asset_path.with_extension("uexp");
        let uexp_data = fs::read(&uexp_path)
            .map_err(|e| Error::Other(format!("Failed to read .uexp: {}", e)))?;

        Ok(Self {
            header,
            uexp_data,
            uasset_size,
            pool,
            export_to_handle: HashMap::new(),
        })
    }

    /// Find the root export (the one with null outer)
    pub fn find_root_export(&self) -> Option<usize> {
        self.header
            .exports
            .iter()
            .position(|export| export.outer_index.is_null())
    }

    /// Find all root exports (all exports with null outer)
    pub fn find_all_root_exports(&self) -> Vec<usize> {
        self.header
            .exports
            .iter()
            .enumerate()
            .filter_map(|(idx, export)| {
                if export.outer_index.is_null() {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get the full path for a package index
    pub fn get_package_path(&self, pkg_idx: FPackageIndex) -> Result<ObjectPath> {
        crate::get_package_index_path(&self.header, pkg_idx)
            .map_err(|e| Error::Other(e.to_string()))
    }

    /// Resolve a FPackageIndex to an ObjectRef
    /// If it's an export, return ObjectRef::Loaded if created, otherwise Unloaded with path
    /// If it's an import, return ObjectRef::Unloaded with path
    fn resolve_package_index_to_ref(&self, pkg_idx: FPackageIndex) -> Result<ObjectRef> {
        if pkg_idx.is_export() {
            let export_idx = pkg_idx.to_export_index() as usize;
            if let Some(&handle) = self.export_to_handle.get(&export_idx) {
                Ok(ObjectRef::Loaded(handle))
            } else {
                // Not yet created, use path
                let path = self.get_package_path(pkg_idx)?;
                Ok(ObjectRef::Unloaded(path))
            }
        } else {
            // Import or other - use path
            let path = self.get_package_path(pkg_idx)?;
            Ok(ObjectRef::Unloaded(path))
        }
    }

    /// Create an export object (allocate with empty properties)
    fn create_export(&mut self, export_idx: usize) -> Result<ObjectHandle> {
        // Check if already created
        if let Some(&handle) = self.export_to_handle.get(&export_idx) {
            // eprintln!(
            //     "  Export {} already created (handle {:?})",
            //     export_idx, handle
            // );
            return Ok(handle);
        }

        let export = &self.header.exports[export_idx];

        // Get object name from name map
        let object_name = self
            .header
            .name_map
            .get(export.object_name)
            .map_err(|e| Error::Other(e.to_string()))?;

        // eprintln!(
        //     "Creating export {}: {} (pool has {} objects)",
        //     export_idx,
        //     object_name,
        //     self.pool.len()
        // );

        // Resolve class to ObjectRef
        let class_ref = self.resolve_package_index_to_ref(export.class_index)?;

        // Resolve template to ObjectRef
        let template_ref = if !export.template_index.is_null() {
            Some(self.resolve_package_index_to_ref(export.template_index)?)
        } else {
            None
        };

        // Resolve outer to ObjectRef
        let outer_ref = if !export.outer_index.is_null() {
            Some(self.resolve_package_index_to_ref(export.outer_index)?)
        } else {
            None
        };

        // Create loaded object
        let loaded_obj = LoadedObject {
            name: Name::new(object_name),
            outer: outer_ref,
            class: class_ref,
            template: template_ref,
            object: Box::new(UObject::default()), // temp, will be replaced
        };

        // Allocate in pool
        let handle = self.pool.allocate(loaded_obj);

        // Track export_idx -> handle mapping
        self.export_to_handle.insert(export_idx, handle);

        Ok(handle)
    }

    /// Walk up the class chain and create the appropriate object type
    fn create_object_for_export(
        &self,
        export: &FLegacyPackageHeader,
        export_idx: usize,
    ) -> Result<Box<dyn ObjectType>> {
        let class_pkg_idx = export.exports[export_idx].class_index;

        let object_flags =
            jmap::EObjectFlags::from_bits(export.exports[export_idx].object_flags).unwrap();

        let class_path = self.get_package_path(class_pkg_idx)?;

        let mut obj: Box<dyn ObjectType> = match class_path.as_str() {
            "/Script/CoreUObject.Class"
            | "/Script/Engine.BlueprintGeneratedClass"
            | "/Script/UMG.WidgetBlueprintGeneratedClass" => Box::new(UClass::default()),
            "/Script/CoreUObject.Function" => Box::new(UFunction::default()),
            "/Script/CoreUObject.Struct" => Box::new(UStruct::default()),
            _ => Box::new(UObject::default()),
        };

        obj.as_uobject_mut().object_flags = object_flags;

        Ok(obj)
    }

    /// Serialize an export's properties (must be created first)
    fn serialize_export(&mut self, export_idx: usize) -> Result<()> {
        // Look up the handle for this export
        let handle = *self.export_to_handle.get(&export_idx).ok_or_else(|| {
            Error::Other(format!(
                "Export {} not created before serialization",
                export_idx
            ))
        })?;

        let export = &self.header.exports[export_idx];
        let object_name = self
            .header
            .name_map
            .get(export.object_name)
            .map_err(|e| Error::Other(e.to_string()))?;

        // eprintln!(
        //     "Serializing export {}: {} (pool has {} objects)",
        //     export_idx,
        //     object_name,
        //     self.pool.len()
        // );

        // Extract export data
        let export_start = (export.serial_offset as usize) - self.uasset_size;
        let export_end = export_start + export.serial_size as usize;

        if export_end > self.uexp_data.len() {
            return Err(Error::Other(format!(
                "Export {} data exceeds .uexp file size",
                export_idx
            )));
        }

        let export_data = &self.uexp_data[export_start..export_end];

        let deserialized_object = {
            let mut archive =
                AssetArchiveReader::new(Cursor::new(export_data), &self.header, self.pool);

            // Create object type by walking the class chain
            let mut object_type = self.create_object_for_export(&self.header, export_idx)?;

            // Deserialize into the object
            object_type.de(&mut archive)?;

            // println!(
            //     "Export has {} bytes of extra data",
            //     export_data.len() as u64 - archive.stream_position().unwrap()
            // );

            // let extra = &export_data[archive.stream_position().unwrap() as usize..];
            // for l in extra.chunks(16) {
            //     println!("{l:02x?}");
            // }

            object_type
        };

        // Now archive is dropped, we can get mutable reference
        let obj = self.pool.get_mut(handle).ok_or_else(|| {
            Error::Other(format!(
                "Failed to get mutable reference to export {}",
                export_idx
            ))
        })?;

        // Replace the object type with the deserialized one
        obj.object = deserialized_object;

        Ok(())
    }

    /// Compute the create and serialize order for an export using topological sort
    ///
    /// Returns (create_order, serialize_order)
    fn compute_load_order(&self, root_export_idx: usize) -> Result<(Vec<usize>, Vec<usize>)> {
        let mut visited = HashSet::new();
        let mut create_order = Vec::new();
        let mut serialize_order = Vec::new();

        self.visit_export_dependencies(
            root_export_idx,
            &mut visited,
            &mut create_order,
            &mut serialize_order,
        )?;

        Ok((create_order, serialize_order))
    }

    /// Visit an export and its dependencies (DFS) with two-phase ordering
    ///
    /// Builds two separate orderings:
    /// - create_order: When objects should be allocated (stub with empty properties)
    /// - serialize_order: When object properties should be deserialized
    fn visit_export_dependencies(
        &self,
        export_idx: usize,
        visited: &mut HashSet<usize>,
        create_order: &mut Vec<usize>,
        serialize_order: &mut Vec<usize>,
    ) -> Result<()> {
        if visited.contains(&export_idx) {
            return Ok(());
        }

        visited.insert(export_idx);

        let export = &self.header.exports[export_idx];

        // Read dependencies directly from header
        let deps = if export.first_export_dependency_index >= 0 {
            let first_idx = export.first_export_dependency_index as usize;
            let sbs_count = export.serialize_before_serialize_dependencies as usize;
            let cbs_count = export.create_before_serialize_dependencies as usize;
            let sbc_count = export.serialize_before_create_dependencies as usize;
            let cbc_count = export.create_before_create_dependencies as usize;

            ExportDependencies {
                serialize_before_serialize: self.header.preload_dependencies
                    [first_idx..first_idx + sbs_count]
                    .to_vec(),
                create_before_serialize: self.header.preload_dependencies
                    [first_idx + sbs_count..first_idx + sbs_count + cbs_count]
                    .to_vec(),
                serialize_before_create: self.header.preload_dependencies[first_idx
                    + sbs_count
                    + cbs_count
                    ..first_idx + sbs_count + cbs_count + sbc_count]
                    .to_vec(),
                create_before_create: self.header.preload_dependencies[first_idx
                    + sbs_count
                    + cbs_count
                    + sbc_count
                    ..first_idx + sbs_count + cbs_count + sbc_count + cbc_count]
                    .to_vec(),
            }
        } else {
            ExportDependencies::default()
        };

        // PHASE 1: CREATE ORDERING
        // Objects that must be created before this object can be created

        // CreateBeforeCreate - must be created first
        for pkg_idx in &deps.create_before_create {
            if let Some(dep_export_idx) = self.package_index_to_export(*pkg_idx) {
                self.visit_export_dependencies(
                    dep_export_idx,
                    visited,
                    create_order,
                    serialize_order,
                )?;
            }
        }

        // SerializeBeforeCreate - must be fully serialized before we can create this
        for pkg_idx in &deps.serialize_before_create {
            if let Some(dep_export_idx) = self.package_index_to_export(*pkg_idx) {
                self.visit_export_dependencies(
                    dep_export_idx,
                    visited,
                    create_order,
                    serialize_order,
                )?;
            }
        }

        // Add this export to create order (after its create dependencies)
        create_order.push(export_idx);

        // PHASE 2: SERIALIZE ORDERING
        // Objects that must be created or serialized before this object can be serialized

        // CreateBeforeSerialization - must be created before we can serialize properties
        for pkg_idx in &deps.create_before_serialize {
            if let Some(dep_export_idx) = self.package_index_to_export(*pkg_idx) {
                self.visit_export_dependencies(
                    dep_export_idx,
                    visited,
                    create_order,
                    serialize_order,
                )?;
            }
        }

        // SerializeBeforeSerialization - must be fully serialized before we serialize this
        for pkg_idx in &deps.serialize_before_serialize {
            if let Some(dep_export_idx) = self.package_index_to_export(*pkg_idx) {
                self.visit_export_dependencies(
                    dep_export_idx,
                    visited,
                    create_order,
                    serialize_order,
                )?;
            }
        }

        // Add this export to serialize order (after its serialize dependencies)
        serialize_order.push(export_idx);

        Ok(())
    }

    /// Convert a package index to an export index if it's an export
    fn package_index_to_export(&self, pkg_idx: FPackageIndex) -> Option<usize> {
        if pkg_idx.is_export() {
            Some(pkg_idx.to_export_index() as usize)
        } else {
            None
        }
    }

    /// Load the root export and all its dependencies
    pub fn load_root(&mut self) -> Result<ObjectHandle> {
        let root_export_idx = self.find_root_export().ok_or_else(|| {
            Error::Other("No root export found (export with null outer)".to_string())
        })?;

        // Compute create and serialize order
        let (create_order, serialize_order) = self.compute_load_order(root_export_idx)?;

        // eprintln!("Create order: {:?}", create_order);
        // eprintln!("Serialize order: {:?}", serialize_order);
        // eprintln!("Total exports to load: {}", create_order.len());
        // eprintln!();

        // PHASE 1: Create all exports (allocate stubs)
        // eprintln!("=== PHASE 1: Creating exports ===");
        for export_idx in create_order {
            self.create_export(export_idx)?;
        }

        // PHASE 2: Serialize all exports (load properties)
        // eprintln!("\n=== PHASE 2: Serializing exports ===");
        for export_idx in serialize_order {
            self.serialize_export(export_idx)?;
        }

        // Return handle to root
        self.export_to_handle
            .get(&root_export_idx)
            .copied()
            .ok_or_else(|| Error::Other("Root export not found after loading".to_string()))
    }

    /// Load all root exports and their dependencies
    pub fn load_all_roots(&mut self) -> Result<Vec<ObjectHandle>> {
        let root_export_indices = self.find_all_root_exports();

        if root_export_indices.is_empty() {
            return Err(Error::Other(
                "No root exports found (exports with null outer)".to_string(),
            ));
        }

        // eprintln!("Found {} root exports", root_export_indices.len());

        // Compute combined create and serialize order for all roots
        let mut visited = HashSet::new();
        let mut combined_create_order = Vec::new();
        let mut combined_serialize_order = Vec::new();

        for &root_idx in &root_export_indices {
            self.visit_export_dependencies(
                root_idx,
                &mut visited,
                &mut combined_create_order,
                &mut combined_serialize_order,
            )?;
        }

        // eprintln!("Create order: {:?}", combined_create_order);
        // eprintln!("Serialize order: {:?}", combined_serialize_order);
        // eprintln!("Total exports to load: {}", combined_create_order.len());
        // eprintln!();

        // PHASE 1: Create all exports (allocate stubs)
        // eprintln!("=== PHASE 1: Creating exports ===");
        for export_idx in combined_create_order {
            self.create_export(export_idx)?;
        }

        // PHASE 2: Serialize all exports (load properties)
        // eprintln!("\n=== PHASE 2: Serializing exports ===");
        for export_idx in combined_serialize_order {
            self.serialize_export(export_idx)?;
        }

        // Return handles to all roots
        let mut root_handles = Vec::new();
        for root_idx in root_export_indices {
            let handle = self
                .export_to_handle
                .get(&root_idx)
                .copied()
                .ok_or_else(|| {
                    Error::Other(format!("Root export {} not found after loading", root_idx))
                })?;
            root_handles.push(handle);
        }

        Ok(root_handles)
    }
}

/// Load an asset file and return the root object handle
pub fn load_asset(asset_path: &Path, pool: &mut ObjectPool) -> Result<ObjectHandle> {
    let mut ctx = AssetLoadContext::new(asset_path, pool)?;
    ctx.load_root()
}

/// Load an asset file and return all root object handles
pub fn load_asset_all_roots(asset_path: &Path, pool: &mut ObjectPool) -> Result<Vec<ObjectHandle>> {
    let mut ctx = AssetLoadContext::new(asset_path, pool)?;
    ctx.load_all_roots()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_find_root_export() -> Result<()> {
        let asset_path = Path::new("test_assets/RMA_WallPlatforms.uasset");

        if !asset_path.exists() {
            println!("Test assets not found, skipping");
            return Ok(());
        }

        let mut pool = ObjectPool::new();
        let ctx = AssetLoadContext::new(asset_path, &mut pool)?;

        let root_idx = ctx.find_root_export();
        assert!(root_idx.is_some(), "Should find a root export");

        if let Some(idx) = root_idx {
            let export = &ctx.header.exports[idx];
            assert!(
                export.outer_index.is_null(),
                "Root export should have null outer"
            );
        }

        Ok(())
    }

    #[test]
    fn test_load_asset() -> Result<()> {
        let asset_path = Path::new("test_assets/RMA_WallPlatforms.uasset");

        let mut pool = ObjectPool::new();
        let root_handle = load_asset(asset_path, &mut pool)?;

        let _root_obj = pool
            .get(root_handle)
            .ok_or_else(|| Error::Other("Root object not found".to_string()))?;

        let root_path = pool.build_path(root_handle);
        println!("Loaded root object: {}", root_path.as_str());
        println!("Total objects loaded: {}", pool.len());

        assert!(
            !pool.is_empty(),
            "Should have loaded at least the root object"
        );

        let mut printer = crate::util::printer::ObjectPrinter::new(&pool);
        let output = printer.print_object(root_handle).unwrap();
        println!("{}", output);

        Ok(())
    }

    #[test]
    fn test_load_multiple_assets_with_printer() -> Result<()> {
        use crate::util::printer::ObjectPrinter;

        // Find two different test assets
        let asset1_path = Path::new("test_assets/RMA_WallPlatforms.uasset");
        let asset2_path = Path::new("test_assets/RMA_EndG.uasset");

        if !asset1_path.exists() || !asset2_path.exists() {
            println!("Test assets not found, skipping");
            return Ok(());
        }

        // Load both assets into the same pool
        let mut pool = ObjectPool::new();

        eprintln!("\n=== Loading Asset 1 ===");
        let root1_handle = load_asset(asset1_path, &mut pool)?;
        let pool_size_after_1 = pool.len();

        eprintln!("\n=== Loading Asset 2 ===");
        let root2_handle = load_asset(asset2_path, &mut pool)?;
        let pool_size_after_2 = pool.len();

        eprintln!("\n=== Pool Statistics ===");
        eprintln!("Objects after loading asset 1: {}", pool_size_after_1);
        eprintln!("Objects after loading asset 2: {}", pool_size_after_2);
        eprintln!("Total objects in pool: {}", pool.len());

        // Create a printer and print both root objects
        let mut printer = ObjectPrinter::new(&pool);

        eprintln!("\n=== Asset 1 Root Object ===");
        let output1 = printer
            .print_object(root1_handle)
            .map_err(|e| Error::Other(format!("Print error: {:?}", e)))?;
        println!("{}", output1);

        eprintln!("\n=== Asset 2 Root Object ===");
        let output2 = printer
            .print_object(root2_handle)
            .map_err(|e| Error::Other(format!("Print error: {:?}", e)))?;
        println!("{}", output2);

        // Verify both assets loaded correctly
        assert!(pool_size_after_1 > 0, "Asset 1 should load objects");
        assert!(
            pool_size_after_2 > pool_size_after_1,
            "Asset 2 should add more objects"
        );

        let root1_path = pool.build_path(root1_handle);
        let root2_path = pool.build_path(root2_handle);

        assert_ne!(root1_path, root2_path, "Root objects should be different");

        Ok(())
    }
}
