//! Package writer for serializing assets from object pool
//!
//! This module orchestrates the complete workflow for writing .uasset + .uexp files
//! from a pool of loaded objects.
//!
//! # Current Limitations
//!
//! This is a work-in-progress implementation. The following components are complete:
//! - Name map building
//! - Object reference resolution
//! - Import table construction
//! - Export table construction
//! - Property serialization
//!
//! The missing piece is that retoc doesn't currently provide a `serialize` method
//! for `FLegacyPackageHeader`, only deserialization. This would need to be added
//! to retoc to complete the round-trip serialization.

use crate::AssetVersionInfo;
use crate::archive::writer::AssetArchiveWriter;
use crate::core::name::Name;
use crate::core::object_path::ObjectPath;
use crate::core::object_pool::{
    AssetArchiveType, LoadedObject, ObjectHandle, ObjectPool, ObjectRef,
};
use crate::loader::dependency_collector::ExportDependencies;
use crate::saver::export_builder::ExportBuilder;
use crate::saver::import_builder::build_imports;
use crate::saver::object_ref_resolver::ObjectRefResolver;
use anyhow::Result;
use retoc::legacy_asset::{FObjectExport, FObjectImport, FPackageNameMap};
use retoc::zen::FPackageIndex;
use std::collections::HashSet;

/// Components of a serialized package
#[derive(Debug)]
pub struct PackageComponents {
    pub imports: Vec<FObjectImport>,
    pub exports: Vec<FObjectExport>,
    pub preload_dependencies: Vec<FPackageIndex>,
    pub name_map: FPackageNameMap,
    pub uexp_data: Vec<u8>,
    pub version: AssetVersionInfo,
    pub package_name: String,
}

/// Write a package from the object pool
pub struct PackageWriter<'a> {
    /// Object pool containing loaded objects
    pool: &'a ObjectPool,

    /// Version information for the package
    version: AssetVersionInfo,

    /// Package name
    package_name: String,

    /// Root exports to include (if empty, includes all objects in pool)
    root_exports: Vec<ObjectHandle>,
}

impl<'a> PackageWriter<'a> {
    /// Create a new package writer
    pub fn new(
        pool: &'a ObjectPool,
        version: AssetVersionInfo,
        package_name: String,
        root_exports: Vec<ObjectHandle>,
    ) -> Self {
        Self {
            pool,
            version,
            package_name,
            root_exports,
        }
    }

    /// Prepare package components for serialization
    ///
    /// Returns the components needed to serialize the package:
    /// - Import table
    /// - Export table
    /// - Preload dependencies
    /// - Name map
    /// - Serialized export data (.uexp)
    ///
    /// Note: This stops short of actual .uasset serialization because retoc
    /// doesn't currently provide FLegacyPackageHeader::serialize().
    pub fn prepare(self, jmap: &jmap::Jmap) -> Result<PackageComponents> {
        // Phase 1: Collect all object references (including struct types from properties)
        let all_refs = self.collect_all_references();

        // Phase 2: Initialize builders
        let mut name_map = FPackageNameMap::create();
        let mut resolver = ObjectRefResolver::new();
        let mut export_builder = ExportBuilder::new();

        // Add package name to name map
        name_map.store(&self.package_name);

        // Phase 3: Determine which exports to process
        let mut export_handles = vec![];
        for root in &self.root_exports {
            let reachable = self.collect_exports_from_root(*root)?;
            export_handles.extend(reachable);
        }

        // Phase 4: Classify references and collect import paths
        let mut import_paths = HashSet::new();
        for object_ref in &all_refs {
            match self.classify_reference_for_import(object_ref, &export_handles) {
                Some(import_path) => {
                    import_paths.insert(import_path);
                }
                None => {
                    // It's an export, skip
                }
            }
        }

        // Phase 5: Build and sort imports
        let (imports, sorted_import_paths) =
            build_imports(&import_paths, self.pool, jmap, &mut name_map)?;

        // Phase 5b: Sort exports by (class_name, path) to match UE behavior
        // This must happen BEFORE allocating indices so dependencies use correct indices
        let mut export_entries: Vec<(ObjectHandle, Name, ObjectPath)> = export_handles
            .iter()
            .map(|&handle| {
                let object = self
                    .pool
                    .get(handle)
                    .ok_or_else(|| anyhow::anyhow!("Object handle not found: {:?}", handle))?;
                let class_name = match &object.class {
                    ObjectRef::Loaded(h) => self.pool.get(*h).unwrap().name.clone(),
                    ObjectRef::Unloaded(p) => Name::new(p.object_name()),
                };
                let object_path = self.pool.build_path(handle);
                Ok((handle, class_name, object_path))
            })
            .collect::<Result<Vec<_>>>()?;

        export_entries.sort_by(|a, b| (&a.1, &a.2).cmp(&(&b.1, &b.2)));
        let sorted_export_handles: Vec<ObjectHandle> =
            export_entries.iter().map(|(h, _, _)| *h).collect();

        // Phase 6: Register ALL final package indices in resolver
        // 6a: Register sorted imports
        for (import_index, import_path) in sorted_import_paths.iter().enumerate() {
            resolver.register_import(import_path.clone(), import_index as u32);
        }

        // 6b: Register sorted exports
        for (export_index, &handle) in sorted_export_handles.iter().enumerate() {
            let object_path = self.pool.build_path(handle);

            resolver.register_export_with_path(
                handle,
                object_path.to_string(),
                export_index as u32,
            );
        }

        // Phase 7: NOW serialize properties and compute dependencies (using final indices)
        let mut preload_dependencies = Vec::new();

        for handle in &sorted_export_handles {
            let object = self
                .pool
                .get(*handle)
                .ok_or_else(|| anyhow::anyhow!("Object handle not found: {:?}", handle))?;

            // Serialize properties
            let serial_data =
                self.serialize_properties(object, &mut resolver, &mut name_map, jmap)?;

            // Compute dependencies
            let deps = self.compute_dependencies(object, &resolver)?;

            // Add export to builder
            let export_index = export_builder.add_export(
                *handle,
                object,
                &resolver,
                &mut name_map,
                serial_data,
            )?;

            // Write dependencies to preload array and update export metadata
            let (first_dep_idx, _sbs, _cbs, _sbc, _cbc) =
                deps.write_to_preload_array(&mut preload_dependencies);

            export_builder.update_dependencies(export_index, &deps, first_dep_idx)?;
        }

        // Phase 8: Finalize exports
        // Export offsets will be set to 0 initially and filled in later during serialization.
        // The retoc serialize method does a two-pass approach:
        // 1. Write header, calculate actual header size
        // 2. Go back and patch export offsets with actual header size
        // So we don't need to estimate here - just pass 0.
        let (exports, uexp_data) = export_builder.finalize(0);

        Ok(PackageComponents {
            imports,
            exports,
            preload_dependencies,
            name_map,
            uexp_data,
            version: self.version,
            package_name: self.package_name,
        })
    }

    /// Collect all ObjectRef instances from all properties in the pool
    fn collect_all_references(&self) -> HashSet<ObjectRef> {
        let mut refs = HashSet::new();

        for (_handle, object) in self.pool.iter() {
            // Add metadata references
            refs.insert(object.class.clone());
            if let Some(ref template_ref) = object.template {
                refs.insert(template_ref.clone());
            }
            if let Some(ref outer_ref) = object.outer {
                refs.insert(outer_ref.clone());
            }

            // Walk properties to extract all ObjectRef instances
            let mut prop_refs = Vec::new();
            object.object.collect_property_refs(&mut prop_refs);
            refs.extend(prop_refs);
        }

        refs
    }

    /// Classify a reference for import building
    ///
    /// Returns Some(path) if the reference should be an import, None if it's an export
    fn classify_reference_for_import(
        &self,
        object_ref: &ObjectRef,
        export_handles: &[ObjectHandle],
    ) -> Option<ObjectPath> {
        match object_ref {
            ObjectRef::Loaded(handle) => {
                // Check if this handle is in our export list
                if export_handles.contains(handle) {
                    None // It's an export
                } else {
                    // It's loaded but not exported, so it should be an import
                    // Get its path by building it
                    Some(self.pool.build_path(*handle))
                }
            }
            ObjectRef::Unloaded(path) => {
                // Check if this path matches any export
                let is_export = export_handles.iter().any(|&handle| {
                    let obj_path = self.pool.build_path(handle);
                    &obj_path == path
                });

                if is_export { None } else { Some(path.clone()) }
            }
        }
    }

    /// Serialize properties for an object
    fn serialize_properties(
        &self,
        object: &LoadedObject,
        resolver: &mut ObjectRefResolver,
        name_map: &mut FPackageNameMap,
        jmap: &jmap::Jmap,
    ) -> Result<Vec<u8>> {
        // Get class path for jmap lookups
        let class_path = self.pool.resolve_path(&object.class);

        let mut serializer = AssetArchiveWriter::new(
            std::io::Cursor::new(Vec::new()),
            self.version.clone(),
            resolver,
            name_map,
            jmap,
            class_path,
        );

        // Serialize object through ObjectType
        object
            .object
            .ser(&mut serializer)
            .map_err(|e| anyhow::anyhow!("Failed to serialize object: {}", e))?;

        let mut buf = serializer.into_inner().into_inner();
        buf.extend_from_slice(&[0, 0, 0, 0]); // mystery
        Ok(buf)
    }

    /// Compute dependencies for an export
    fn compute_dependencies(
        &self,
        object: &LoadedObject,
        resolver: &ObjectRefResolver,
    ) -> Result<ExportDependencies> {
        ExportDependencies::collect_from_loaded_object(object, resolver)
            .map_err(|e| anyhow::anyhow!("Failed to collect dependencies: {}", e))
    }

    /// Collect all exports reachable from a root in dependency order (dependencies first)
    fn collect_exports_from_root(&self, root: ObjectHandle) -> Result<Vec<ObjectHandle>> {
        let mut visited = HashSet::new();
        let mut order = Vec::new();
        self.visit_export_deps(root, &mut visited, &mut order)?;
        Ok(order)
    }

    /// DFS traversal to visit export dependencies
    fn visit_export_deps(
        &self,
        handle: ObjectHandle,
        visited: &mut HashSet<ObjectHandle>,
        order: &mut Vec<ObjectHandle>,
    ) -> Result<()> {
        if visited.contains(&handle) {
            return Ok(());
        }
        visited.insert(handle);

        let object = self
            .pool
            .get(handle)
            .ok_or_else(|| anyhow::anyhow!("Object handle {:?} not found in pool", handle))?;

        // Visit dependencies in order: outer, class, template
        // (This matches the order they'll appear in CreateBeforeCreate and SerializeBeforeCreate)

        // Outer dependency (CreateBeforeCreate)
        if let Some(ref outer_ref) = object.outer
            && let Some(outer_handle) = outer_ref.as_handle()
        {
            self.visit_export_deps(outer_handle, visited, order)?;
        }

        // Class dependency (SerializeBeforeCreate)
        if let Some(class_handle) = object.class.as_handle() {
            self.visit_export_deps(class_handle, visited, order)?;
        }

        // Template dependency (SerializeBeforeCreate)
        if let Some(ref template_ref) = object.template
            && let Some(template_handle) = template_ref.as_handle()
        {
            self.visit_export_deps(template_handle, visited, order)?;
        }

        // Property dependencies (CreateBeforeSerialization)
        // Walk the property tree to find ObjectRef instances
        self.visit_property_deps(object.properties(), visited, order)?;

        // Add this export after all its dependencies (post-order)
        order.push(handle);

        Ok(())
    }

    /// Walk properties to find ObjectRef dependencies
    fn visit_property_deps(
        &self,
        properties: &uesave::Properties<AssetArchiveType>,
        visited: &mut HashSet<ObjectHandle>,
        order: &mut Vec<ObjectHandle>,
    ) -> Result<()> {
        // Collect handles first (closure borrows self immutably)
        let mut handles = Vec::new();
        crate::core::property_visitor::visit_object_refs(properties, &mut |obj_ref| {
            if let Some(handle) = self.resolve_object_ref_to_handle(obj_ref) {
                handles.push(handle);
            }
        });

        // Then visit each handle
        for handle in handles {
            self.visit_export_deps(handle, visited, order)?;
        }
        Ok(())
    }

    /// Resolve an ObjectRef to an ObjectHandle if it's in the pool (export)
    fn resolve_object_ref_to_handle(&self, obj_ref: &ObjectRef) -> Option<ObjectHandle> {
        match obj_ref {
            ObjectRef::Loaded(handle) => Some(*handle),
            ObjectRef::Unloaded(path) => self.pool.find_by_path(path),
        }
    }
}
