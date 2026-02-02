use asset_ser::core::object_pool::{AssetArchiveType, ObjectRef};
use uesave::{
    Float, GameplayTagContainer, Properties, Property, Rotator, StructValue, ValueVec, Vector,
};

/// Marker trait for typed property access
pub trait TypedProperties: Sized {
    /// The expected struct type name (e.g., "RoomLinePoint")
    const STRUCT_TYPE: &'static str;

    fn from_properties(props: &Properties<AssetArchiveType>) -> Option<TypedPropertiesRef<'_, Self>>
    where
        Self: Sized,
    {
        Some(TypedPropertiesRef {
            props,
            _phantom: std::marker::PhantomData,
        })
    }

    fn from_properties_mut(
        props: &mut Properties<AssetArchiveType>,
    ) -> Option<TypedPropertiesMut<'_, Self>>
    where
        Self: Sized,
    {
        Some(TypedPropertiesMut {
            props,
            _phantom: std::marker::PhantomData,
        })
    }
}

pub struct TypedPropertiesRef<'a, T: TypedProperties> {
    props: &'a Properties<AssetArchiveType>,
    _phantom: std::marker::PhantomData<T>,
}
pub struct TypedPropertiesMut<'a, T: TypedProperties> {
    props: &'a mut Properties<AssetArchiveType>,
    _phantom: std::marker::PhantomData<T>,
}

impl<'a, T: TypedProperties> TypedPropertiesRef<'a, T> {
    pub fn properties(&self) -> &Properties<AssetArchiveType> {
        self.props
    }
    pub fn get<P>(&self, name: &str) -> &P
    where
        P: PropertyAccess,
    {
        P::get_ref(self.props, name)
    }
    pub fn try_get<P>(&self, name: &str) -> Option<&P>
    where
        P: PropertyAccess,
    {
        P::try_get_ref(self.props, name)
    }
}

impl<'a, T: TypedProperties> TypedPropertiesMut<'a, T> {
    pub fn properties(&self) -> &Properties<AssetArchiveType> {
        self.props
    }
    pub fn properties_mut(&mut self) -> &mut Properties<AssetArchiveType> {
        self.props
    }
    pub fn get<P>(&self, name: &str) -> &P
    where
        P: PropertyAccess,
    {
        P::get_ref(self.props, name)
    }
    pub fn get_mut<P>(&mut self, name: &str) -> &mut P
    where
        P: PropertyAccess,
    {
        P::get_mut(self.props, name)
    }
    pub fn try_get<P>(&self, name: &str) -> Option<&P>
    where
        P: PropertyAccess,
    {
        P::try_get_ref(self.props, name)
    }
    pub fn try_get_mut<P>(&mut self, name: &str) -> Option<&mut P>
    where
        P: PropertyAccess,
    {
        P::try_get_mut(self.props, name)
    }
}

/// Trait for types that can be accessed from Property enum
pub trait PropertyAccess: Sized {
    fn default_property() -> Property<AssetArchiveType>;
    fn get_ref<'a>(props: &'a Properties<AssetArchiveType>, name: &str) -> &'a Self;
    fn get_mut<'a>(props: &'a mut Properties<AssetArchiveType>, name: &str) -> &'a mut Self;
    fn try_get_ref<'a>(props: &'a Properties<AssetArchiveType>, name: &str) -> Option<&'a Self>;
    fn try_get_mut<'a>(
        props: &'a mut Properties<AssetArchiveType>,
        name: &str,
    ) -> Option<&'a mut Self>;
}

/// Generic typed array wrapper for struct arrays
pub struct TypedArray<'a, T: TypedProperties> {
    structs: &'a Vec<StructValue<AssetArchiveType>>,
    _phantom: std::marker::PhantomData<T>,
}

/// Mutable generic typed array wrapper for struct arrays
pub struct TypedArrayMut<'a, T: TypedProperties> {
    structs: &'a mut Vec<StructValue<AssetArchiveType>>,
    _phantom: std::marker::PhantomData<T>,
}

impl<'a, T: TypedProperties> TypedArray<'a, T> {
    /// Create from a ValueVec, returns None if not a Struct array
    pub fn from_value_vec(vec: &'a ValueVec<AssetArchiveType>) -> Option<Self> {
        match vec {
            ValueVec::Struct(structs) => Some(Self {
                structs,
                _phantom: std::marker::PhantomData,
            }),
            _ => None,
        }
    }

    /// Get the length of the array
    pub fn len(&self) -> usize {
        self.structs.len()
    }

    /// Check if the array is empty
    pub fn is_empty(&self) -> bool {
        self.structs.is_empty()
    }

    /// Get a typed view of an element at index
    pub fn get(&self, index: usize) -> Option<TypedPropertiesRef<'_, T>> {
        match self.structs.get(index) {
            Some(StructValue::Struct(props)) => T::from_properties(props),
            _ => None,
        }
    }

    /// Iterate over typed elements
    pub fn iter(&self) -> impl Iterator<Item = TypedPropertiesRef<'_, T>> {
        self.structs.iter().filter_map(|sv| match sv {
            StructValue::Struct(props) => T::from_properties(props),
            _ => None,
        })
    }
}

impl<'a, T: TypedProperties> TypedArrayMut<'a, T> {
    /// Create from a mutable ValueVec, returns None if not a Struct array
    pub fn from_value_vec(vec: &'a mut ValueVec<AssetArchiveType>) -> Option<Self> {
        match vec {
            ValueVec::Struct(structs) => Some(Self {
                structs,
                _phantom: std::marker::PhantomData,
            }),
            _ => None,
        }
    }

    /// Get the length of the array
    pub fn len(&self) -> usize {
        self.structs.len()
    }

    /// Check if the array is empty
    pub fn is_empty(&self) -> bool {
        self.structs.is_empty()
    }

    /// Get a mutable typed view of an element at index
    pub fn get_mut(&mut self, index: usize) -> Option<TypedPropertiesMut<'_, T>> {
        match self.structs.get_mut(index) {
            Some(StructValue::Struct(props)) => T::from_properties_mut(props),
            _ => None,
        }
    }

    /// Get an immutable typed view of an element at index
    pub fn get(&self, index: usize) -> Option<TypedPropertiesRef<'_, T>> {
        match self.structs.get(index) {
            Some(StructValue::Struct(props)) => T::from_properties(props),
            _ => None,
        }
    }

    /// Add a new element with default properties and return a mutable view of it
    pub fn push_default(&mut self) -> TypedPropertiesMut<'_, T> {
        self.structs
            .push(StructValue::Struct(Properties::default()));
        let last_idx = self.structs.len() - 1;
        self.get_mut(last_idx).unwrap()
    }

    /// Push an existing StructValue
    pub fn push(&mut self, value: StructValue<AssetArchiveType>) {
        self.structs.push(value);
    }

    /// Remove an element at index
    pub fn remove(&mut self, index: usize) -> StructValue<AssetArchiveType> {
        self.structs.remove(index)
    }

    /// Iterate over typed elements (immutable)
    pub fn iter(&self) -> impl Iterator<Item = TypedPropertiesRef<'_, T>> {
        self.structs.iter().filter_map(|sv| match sv {
            StructValue::Struct(props) => T::from_properties(props),
            _ => None,
        })
    }
}

// Implement PropertyAccess for Vector
impl PropertyAccess for Vector {
    fn default_property() -> Property<AssetArchiveType> {
        Property::Struct(StructValue::Vector(Vector {
            x: 0.0.into(),
            y: 0.0.into(),
            z: 0.0.into(),
        }))
    }

    fn get_ref<'a>(props: &'a Properties<AssetArchiveType>, name: &str) -> &'a Self {
        let key = uesave::PropertyKey::from(name);
        match props.0.get(&key) {
            Some(Property::Struct(StructValue::Vector(v))) => v,
            Some(_) => panic!("Property '{}' exists but is not a Vector", name),
            None => panic!("Property '{}' not found", name),
        }
    }

    fn get_mut<'a>(props: &'a mut Properties<AssetArchiveType>, name: &str) -> &'a mut Self {
        let key = uesave::PropertyKey::from(name);

        // Insert default if not present
        if !props.0.contains_key(&key) {
            props.0.insert(key.clone(), Self::default_property());
        }

        match props.0.get_mut(&key) {
            Some(Property::Struct(StructValue::Vector(v))) => v,
            Some(_) => panic!("Property '{}' exists but is not a Vector", name),
            None => unreachable!("Just inserted the property"),
        }
    }

    fn try_get_ref<'a>(props: &'a Properties<AssetArchiveType>, name: &str) -> Option<&'a Self> {
        let key = uesave::PropertyKey::from(name);
        match props.0.get(&key) {
            Some(Property::Struct(StructValue::Vector(v))) => Some(v),
            _ => None,
        }
    }

    fn try_get_mut<'a>(
        props: &'a mut Properties<AssetArchiveType>,
        name: &str,
    ) -> Option<&'a mut Self> {
        let key = uesave::PropertyKey::from(name);
        match props.0.get_mut(&key) {
            Some(Property::Struct(StructValue::Vector(v))) => Some(v),
            _ => None,
        }
    }
}

// Implement PropertyAccess for Rotator
impl PropertyAccess for Rotator {
    fn default_property() -> Property<AssetArchiveType> {
        Property::Struct(StructValue::Rotator(Rotator {
            x: 0.0.into(),
            y: 0.0.into(),
            z: 0.0.into(),
        }))
    }

    fn get_ref<'a>(props: &'a Properties<AssetArchiveType>, name: &str) -> &'a Self {
        let key = uesave::PropertyKey::from(name);
        match props.0.get(&key) {
            Some(Property::Struct(StructValue::Rotator(r))) => r,
            Some(_) => panic!("Property '{}' exists but is not a Rotator", name),
            None => panic!("Property '{}' not found", name),
        }
    }

    fn get_mut<'a>(props: &'a mut Properties<AssetArchiveType>, name: &str) -> &'a mut Self {
        let key = uesave::PropertyKey::from(name);

        // Insert default if not present
        if !props.0.contains_key(&key) {
            props.0.insert(key.clone(), Self::default_property());
        }

        match props.0.get_mut(&key) {
            Some(Property::Struct(StructValue::Rotator(r))) => r,
            Some(_) => panic!("Property '{}' exists but is not a Rotator", name),
            None => unreachable!("Just inserted the property"),
        }
    }

    fn try_get_ref<'a>(props: &'a Properties<AssetArchiveType>, name: &str) -> Option<&'a Self> {
        let key = uesave::PropertyKey::from(name);
        match props.0.get(&key) {
            Some(Property::Struct(StructValue::Rotator(r))) => Some(r),
            _ => None,
        }
    }

    fn try_get_mut<'a>(
        props: &'a mut Properties<AssetArchiveType>,
        name: &str,
    ) -> Option<&'a mut Self> {
        let key = uesave::PropertyKey::from(name);
        match props.0.get_mut(&key) {
            Some(Property::Struct(StructValue::Rotator(r))) => Some(r),
            _ => None,
        }
    }
}

// Implement PropertyAccess for Float (f32)
impl PropertyAccess for Float {
    fn default_property() -> Property<AssetArchiveType> {
        Property::Float(0.0.into())
    }

    fn get_ref<'a>(props: &'a Properties<AssetArchiveType>, name: &str) -> &'a Self {
        let key = uesave::PropertyKey::from(name);
        match props.0.get(&key) {
            Some(Property::Float(f)) => f,
            Some(_) => panic!("Property '{}' exists but is not a Float", name),
            None => panic!("Property '{}' not found", name),
        }
    }

    fn get_mut<'a>(props: &'a mut Properties<AssetArchiveType>, name: &str) -> &'a mut Self {
        let key = uesave::PropertyKey::from(name);

        // Insert default if not present
        if !props.0.contains_key(&key) {
            props.0.insert(key.clone(), Self::default_property());
        }

        match props.0.get_mut(&key) {
            Some(Property::Float(f)) => f,
            Some(_) => panic!("Property '{}' exists but is not a Float", name),
            None => unreachable!("Just inserted the property"),
        }
    }

    fn try_get_ref<'a>(props: &'a Properties<AssetArchiveType>, name: &str) -> Option<&'a Self> {
        let key = uesave::PropertyKey::from(name);
        match props.0.get(&key) {
            Some(Property::Float(f)) => Some(f),
            _ => None,
        }
    }

    fn try_get_mut<'a>(
        props: &'a mut Properties<AssetArchiveType>,
        name: &str,
    ) -> Option<&'a mut Self> {
        let key = uesave::PropertyKey::from(name);
        match props.0.get_mut(&key) {
            Some(Property::Float(f)) => Some(f),
            _ => None,
        }
    }
}

// Implement PropertyAccess for nested Properties (Struct properties)
impl PropertyAccess for Properties<AssetArchiveType> {
    fn default_property() -> Property<AssetArchiveType> {
        Property::Struct(StructValue::Struct(Properties::default()))
    }

    fn get_ref<'a>(props: &'a Properties<AssetArchiveType>, name: &str) -> &'a Self {
        let key = uesave::PropertyKey::from(name);
        match props.0.get(&key) {
            Some(Property::Struct(StructValue::Struct(s))) => s,
            Some(_) => panic!("Property '{}' exists but is not a Struct", name),
            None => panic!("Property '{}' not found", name),
        }
    }

    fn get_mut<'a>(props: &'a mut Properties<AssetArchiveType>, name: &str) -> &'a mut Self {
        let key = uesave::PropertyKey::from(name);

        // Insert default if not present
        if !props.0.contains_key(&key) {
            props.0.insert(key.clone(), Self::default_property());
        }

        match props.0.get_mut(&key) {
            Some(Property::Struct(StructValue::Struct(s))) => s,
            Some(_) => panic!("Property '{}' exists but is not a Struct", name),
            None => unreachable!("Just inserted the property"),
        }
    }

    fn try_get_ref<'a>(props: &'a Properties<AssetArchiveType>, name: &str) -> Option<&'a Self> {
        let key = uesave::PropertyKey::from(name);
        match props.0.get(&key) {
            Some(Property::Struct(StructValue::Struct(s))) => Some(s),
            _ => None,
        }
    }

    fn try_get_mut<'a>(
        props: &'a mut Properties<AssetArchiveType>,
        name: &str,
    ) -> Option<&'a mut Self> {
        let key = uesave::PropertyKey::from(name);
        match props.0.get_mut(&key) {
            Some(Property::Struct(StructValue::Struct(s))) => Some(s),
            _ => None,
        }
    }
}

// Implement PropertyAccess for Bool
impl PropertyAccess for bool {
    fn default_property() -> Property<AssetArchiveType> {
        Property::Bool(false)
    }

    fn get_ref<'a>(props: &'a Properties<AssetArchiveType>, name: &str) -> &'a Self {
        let key = uesave::PropertyKey::from(name);
        match props.0.get(&key) {
            Some(Property::Bool(b)) => b,
            Some(_) => panic!("Property '{}' exists but is not a Bool", name),
            None => panic!("Property '{}' not found", name),
        }
    }

    fn get_mut<'a>(props: &'a mut Properties<AssetArchiveType>, name: &str) -> &'a mut Self {
        let key = uesave::PropertyKey::from(name);

        // Insert default if not present
        if !props.0.contains_key(&key) {
            props.0.insert(key.clone(), Self::default_property());
        }

        match props.0.get_mut(&key) {
            Some(Property::Bool(b)) => b,
            Some(_) => panic!("Property '{}' exists but is not a Bool", name),
            None => unreachable!("Just inserted the property"),
        }
    }

    fn try_get_ref<'a>(props: &'a Properties<AssetArchiveType>, name: &str) -> Option<&'a Self> {
        let key = uesave::PropertyKey::from(name);
        match props.0.get(&key) {
            Some(Property::Bool(b)) => Some(b),
            _ => None,
        }
    }

    fn try_get_mut<'a>(
        props: &'a mut Properties<AssetArchiveType>,
        name: &str,
    ) -> Option<&'a mut Self> {
        let key = uesave::PropertyKey::from(name);
        match props.0.get_mut(&key) {
            Some(Property::Bool(b)) => Some(b),
            _ => None,
        }
    }
}

// Implement PropertyAccess for String (used for Enum properties)
impl PropertyAccess for String {
    fn default_property() -> Property<AssetArchiveType> {
        Property::Enum(String::new())
    }

    fn get_ref<'a>(props: &'a Properties<AssetArchiveType>, name: &str) -> &'a Self {
        let key = uesave::PropertyKey::from(name);
        match props.0.get(&key) {
            Some(Property::Enum(s)) => s,
            Some(_) => panic!("Property '{}' exists but is not an Enum", name),
            None => panic!("Property '{}' not found", name),
        }
    }

    fn get_mut<'a>(props: &'a mut Properties<AssetArchiveType>, name: &str) -> &'a mut Self {
        let key = uesave::PropertyKey::from(name);

        // Insert default if not present
        if !props.0.contains_key(&key) {
            props.0.insert(key.clone(), Self::default_property());
        }

        match props.0.get_mut(&key) {
            Some(Property::Enum(s)) => s,
            Some(_) => panic!("Property '{}' exists but is not an Enum", name),
            None => unreachable!("Just inserted the property"),
        }
    }

    fn try_get_ref<'a>(props: &'a Properties<AssetArchiveType>, name: &str) -> Option<&'a Self> {
        let key = uesave::PropertyKey::from(name);
        match props.0.get(&key) {
            Some(Property::Enum(s)) => Some(s),
            _ => None,
        }
    }

    fn try_get_mut<'a>(
        props: &'a mut Properties<AssetArchiveType>,
        name: &str,
    ) -> Option<&'a mut Self> {
        let key = uesave::PropertyKey::from(name);
        match props.0.get_mut(&key) {
            Some(Property::Enum(s)) => Some(s),
            _ => None,
        }
    }
}

// Implement PropertyAccess for ObjectRef
impl PropertyAccess for ObjectRef {
    fn default_property() -> Property<AssetArchiveType> {
        Property::Object(ObjectRef::Unloaded("/Script/CoreUObject.Object".into()))
    }

    fn get_ref<'a>(props: &'a Properties<AssetArchiveType>, name: &str) -> &'a Self {
        let key = uesave::PropertyKey::from(name);
        match props.0.get(&key) {
            Some(Property::Object(s)) => s,
            Some(_) => panic!("Property '{}' exists but is not an Object", name),
            None => panic!("Property '{}' not found", name),
        }
    }

    fn get_mut<'a>(props: &'a mut Properties<AssetArchiveType>, name: &str) -> &'a mut Self {
        let key = uesave::PropertyKey::from(name);

        // Insert default if not present
        if !props.0.contains_key(&key) {
            props.0.insert(key.clone(), Self::default_property());
        }

        match props.0.get_mut(&key) {
            Some(Property::Object(s)) => s,
            Some(_) => panic!("Property '{}' exists but is not an Object", name),
            None => unreachable!("Just inserted the property"),
        }
    }

    fn try_get_ref<'a>(props: &'a Properties<AssetArchiveType>, name: &str) -> Option<&'a Self> {
        let key = uesave::PropertyKey::from(name);
        match props.0.get(&key) {
            Some(Property::Object(s)) => Some(s),
            _ => None,
        }
    }

    fn try_get_mut<'a>(
        props: &'a mut Properties<AssetArchiveType>,
        name: &str,
    ) -> Option<&'a mut Self> {
        let key = uesave::PropertyKey::from(name);
        match props.0.get_mut(&key) {
            Some(Property::Object(s)) => Some(s),
            _ => None,
        }
    }
}

// Implement PropertyAccess for ValueVec (Array properties)
impl PropertyAccess for ValueVec<AssetArchiveType> {
    fn default_property() -> Property<AssetArchiveType> {
        // Default to empty Struct array (most common for our use case)
        Property::Array(ValueVec::Struct(Vec::new()))
    }

    fn get_ref<'a>(props: &'a Properties<AssetArchiveType>, name: &str) -> &'a Self {
        let key = uesave::PropertyKey::from(name);
        match props.0.get(&key) {
            Some(Property::Array(arr)) => arr,
            Some(_) => panic!("Property '{}' exists but is not an Array", name),
            None => panic!("Property '{}' not found", name),
        }
    }

    fn get_mut<'a>(props: &'a mut Properties<AssetArchiveType>, name: &str) -> &'a mut Self {
        let key = uesave::PropertyKey::from(name);

        // Insert default if not present
        if !props.0.contains_key(&key) {
            props.0.insert(key.clone(), Self::default_property());
        }

        match props.0.get_mut(&key) {
            Some(Property::Array(arr)) => arr,
            Some(_) => panic!("Property '{}' exists but is not an Array", name),
            None => unreachable!("Just inserted the property"),
        }
    }

    fn try_get_ref<'a>(props: &'a Properties<AssetArchiveType>, name: &str) -> Option<&'a Self> {
        let key = uesave::PropertyKey::from(name);
        match props.0.get(&key) {
            Some(Property::Array(arr)) => Some(arr),
            _ => None,
        }
    }

    fn try_get_mut<'a>(
        props: &'a mut Properties<AssetArchiveType>,
        name: &str,
    ) -> Option<&'a mut Self> {
        let key = uesave::PropertyKey::from(name);
        match props.0.get_mut(&key) {
            Some(Property::Array(arr)) => Some(arr),
            _ => None,
        }
    }
}

impl PropertyAccess for GameplayTagContainer {
    fn default_property() -> Property<AssetArchiveType> {
        Property::Struct(StructValue::GameplayTagContainer(GameplayTagContainer {
            gameplay_tags: Vec::new(),
        }))
    }

    fn get_ref<'a>(props: &'a Properties<AssetArchiveType>, name: &str) -> &'a Self {
        let key = uesave::PropertyKey::from(name);
        match props.0.get(&key) {
            Some(Property::Struct(StructValue::GameplayTagContainer(value))) => value,
            Some(_) => panic!(
                "Property '{}' exists but is not a GameplayTagContainer",
                name
            ),
            None => panic!("Property '{}' not found", name),
        }
    }

    fn get_mut<'a>(props: &'a mut Properties<AssetArchiveType>, name: &str) -> &'a mut Self {
        let key = uesave::PropertyKey::from(name);

        // Insert default if not present
        if !props.0.contains_key(&key) {
            props.0.insert(key.clone(), Self::default_property());
        }

        match props.0.get_mut(&key) {
            Some(Property::Struct(StructValue::GameplayTagContainer(value))) => value,
            Some(_) => panic!(
                "Property '{}' exists but is not a GameplayTagContainer",
                name
            ),
            None => unreachable!("Just inserted the property"),
        }
    }

    fn try_get_ref<'a>(props: &'a Properties<AssetArchiveType>, name: &str) -> Option<&'a Self> {
        let key = uesave::PropertyKey::from(name);
        match props.0.get(&key) {
            Some(Property::Struct(StructValue::GameplayTagContainer(value))) => Some(value),
            _ => None,
        }
    }

    fn try_get_mut<'a>(
        props: &'a mut Properties<AssetArchiveType>,
        name: &str,
    ) -> Option<&'a mut Self> {
        let key = uesave::PropertyKey::from(name);
        match props.0.get_mut(&key) {
            Some(Property::Struct(StructValue::GameplayTagContainer(value))) => Some(value),
            _ => None,
        }
    }
}

// Implement PropertyAccess for i32 (IntProperty)
impl PropertyAccess for i32 {
    fn default_property() -> Property<AssetArchiveType> {
        Property::Int(0)
    }

    fn get_ref<'a>(props: &'a Properties<AssetArchiveType>, name: &str) -> &'a Self {
        let key = uesave::PropertyKey::from(name);
        match props.0.get(&key) {
            Some(Property::Int(i)) => i,
            Some(_) => panic!("Property '{}' exists but is not an Int", name),
            None => panic!("Property '{}' not found", name),
        }
    }

    fn get_mut<'a>(props: &'a mut Properties<AssetArchiveType>, name: &str) -> &'a mut Self {
        let key = uesave::PropertyKey::from(name);

        // Insert default if not present
        if !props.0.contains_key(&key) {
            props.0.insert(key.clone(), Self::default_property());
        }

        match props.0.get_mut(&key) {
            Some(Property::Int(i)) => i,
            Some(_) => panic!("Property '{}' exists but is not an Int", name),
            None => unreachable!("Just inserted the property"),
        }
    }

    fn try_get_ref<'a>(props: &'a Properties<AssetArchiveType>, name: &str) -> Option<&'a Self> {
        let key = uesave::PropertyKey::from(name);
        match props.0.get(&key) {
            Some(Property::Int(i)) => Some(i),
            _ => None,
        }
    }

    fn try_get_mut<'a>(
        props: &'a mut Properties<AssetArchiveType>,
        name: &str,
    ) -> Option<&'a mut Self> {
        let key = uesave::PropertyKey::from(name);
        match props.0.get_mut(&key) {
            Some(Property::Int(i)) => Some(i),
            _ => None,
        }
    }
}
