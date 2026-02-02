//! Import table builder for asset serialization
//!
//! This module builds the import table for legacy assets, parsing object paths
//! and constructing FObjectImport entries with proper outer chains.

use anyhow::{Context, Result, bail};
use retoc::legacy_asset::{FObjectImport, FPackageNameMap};
use retoc::zen::FPackageIndex;
use std::collections::{HashMap, HashSet};

use crate::core::name::Name;
use crate::core::object_path::ObjectPath;
use crate::core::object_pool::ObjectPool;

/// Determine the class name for an object path
///
/// Looks up the class in the object pool (if loaded) or jmap.
pub fn determine_class(
    path: &ObjectPath,
    pool: &ObjectPool,
    jmap: &jmap::Jmap,
) -> Result<ObjectPath> {
    // First check if the object is loaded in the pool
    if let Some(handle) = pool.find_by_path(path) {
        let obj = pool.get(handle).context("Object handle invalid")?;
        // Class is now an ObjectRef, need to get its path
        return Ok(pool.resolve_path(&obj.class));
    }

    // Otherwise look it up in jmap
    if let Some(jmap_class) = jmap.objects.get(path.as_str()) {
        return Ok(ObjectPath::new(jmap_class.get_object().class.as_str()));
    }

    // Handle packages: /Script/X or /Game/X without a dot are packages
    let path_str = path.as_str();
    if (path_str.starts_with("/Script/") || path_str.starts_with("/Game/"))
        && !path_str.contains('.')
    {
        return Ok(ObjectPath::new("/Script/CoreUObject.Package"));
    } else if path_str.ends_with("_C") {
        // could potentially be a widget class or a even some custom class but this is probably fine
        return Ok(ObjectPath::new("/Script/Engine.BlueprintGeneratedClass"));
    }

    bail!("Cannot determine class for path: {}", path)
}

/// Build import table from a set of import paths
///
/// This expands the set to include all outer imports, determines classes,
/// sorts alphabetically, and constructs FObjectImport entries.
///
/// Returns (imports, sorted_import_paths)
pub fn build_imports(
    import_paths: &HashSet<ObjectPath>,
    pool: &ObjectPool,
    jmap: &jmap::Jmap,
    name_map: &mut FPackageNameMap,
) -> Result<(Vec<FObjectImport>, Vec<ObjectPath>)> {
    // First, expand the set to include all outers and determine classes
    let mut path_to_class = HashMap::new();

    for path in import_paths {
        add_import_with_outers(path, pool, jmap, &mut path_to_class)?;
    }

    if path_to_class.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }

    // Build a vector of (class_name, path, class_path) for sorting
    let mut sortable: Vec<(Name, ObjectPath, ObjectPath)> = path_to_class
        .iter()
        .map(|(path, class_path)| {
            let class_name = Name::new(class_path.object_name());
            (class_name, path.clone(), class_path.clone())
        })
        .collect();

    // Sort by (class_name, path) (case-insensitive, matching UE's FCString::Stricmp)
    sortable.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));

    // Build a mapping from path to sorted index
    let mut path_to_sorted_index = HashMap::new();
    for (new_index, (_, path, _)) in sortable.iter().enumerate() {
        path_to_sorted_index.insert(path.clone(), new_index as u32);
    }

    // Build the actual FObjectImport structs
    let mut sorted_imports = Vec::new();
    let mut sorted_paths = Vec::new();

    for (class_name, path, class_path) in sortable {
        let object_name = path.object_name();

        // Determine outer_index using the sorted indices
        let outer_index = if let Some(outer_path) = path.outer_path() {
            let outer_idx = path_to_sorted_index
                .get(&outer_path)
                .with_context(|| format!("Outer path not found in imports: {}", outer_path))?;
            FPackageIndex::create_import(*outer_idx)
        } else {
            FPackageIndex::create_null()
        };

        let class_package = class_path
            .outer_path()
            .with_context(|| format!("Class cannot be top level: {}", class_path))?;

        let import = FObjectImport {
            class_package: name_map.store(class_package.as_str()),
            class_name: name_map.store(class_name.as_str()),
            outer_index,
            object_name: name_map.store(object_name),
            is_optional: false,
        };

        sorted_imports.push(import);
        sorted_paths.push(path);
    }

    Ok((sorted_imports, sorted_paths))
}

/// Recursively add an import path and all its outers to the path_to_class map
fn add_import_with_outers(
    path: &ObjectPath,
    pool: &ObjectPool,
    jmap: &jmap::Jmap,
    path_to_class: &mut HashMap<ObjectPath, ObjectPath>,
) -> Result<()> {
    // Check if already added
    if path_to_class.contains_key(path) {
        return Ok(());
    }

    // Determine class for this path
    let class_path = determine_class(path, pool, jmap)?;

    // Recursively add outer first if it exists
    if let Some(outer_path) = path.outer_path() {
        add_import_with_outers(&outer_path, pool, jmap, path_to_class)?;
    }

    // Now add this import
    path_to_class.insert(path.clone(), class_path);

    Ok(())
}
