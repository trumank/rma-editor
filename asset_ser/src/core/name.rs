#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Name(String);

impl Name {
    pub fn new(s: impl AsRef<str>) -> Self {
        Self(s.as_ref().into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl PartialEq for Name {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl Eq for Name {}

impl std::hash::Hash for Name {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for byte in self.0.bytes() {
            byte.to_ascii_lowercase().hash(state);
        }
    }
}

impl From<&str> for Name {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for Name {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl AsRef<str> for Name {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl PartialOrd for Name {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Name {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Case-insensitive comparison by comparing lowercase bytes
        let self_lower = self.0.to_ascii_lowercase();
        let other_lower = other.0.to_ascii_lowercase();
        self_lower.cmp(&other_lower)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_case_insensitive_equality() {
        let name1 = Name::new("HelloWorld");
        let name2 = Name::new("helloworld");
        let name3 = Name::new("HELLOWORLD");
        let name4 = Name::new("HeLLOwOrLd");

        assert_eq!(name1, name2);
        assert_eq!(name1, name3);
        assert_eq!(name1, name4);
        assert_eq!(name2, name3);
    }

    #[test]
    fn test_name_hash_consistency() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let name1 = Name::new("TestName");
        let name2 = Name::new("testname");
        let name3 = Name::new("TESTNAME");

        let mut hasher1 = DefaultHasher::new();
        name1.hash(&mut hasher1);
        let hash1 = hasher1.finish();

        let mut hasher2 = DefaultHasher::new();
        name2.hash(&mut hasher2);
        let hash2 = hasher2.finish();

        let mut hasher3 = DefaultHasher::new();
        name3.hash(&mut hasher3);
        let hash3 = hasher3.finish();

        // All hashes should be the same since the names are equal (case-insensitive)
        assert_eq!(hash1, hash2);
        assert_eq!(hash1, hash3);
    }

    #[test]
    fn test_name_hashmap() {
        use std::collections::HashMap;

        let mut map = HashMap::new();
        map.insert(Name::new("MyObject"), 42);

        // Should be able to look up with different casing
        assert_eq!(map.get(&Name::new("myobject")), Some(&42));
        assert_eq!(map.get(&Name::new("MYOBJECT")), Some(&42));
        assert_eq!(map.get(&Name::new("MyObject")), Some(&42));
    }

    #[test]
    fn test_name_conversions() {
        let name_from_str = Name::from("test");
        let name_from_string = Name::from(String::from("test"));

        assert_eq!(name_from_str, name_from_string);
        assert_eq!(name_from_str.as_str(), "test");
    }

    #[test]
    fn test_name_ordering() {
        let name1 = Name::new("apple");
        let name2 = Name::new("BANANA");
        let name3 = Name::new("cherry");
        let name4 = Name::new("Apple");
        let name5 = Name::new("banana");

        // Case-insensitive ordering
        assert!(name1 < name2);
        assert!(name2 < name3);
        assert!(name1 == name4); // Equal despite different case
        assert!(name2 == name5); // Equal despite different case

        // Test with same letters but different case
        let upper = Name::new("TEST");
        let lower = Name::new("test");
        let mixed = Name::new("TeSt");

        assert_eq!(upper.cmp(&lower), std::cmp::Ordering::Equal);
        assert_eq!(upper.cmp(&mixed), std::cmp::Ordering::Equal);
        assert_eq!(lower.cmp(&mixed), std::cmp::Ordering::Equal);
    }

    #[test]
    fn test_name_sorting() {
        let mut names = [
            Name::new("Zebra"),
            Name::new("apple"),
            Name::new("BANANA"),
            Name::new("cherry"),
            Name::new("Apple"),
        ];

        names.sort();

        // Should be sorted case-insensitively
        assert_eq!(names[0].as_str(), "apple"); // or "Apple", both are equal
        assert_eq!(names[2].as_str(), "BANANA");
        assert_eq!(names[3].as_str(), "cherry");
        assert_eq!(names[4].as_str(), "Zebra");
    }
}
