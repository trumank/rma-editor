use crate::core::object_pool::LoadedObject;
use crate::object::Result;
use crate::saver::object_ref_resolver::ObjectRefResolver;
use retoc::zen::FPackageIndex;
use std::collections::HashSet;

#[derive(Debug, Default, Clone)]
pub struct ExportDependencies {
    /// Objects that must be fully serialized before this export can be serialized
    pub serialize_before_serialize: Vec<FPackageIndex>,

    /// Objects that must be created (but not serialized) before this export can be serialized
    pub create_before_serialize: Vec<FPackageIndex>,

    /// Objects that must be fully serialized before this export can be created
    pub serialize_before_create: Vec<FPackageIndex>,

    /// Objects that must be created before this export can be created
    pub create_before_create: Vec<FPackageIndex>,
}

impl ExportDependencies {
    /// Collect dependencies from a LoadedObject using ObjectRef types
    ///
    /// This is the new dependency collection method that works with the object pool
    /// architecture where properties contain ObjectRef instead of FPackageIndex.
    ///
    /// # Arguments
    /// * `object` - The loaded object to collect dependencies from
    /// * `resolver` - Resolver to convert ObjectRef to FPackageIndex
    pub fn collect_from_loaded_object(
        object: &LoadedObject,
        resolver: &ObjectRefResolver,
    ) -> Result<Self> {
        let mut deps = Self::default();

        // 1. SerializationBeforeCreateDependencies
        // Class must be serialized to know structure
        if let Ok(pkg_idx) = resolver.resolve(&object.class) {
            deps.serialize_before_create.push(pkg_idx);
        }

        // Template (archetype/CDO) must be serialized for property initialization
        if let Some(ref template_ref) = object.template
            && let Ok(pkg_idx) = resolver.resolve(template_ref)
        {
            deps.serialize_before_create.push(pkg_idx);
        }

        // 2. CreateBeforeCreateDependencies
        // Outer must exist first (container relationship)
        if let Some(ref outer_ref) = object.outer {
            match resolver.resolve(outer_ref) {
                Ok(pkg_idx) => {
                    deps.create_before_create.push(pkg_idx);
                }
                Err(e) => {
                    eprintln!("Failed to resolve outer_ref '{:?}': {}", outer_ref, e);
                }
            }
        }

        // TODO: Super struct (not currently stored in LoadedObject)

        // 3. CreateBeforeSerializationDependencies
        // Collect all object references by walking properties
        let mut object_refs = Vec::new();
        object.object.collect_property_refs(&mut object_refs);

        for obj_ref in object_refs {
            if let Ok(pkg_idx) = resolver.resolve(&obj_ref) {
                // Exclude template since it's already in serialize_before_create
                if let Some(ref template_ref) = object.template
                    && template_ref == &obj_ref
                {
                    continue;
                }
                deps.create_before_serialize.push(pkg_idx);
            } else {
                // Debug: Log unresolved refs
                eprintln!(
                    "Warning: Could not resolve object ref in create_before_serialize: {:?}",
                    obj_ref
                );
            }
        }

        // 4. SerializationBeforeSerializationDependencies
        // Custom dependencies from GetPreloadDependencies()
        let mut preload_refs = Vec::new();
        object.object.get_preload_dependencies(&mut preload_refs);
        for preload_ref in preload_refs {
            if let Ok(pkg_idx) = resolver.resolve(&preload_ref) {
                deps.serialize_before_serialize.push(pkg_idx);
            }
        }

        // Remove redundancies
        deps.remove_redundancies();

        Ok(deps)
    }

    /// Remove redundant dependencies based on the dependency hierarchy
    ///
    /// Rules from SavePackage.cpp:4088-4127:
    /// - SerializationBeforeCreate implies SerializationBeforeSerialization
    /// - SerializationBeforeCreate implies CreateBeforeSerialization
    /// - SerializationBeforeSerialization implies CreateBeforeSerialization
    /// - CreateBeforeCreate implies CreateBeforeSerialization
    fn remove_redundancies(&mut self) {
        let sbc_set: HashSet<_> = self.serialize_before_create.iter().copied().collect();
        let sbs_set: HashSet<_> = self.serialize_before_serialize.iter().copied().collect();
        let cbc_set: HashSet<_> = self.create_before_create.iter().copied().collect();

        // Remove SerializationBeforeSerialization if in SerializationBeforeCreate
        self.serialize_before_serialize
            .retain(|idx| !sbc_set.contains(idx));

        // Remove CreateBeforeSerialization if redundant
        self.create_before_serialize.retain(|idx| {
            !sbc_set.contains(idx)  // SerializationBeforeCreate implies CreateBeforeSerialization
                && !sbs_set.contains(idx)  // SerializationBeforeSerialization implies CreateBeforeSerialization
                && !cbc_set.contains(idx) // CreateBeforeCreate implies CreateBeforeSerialization
        });

        // Remove duplicates within each category
        // Note: We use sort+dedup for CreateBefore* since order doesn't matter there,
        // but for SerializationBefore* we preserve order and just dedup
        Self::dedup(&mut self.serialize_before_serialize);
        Self::dedup(&mut self.create_before_serialize);
        Self::dedup(&mut self.serialize_before_create);
        Self::dedup(&mut self.create_before_create);
    }

    /// Deduplicate preserving order (for when order matters like SerializationBeforeCreate)
    fn dedup(vec: &mut Vec<FPackageIndex>) {
        let mut seen = HashSet::new();
        vec.retain(|idx| seen.insert(*idx));
    }

    /// Write dependencies to the preload_dependencies array and update export metadata
    ///
    /// Returns (first_index, sbs_count, cbs_count, sbc_count, cbc_count)
    pub fn write_to_preload_array(
        &self,
        preload_dependencies: &mut Vec<FPackageIndex>,
    ) -> (i32, i32, i32, i32, i32) {
        // If no dependencies, return -1 as first index
        if self.is_empty() {
            return (-1, 0, 0, 0, 0);
        }

        let first_index = preload_dependencies.len() as i32;

        // Append dependencies in the correct order: SbS, CbS, SbC, CbC
        preload_dependencies.extend(&self.serialize_before_serialize);
        preload_dependencies.extend(&self.create_before_serialize);
        preload_dependencies.extend(&self.serialize_before_create);
        preload_dependencies.extend(&self.create_before_create);

        (
            first_index,
            self.serialize_before_serialize.len() as i32,
            self.create_before_serialize.len() as i32,
            self.serialize_before_create.len() as i32,
            self.create_before_create.len() as i32,
        )
    }

    /// Check if there are any dependencies
    pub fn is_empty(&self) -> bool {
        self.serialize_before_serialize.is_empty()
            && self.create_before_serialize.is_empty()
            && self.serialize_before_create.is_empty()
            && self.create_before_create.is_empty()
    }

    /// Total number of dependencies
    pub fn total_count(&self) -> usize {
        self.serialize_before_serialize.len()
            + self.create_before_serialize.len()
            + self.serialize_before_create.len()
            + self.create_before_create.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redundancy_elimination() {
        let mut deps = ExportDependencies {
            serialize_before_create: vec![FPackageIndex { index: 1 }],
            serialize_before_serialize: vec![
                FPackageIndex { index: 1 }, // Redundant with serialize_before_create
                FPackageIndex { index: 2 },
            ],
            create_before_serialize: vec![
                FPackageIndex { index: 1 }, // Redundant with serialize_before_create
                FPackageIndex { index: 2 }, // Redundant with serialize_before_serialize
                FPackageIndex { index: 3 },
            ],
            create_before_create: vec![FPackageIndex { index: 4 }],
        };

        deps.remove_redundancies();

        // Index 1 should only be in serialize_before_create
        assert!(
            deps.serialize_before_create
                .contains(&FPackageIndex { index: 1 })
        );
        assert!(
            !deps
                .serialize_before_serialize
                .contains(&FPackageIndex { index: 1 })
        );
        assert!(
            !deps
                .create_before_serialize
                .contains(&FPackageIndex { index: 1 })
        );

        // Index 2 should only be in serialize_before_serialize
        assert!(
            deps.serialize_before_serialize
                .contains(&FPackageIndex { index: 2 })
        );
        assert!(
            !deps
                .create_before_serialize
                .contains(&FPackageIndex { index: 2 })
        );

        // Index 3 should remain in create_before_serialize
        assert!(
            deps.create_before_serialize
                .contains(&FPackageIndex { index: 3 })
        );

        // Index 4 should remain in create_before_create
        assert!(
            deps.create_before_create
                .contains(&FPackageIndex { index: 4 })
        );
    }

    #[test]
    fn test_write_to_preload_array() {
        let deps = ExportDependencies {
            serialize_before_serialize: vec![FPackageIndex { index: 1 }],
            create_before_serialize: vec![FPackageIndex { index: 2 }, FPackageIndex { index: 3 }],
            serialize_before_create: vec![FPackageIndex { index: 4 }],
            create_before_create: vec![FPackageIndex { index: 5 }],
        };

        let mut preload = Vec::new();
        let (first, sbs, cbs, sbc, cbc) = deps.write_to_preload_array(&mut preload);

        assert_eq!(first, 0);
        assert_eq!(sbs, 1);
        assert_eq!(cbs, 2);
        assert_eq!(sbc, 1);
        assert_eq!(cbc, 1);
        assert_eq!(preload.len(), 5);

        // Verify order: SbS, CbS, SbC, CbC
        assert_eq!(preload[0].index, 1); // serialize_before_serialize
        assert_eq!(preload[1].index, 2); // create_before_serialize[0]
        assert_eq!(preload[2].index, 3); // create_before_serialize[1]
        assert_eq!(preload[3].index, 4); // serialize_before_create
        assert_eq!(preload[4].index, 5); // create_before_create
    }
}
