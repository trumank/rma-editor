use crate::core::name::Name;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ObjectPath(Name);

impl ObjectPath {
    /// Create a new ObjectPath from a string
    pub fn new(s: impl Into<Name>) -> Self {
        Self(s.into())
    }

    /// Get the underlying string
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Get the underlying Name
    pub fn as_name(&self) -> &Name {
        &self.0
    }

    /// Extract the object name from the path
    ///
    /// # Examples
    /// - "/Game/Maps/TestMap.PersistentLevel" → "PersistentLevel"
    /// - "/Script/Engine.Actor" → "Actor"
    /// - "/Game/Test.Outer:Inner" → "Inner"
    pub fn object_name(&self) -> &str {
        let path = self.0.as_str();

        path.rsplit_once([':', '.'])
            .map(|(_outer, name)| name)
            .unwrap_or(path)
    }

    /// Extract the outer path (parent object)
    ///
    /// # Examples
    /// - "/Game/Maps/TestMap" → None
    /// - "/Game/Test.Outer:Inner" → Some("/Game/Test.Outer")
    /// - "/Game/Pkg.A:B:C" → Some("/Game/Pkg.A:B")
    pub fn outer_path(&self) -> Option<ObjectPath> {
        let path = self.0.as_str();

        path.rsplit_once([':', '.'])
            .map(|(outer, _name)| ObjectPath::new(outer))
    }

    /// Convert to Name
    pub fn into_name(self) -> Name {
        self.0
    }
}

impl From<&str> for ObjectPath {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for ObjectPath {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<Name> for ObjectPath {
    fn from(name: Name) -> Self {
        Self(name)
    }
}

impl AsRef<str> for ObjectPath {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl std::fmt::Display for ObjectPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl PartialOrd for ObjectPath {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ObjectPath {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Delegate to Name's case-insensitive ordering
        self.0.cmp(&other.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_object_name() {
        assert_eq!(
            ObjectPath::new("/Game/Maps/TestMap.PersistentLevel").object_name(),
            "PersistentLevel"
        );
        assert_eq!(
            ObjectPath::new("/Script/Engine.Actor").object_name(),
            "Actor"
        );
        assert_eq!(
            ObjectPath::new("/Game/Test.Outer:Inner").object_name(),
            "Inner"
        );
        assert_eq!(ObjectPath::new("SimpleName").object_name(), "SimpleName");
    }

    #[test]
    fn test_outer_path() {
        assert_eq!(
            ObjectPath::new("/Game/Maps/TestMap.PersistentLevel").outer_path(),
            Some(ObjectPath::new("/Game/Maps/TestMap"))
        );
        assert_eq!(
            ObjectPath::new("/Game/Test.Outer:Inner").outer_path(),
            Some(ObjectPath::new("/Game/Test.Outer"))
        );
        assert_eq!(
            ObjectPath::new("/Game/Pkg.A:B:C").outer_path(),
            Some(ObjectPath::new("/Game/Pkg.A:B"))
        );
    }

    #[test]
    fn test_case_insensitive_equality() {
        let path1 = ObjectPath::new("/Game/Test/MyObject");
        let path2 = ObjectPath::new("/game/test/myobject");
        let path3 = ObjectPath::new("/GAME/TEST/MYOBJECT");

        assert_eq!(path1, path2);
        assert_eq!(path1, path3);
    }

    #[test]
    fn test_object_path_ordering() {
        let path1 = ObjectPath::new("/Game/A/Object");
        let path2 = ObjectPath::new("/Game/B/Object");
        let path3 = ObjectPath::new("/Script/Engine.Actor");
        let path4 = ObjectPath::new("/game/a/object");

        // Case-insensitive ordering
        assert!(path1 < path2);
        assert!(path1 < path3);
        assert!(path2 < path3);
        assert_eq!(path1.cmp(&path4), std::cmp::Ordering::Equal);
    }

    #[test]
    fn test_object_path_sorting() {
        let mut paths = [
            ObjectPath::new("/Script/Engine.Actor"),
            ObjectPath::new("/Game/Maps/TestMap"),
            ObjectPath::new("/game/assets/MyAsset"),
            ObjectPath::new("/Script/CoreUObject.Vector"),
            ObjectPath::new("/GAME/Characters/Player"),
        ];

        paths.sort();

        // Should be sorted case-insensitively
        // /Game paths come before /Script paths alphabetically
        assert!(paths[0].as_str().to_lowercase().starts_with("/game"));
        assert!(paths[1].as_str().to_lowercase().starts_with("/game"));
        assert!(paths[2].as_str().to_lowercase().starts_with("/game"));
        assert!(paths[3].as_str().to_lowercase().starts_with("/script"));
        assert!(paths[4].as_str().to_lowercase().starts_with("/script"));
    }

    #[test]
    fn test_subobject_ordering() {
        let path1 = ObjectPath::new("/Game/Test.Object:SubA");
        let path2 = ObjectPath::new("/Game/Test.Object:SubB");
        let path3 = ObjectPath::new("/game/test.object:suba");

        assert!(path1 < path2);
        assert_eq!(path1.cmp(&path3), std::cmp::Ordering::Equal);
    }
}
