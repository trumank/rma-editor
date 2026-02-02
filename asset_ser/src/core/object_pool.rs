use crate::core::name::Name;
use crate::core::object_path::ObjectPath;
use crate::object::ObjectType;
use std::collections::HashMap;
use uesave::ArchiveType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ObjectHandle(u32);

/// Reference to an object - either loaded or unloaded
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ObjectRef {
    Loaded(ObjectHandle),
    Unloaded(ObjectPath),
}

impl From<ObjectHandle> for ObjectRef {
    fn from(value: ObjectHandle) -> Self {
        Self::Loaded(value)
    }
}

impl ObjectRef {
    pub fn loaded(handle: ObjectHandle) -> Self {
        Self::Loaded(handle)
    }

    pub fn unloaded(path: impl Into<ObjectPath>) -> Self {
        Self::Unloaded(path.into())
    }

    pub fn is_loaded(&self) -> bool {
        matches!(self, Self::Loaded(_))
    }

    /// Get the object handle if this is a loaded reference
    pub fn as_handle(&self) -> Option<ObjectHandle> {
        match self {
            Self::Loaded(handle) => Some(*handle),
            Self::Unloaded(_) => None,
        }
    }

    /// Get the object path if this is an unloaded reference
    pub fn as_path(&self) -> Option<&str> {
        match self {
            Self::Loaded(_) => None,
            Self::Unloaded(path) => Some(path.as_str()),
        }
    }

    /// Get the object ObjectPath if this is an unloaded reference
    pub fn as_object_path(&self) -> Option<&ObjectPath> {
        match self {
            Self::Loaded(_) => None,
            Self::Unloaded(path) => Some(path),
        }
    }
}

/// Archive type for loaded objects with ObjectRef
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AssetArchiveType;

impl ArchiveType for AssetArchiveType {
    type ObjectRef = ObjectRef;
    type SoftObjectPath = (String, i32);
}

impl serde::Serialize for AssetArchiveType {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_unit()
    }
}

impl<'de> serde::Deserialize<'de> for AssetArchiveType {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_unit(serde::de::IgnoredAny)?;
        Ok(Self)
    }
}

/// A loaded object with its properties and metadata
#[derive(Debug)]
pub struct LoadedObject {
    pub name: Name,
    pub outer: Option<ObjectRef>,
    pub class: ObjectRef,
    pub template: Option<ObjectRef>,
    pub object: Box<dyn ObjectType>,
}

impl LoadedObject {
    pub fn properties(&self) -> &uesave::Properties<AssetArchiveType> {
        self.object.properties()
    }
    pub fn properties_mut(&mut self) -> &mut uesave::Properties<AssetArchiveType> {
        self.object.properties_mut()
    }
}

/// Pool allocator for managing loaded objects
#[derive(Debug, Default)]
pub struct ObjectPool {
    objects: Vec<LoadedObject>,
    path_to_handle: HashMap<ObjectPath, ObjectHandle>,
}

impl ObjectPool {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build the full path for an object by walking the outer chain
    pub fn build_path(&self, handle: ObjectHandle) -> ObjectPath {
        let obj = self.get(handle).unwrap();

        match &obj.outer {
            Some(outer) => {
                let outer_path = self.resolve_path(outer);
                ObjectPath::new(format!("{}.{}", outer_path, obj.name.as_str()))
            }
            None => ObjectPath::new(obj.name.as_str()),
        }
    }

    /// Resolve an ObjectRef to its path
    pub fn resolve_path(&self, obj_ref: &ObjectRef) -> ObjectPath {
        match obj_ref {
            ObjectRef::Loaded(h) => self.build_path(*h),
            ObjectRef::Unloaded(p) => p.clone(),
        }
    }

    /// Find an object by name and outer reference
    pub fn find_by_name_and_outer(
        &self,
        name: &Name,
        outer: &Option<ObjectRef>,
    ) -> Option<ObjectHandle> {
        self.objects.iter().enumerate().find_map(|(idx, obj)| {
            if &obj.name == name && &obj.outer == outer {
                Some(ObjectHandle(idx as u32))
            } else {
                None
            }
        })
    }

    /// Allocate a new object in the pool and return its handle
    pub fn allocate(&mut self, object: LoadedObject) -> ObjectHandle {
        if let Some(handle) = self.find_by_name_and_outer(&object.name, &object.outer) {
            return handle;
        }

        let handle = ObjectHandle(self.objects.len() as u32);

        self.objects.push(object);

        let path = self.build_path(handle);
        self.path_to_handle.insert(path, handle);

        handle
    }

    /// Get an object by handle
    pub fn get(&self, handle: ObjectHandle) -> Option<&LoadedObject> {
        self.objects.get(handle.0 as usize)
    }

    /// Get a mutable reference to an object by handle
    pub fn get_mut(&mut self, handle: ObjectHandle) -> Option<&mut LoadedObject> {
        self.objects.get_mut(handle.0 as usize)
    }

    /// Look up an object handle by path (case-insensitive)
    pub fn find_by_path(&self, path: &ObjectPath) -> Option<ObjectHandle> {
        self.path_to_handle.get(path).copied()
    }

    /// Get an object by path (case-insensitive)
    pub fn get_by_path(&self, path: &ObjectPath) -> Option<&LoadedObject> {
        self.find_by_path(path).and_then(|h| self.get(h))
    }

    /// Check if an object is loaded by path (case-insensitive)
    pub fn is_loaded(&self, path: &str) -> bool {
        self.path_to_handle.contains_key(&ObjectPath::new(path))
    }

    /// Total number of loaded objects
    pub fn len(&self) -> usize {
        self.objects.len()
    }

    /// Check if the pool is empty
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    /// Iterate over all loaded objects
    pub fn iter(&self) -> impl Iterator<Item = (ObjectHandle, &LoadedObject)> {
        self.objects
            .iter()
            .enumerate()
            .map(|(i, obj)| (ObjectHandle(i as u32), obj))
    }
}

#[cfg(test)]
mod tests {
    use crate::object::UObject;

    use super::*;

    #[test]
    fn test_object_pool_allocation() {
        let mut pool = ObjectPool::new();

        // First, allocate a class object
        let class_obj = LoadedObject {
            name: "Actor".into(),
            outer: Some(ObjectRef::Unloaded("/Script/Engine".into())),
            class: ObjectRef::Unloaded("/Script/CoreUObject.Class".into()), // Placeholder
            template: None,
            object: Box::new(UObject::default()),
        };
        let class_handle = pool.allocate(class_obj);

        let obj1 = LoadedObject {
            name: "Object1".into(),
            outer: Some(ObjectRef::Unloaded("/Game/Test".into())),
            class: ObjectRef::Loaded(class_handle),
            template: None,
            object: Box::new(UObject::default()),
        };

        let handle1 = pool.allocate(obj1);
        assert_eq!(handle1.0, 1);
        assert_eq!(pool.build_path(handle1).as_str(), "/Game/Test.Object1");

        let obj2 = LoadedObject {
            name: "Object2".into(),
            outer: Some(ObjectRef::Unloaded("/Game/Test".into())),
            class: ObjectRef::Loaded(class_handle),
            template: None,
            object: Box::new(UObject::default()),
        };

        let handle2 = pool.allocate(obj2);
        assert_eq!(handle2.0, 2);
        assert_eq!(pool.build_path(handle2).as_str(), "/Game/Test.Object2");

        assert_eq!(pool.len(), 3);
    }

    #[test]
    fn test_object_ref() {
        let handle = ObjectHandle(42);
        let loaded_ref = ObjectRef::loaded(handle);

        assert!(loaded_ref.is_loaded());
        assert_eq!(loaded_ref.as_handle(), Some(handle));
        assert_eq!(loaded_ref.as_path(), None);

        let unloaded_ref = ObjectRef::unloaded("/Script/CoreUObject.Vector");

        assert!(!unloaded_ref.is_loaded());
        assert_eq!(unloaded_ref.as_handle(), None);
        assert_eq!(unloaded_ref.as_path(), Some("/Script/CoreUObject.Vector"));
    }
}
