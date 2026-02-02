use asset_ser::core::object_pool::{AssetArchiveType, ObjectRef};
use uesave::{
    Float, GameplayTagContainer, Properties, Property, Rotator, StructValue, ValueVec, Vector,
};

pub trait TypedProperties {
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

// Implement PropertyAccess for (String, i32) (SoftObjectPath for AssetArchiveType)
impl PropertyAccess for ObjectRef {
    fn default_property() -> Property<AssetArchiveType> {
        // TODO for lack of a better default
        Property::Object(ObjectRef::Unloaded("/Script/CoreUObject.Object".into()))
    }

    fn get_ref<'a>(props: &'a Properties<AssetArchiveType>, name: &str) -> &'a Self {
        let key = uesave::PropertyKey::from(name);
        match props.0.get(&key) {
            Some(Property::Object(s)) => s,
            Some(_) => panic!("Property '{}' exists but is not a SoftObject", name),
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
            Some(_) => panic!("Property '{}' exists but is not a SoftObject", name),
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

impl PropertyAccess for uesave::GameplayTagContainer {
    fn default_property() -> Property<AssetArchiveType> {
        Property::Struct(StructValue::GameplayTagContainer(
            uesave::GameplayTagContainer {
                gameplay_tags: Vec::new(),
            },
        ))
    }

    fn get_ref<'a>(props: &'a Properties<AssetArchiveType>, name: &str) -> &'a Self {
        let key = uesave::PropertyKey::from(name);
        match props.0.get(&key) {
            Some(Property::Struct(StructValue::GameplayTagContainer(value))) => value,
            Some(_) => panic!(
                "Property '{}' exists but is not a GameplayTagContainer (Set of Names)",
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
                "Property '{}' exists but is not a GameplayTagContainer (Set of Names)",
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

// Example: FRoomLinePoint wrapper
pub struct FRoomLinePoint;

impl TypedProperties for FRoomLinePoint {
    const STRUCT_TYPE: &'static str = "RoomLinePoint";
}

impl<'a> TypedPropertiesMut<'a, FRoomLinePoint> {
    pub fn location(&mut self) -> &mut Vector {
        self.get_mut("Location")
    }
    pub fn h_range(&mut self) -> &mut f32 {
        &mut self.get_mut::<Float>("HRange").0
    }
    pub fn v_range(&mut self) -> &mut f32 {
        &mut self.get_mut::<Float>("VRange").0
    }
    pub fn cieling_noise_range(&mut self) -> &mut f32 {
        &mut self.get_mut::<Float>("CielingNoiseRange").0
    }
    pub fn wall_noise_range(&mut self) -> &mut f32 {
        &mut self.get_mut::<Float>("WallNoiseRange").0
    }
    pub fn floor_noise_range(&mut self) -> &mut f32 {
        &mut self.get_mut::<Float>("FloorNoiseRange").0
    }
    pub fn cieling_height(&mut self) -> &mut f32 {
        &mut self.get_mut::<Float>("Cielingheight").0
    }
    pub fn height_scale(&mut self) -> &mut f32 {
        &mut self.get_mut::<Float>("HeightScale").0
    }
    pub fn floor_depth(&mut self) -> &mut f32 {
        &mut self.get_mut::<Float>("FloorDepth").0
    }
    pub fn floor_angle(&mut self) -> &mut f32 {
        &mut self.get_mut::<Float>("FloorAngle").0
    }
}

impl<'a> TypedPropertiesRef<'a, FRoomLinePoint> {
    pub fn location(&self) -> &Vector {
        self.get("Location")
    }
    pub fn h_range(&self) -> f32 {
        self.get::<Float>("HRange").0
    }
    pub fn v_range(&self) -> f32 {
        self.get::<Float>("VRange").0
    }
    pub fn cieling_noise_range(&self) -> f32 {
        self.get::<Float>("CielingNoiseRange").0
    }
    pub fn wall_noise_range(&self) -> f32 {
        self.get::<Float>("WallNoiseRange").0
    }
    pub fn floor_noise_range(&self) -> f32 {
        self.get::<Float>("FloorNoiseRange").0
    }
    pub fn cieling_height(&self) -> f32 {
        self.get::<Float>("Cielingheight").0
    }
    pub fn height_scale(&self) -> f32 {
        self.get::<Float>("HeightScale").0
    }
    pub fn floor_depth(&self) -> f32 {
        self.get::<Float>("FloorDepth").0
    }
    pub fn floor_angle(&self) -> f32 {
        self.get::<Float>("FloorAngle").0
    }
}

// Example: FRandRange wrapper
pub struct FRandRange;

impl TypedProperties for FRandRange {
    const STRUCT_TYPE: &'static str = "RandRange";
}

impl<'a> TypedPropertiesMut<'a, FRandRange> {
    /// Get mutable reference to Min property
    pub fn min(&mut self) -> &mut Float {
        self.get_mut("Min")
    }

    /// Get mutable reference to Max property
    pub fn max(&mut self) -> &mut Float {
        self.get_mut("Max")
    }
}

impl<'a> TypedPropertiesRef<'a, FRandRange> {
    /// Get reference to Min property
    pub fn min(&self) -> &Float {
        self.get("Min")
    }

    /// Get reference to Max property
    pub fn max(&self) -> &Float {
        self.get("Max")
    }
}

// FRandLinePoint wrapper
pub struct FRandLinePoint;

impl TypedProperties for FRandLinePoint {
    const STRUCT_TYPE: &'static str = "RandLinePoint";
}

impl<'a> TypedPropertiesMut<'a, FRandLinePoint> {
    pub fn location(&mut self) -> &mut Vector {
        self.get_mut("Location")
    }

    pub fn range(&mut self) -> TypedPropertiesMut<'_, FRandRange> {
        let props = self.get_mut::<Properties<AssetArchiveType>>("Range");
        FRandRange::from_properties_mut(props).expect("Range must be a RandRange struct")
    }

    pub fn noise_range(&mut self) -> TypedPropertiesMut<'_, FRandRange> {
        let props = self.get_mut::<Properties<AssetArchiveType>>("NoiseRange");
        FRandRange::from_properties_mut(props).expect("NoiseRange must be a RandRange struct")
    }

    pub fn skew_factor(&mut self) -> TypedPropertiesMut<'_, FRandRange> {
        let props = self.get_mut::<Properties<AssetArchiveType>>("SkewFactor");
        FRandRange::from_properties_mut(props).expect("SkewFactor must be a RandRange struct")
    }

    pub fn fill_amount(&mut self) -> TypedPropertiesMut<'_, FRandRange> {
        let props = self.get_mut::<Properties<AssetArchiveType>>("FillAmount");
        FRandRange::from_properties_mut(props).expect("FillAmount must be a RandRange struct")
    }
}

impl<'a> TypedPropertiesRef<'a, FRandLinePoint> {
    pub fn location(&self) -> &Vector {
        self.get("Location")
    }

    pub fn range(&self) -> TypedPropertiesRef<'_, FRandRange> {
        let props = self.get::<Properties<AssetArchiveType>>("Range");
        FRandRange::from_properties(props).expect("Range must be a RandRange struct")
    }

    pub fn noise_range(&self) -> TypedPropertiesRef<'_, FRandRange> {
        let props = self.get::<Properties<AssetArchiveType>>("NoiseRange");
        FRandRange::from_properties(props).expect("NoiseRange must be a RandRange struct")
    }

    pub fn skew_factor(&self) -> TypedPropertiesRef<'_, FRandRange> {
        let props = self.get::<Properties<AssetArchiveType>>("SkewFactor");
        FRandRange::from_properties(props).expect("SkewFactor must be a RandRange struct")
    }

    pub fn fill_amount(&self) -> TypedPropertiesRef<'_, FRandRange> {
        let props = self.get::<Properties<AssetArchiveType>>("FillAmount");
        FRandRange::from_properties(props).expect("FillAmount must be a RandRange struct")
    }
}

// FloodFillLine wrapper
pub struct UFloodFillLine;

impl TypedProperties for UFloodFillLine {
    const STRUCT_TYPE: &'static str = "FloodFillLine";
}

// FloodFillPillar wrapper
pub struct UFloodFillPillar;

impl TypedProperties for UFloodFillPillar {
    const STRUCT_TYPE: &'static str = "FloodFillPillar";
}

// FloodFillBox wrapper
pub struct UFloodFillBox;

impl TypedProperties for UFloodFillBox {
    const STRUCT_TYPE: &'static str = "FloodFillBox";
}

// EntranceFeature wrapper
pub struct UEntranceFeature;

impl TypedProperties for UEntranceFeature {
    const STRUCT_TYPE: &'static str = "EntranceFeature";
}

// SpawnActorFeature wrapper
pub struct USpawnActorFeature;

impl TypedProperties for USpawnActorFeature {
    const STRUCT_TYPE: &'static str = "SpawnActorFeature";
}

// DropPodCalldownLocationFeature wrapper
pub struct UDropPodCalldownLocationFeature;

impl TypedProperties for UDropPodCalldownLocationFeature {
    const STRUCT_TYPE: &'static str = "DropPodCalldownLocationFeature";
}

// ResourceFeature wrapper
pub struct UResourceFeature;

impl TypedProperties for UResourceFeature {
    const STRUCT_TYPE: &'static str = "ResourceFeature";
}

// RoomGenerator wrapper
pub struct URoomGenerator;

impl TypedProperties for URoomGenerator {
    const STRUCT_TYPE: &'static str = "RoomGenerator";
}

impl<'a> TypedPropertiesMut<'a, UFloodFillLine> {
    pub fn points(&mut self) -> TypedArrayMut<'_, FRoomLinePoint> {
        let vec = self.get_mut::<ValueVec<AssetArchiveType>>("Points");
        TypedArrayMut::from_value_vec(vec).expect("Points must be a Struct array")
    }
    pub fn wall_noise_override(&mut self) -> Option<&mut Properties<AssetArchiveType>> {
        self.try_get_mut("WallNoiseOverride")
    }
    pub fn ceiling_noise_override(&mut self) -> Option<&mut Properties<AssetArchiveType>> {
        self.try_get_mut("CeilingNoiseOverride")
    }
    pub fn floor_noise_override(&mut self) -> Option<&mut Properties<AssetArchiveType>> {
        self.try_get_mut("FloorNoiseOverride")
    }
    pub fn use_detail_noise(&mut self) -> Option<&mut bool> {
        self.try_get_mut("UseDetailNoise")
    }

    /// Get RoomFeatures as object array (creates if missing)
    /// Note: This is not in the standard FSD schema but appears in some assets
    pub fn room_features_objects(&mut self) -> &mut Vec<ObjectRef> {
        let key = uesave::PropertyKey::from("RoomFeatures");

        // Insert empty Object array if not present
        if !self.properties().0.contains_key(&key) {
            self.properties_mut()
                .0
                .insert(key.clone(), Property::Array(ValueVec::Object(Vec::new())));
        }

        match self.properties_mut().0.get_mut(&key) {
            Some(Property::Array(ValueVec::Object(refs))) => refs,
            Some(_) => panic!("Property 'RoomFeatures' exists but is not an Object array"),
            None => unreachable!("Just inserted the property"),
        }
    }
}

impl<'a> TypedPropertiesRef<'a, UFloodFillLine> {
    pub fn points(&self) -> TypedArray<'_, FRoomLinePoint> {
        let vec = self.get::<ValueVec<AssetArchiveType>>("Points");
        TypedArray::from_value_vec(vec).expect("Points must be a Struct array")
    }
    pub fn wall_noise_override(&self) -> Option<&Properties<AssetArchiveType>> {
        self.try_get("WallNoiseOverride")
    }
    pub fn ceiling_noise_override(&self) -> Option<&Properties<AssetArchiveType>> {
        self.try_get("CeilingNoiseOverride")
    }
    pub fn floor_noise_override(&self) -> Option<&Properties<AssetArchiveType>> {
        self.try_get("FloorNoiseOverride")
    }
    pub fn use_detail_noise(&self) -> Option<&bool> {
        self.try_get("UseDetailNoise")
    }
}

impl<'a> TypedPropertiesMut<'a, UFloodFillPillar> {
    pub fn points(&mut self) -> TypedArrayMut<'_, FRandLinePoint> {
        let vec = self.get_mut::<ValueVec<AssetArchiveType>>("Points");
        TypedArrayMut::from_value_vec(vec).expect("Points must be a Struct array")
    }

    pub fn noise_override(&mut self) -> Option<&mut Properties<AssetArchiveType>> {
        self.try_get_mut("NoiseOverride")
    }

    pub fn range_scale(&mut self) -> TypedPropertiesMut<'_, FRandRange> {
        let props = self.get_mut::<Properties<AssetArchiveType>>("RangeScale");
        FRandRange::from_properties_mut(props).expect("RangeScale must be a RandRange struct")
    }

    pub fn noise_range_scale(&mut self) -> TypedPropertiesMut<'_, FRandRange> {
        let props = self.get_mut::<Properties<AssetArchiveType>>("NoiseRangeScale");
        FRandRange::from_properties_mut(props).expect("NoiseRangeScale must be a RandRange struct")
    }

    pub fn endcap_scale(&mut self) -> TypedPropertiesMut<'_, FRandRange> {
        let props = self.get_mut::<Properties<AssetArchiveType>>("EndcapScale");
        FRandRange::from_properties_mut(props).expect("EndcapScale must be a RandRange struct")
    }
}

impl<'a> TypedPropertiesRef<'a, UFloodFillPillar> {
    pub fn points(&self) -> TypedArray<'_, FRandLinePoint> {
        let vec = self.get::<ValueVec<AssetArchiveType>>("Points");
        TypedArray::from_value_vec(vec).expect("Points must be a Struct array")
    }

    pub fn noise_override(&self) -> Option<&Properties<AssetArchiveType>> {
        self.try_get("NoiseOverride")
    }

    pub fn range_scale(&self) -> TypedPropertiesRef<'_, FRandRange> {
        let props = self.get::<Properties<AssetArchiveType>>("RangeScale");
        FRandRange::from_properties(props).expect("RangeScale must be a RandRange struct")
    }

    pub fn noise_range_scale(&self) -> TypedPropertiesRef<'_, FRandRange> {
        let props = self.get::<Properties<AssetArchiveType>>("NoiseRangeScale");
        FRandRange::from_properties(props).expect("NoiseRangeScale must be a RandRange struct")
    }

    pub fn endcap_scale(&self) -> TypedPropertiesRef<'_, FRandRange> {
        let props = self.get::<Properties<AssetArchiveType>>("EndcapScale");
        FRandRange::from_properties(props).expect("EndcapScale must be a RandRange struct")
    }
}

impl<'a> TypedPropertiesMut<'a, UFloodFillBox> {
    pub fn noise(&mut self) -> Option<&mut Properties<AssetArchiveType>> {
        self.try_get_mut("Noise")
    }

    pub fn position(&mut self) -> &mut Vector {
        self.get_mut("Position")
    }

    pub fn extends(&mut self) -> &mut Vector {
        self.get_mut("Extends")
    }

    pub fn rotation(&mut self) -> &mut Rotator {
        self.get_mut("Rotation")
    }

    pub fn is_carver(&mut self) -> &mut bool {
        self.get_mut("IsCarver")
    }

    pub fn noise_range(&mut self) -> &mut f32 {
        &mut self.get_mut::<Float>("NoiseRange").0
    }
}

impl<'a> TypedPropertiesRef<'a, UFloodFillBox> {
    pub fn noise(&self) -> Option<&Properties<AssetArchiveType>> {
        self.try_get("Noise")
    }

    pub fn position(&self) -> &Vector {
        self.get("Position")
    }

    pub fn extends(&self) -> &Vector {
        self.get("Extends")
    }

    pub fn rotation(&self) -> &Rotator {
        self.get("Rotation")
    }

    pub fn is_carver(&self) -> bool {
        *self.get("IsCarver")
    }

    pub fn noise_range(&self) -> f32 {
        self.get::<Float>("NoiseRange").0
    }
}

impl<'a> TypedPropertiesMut<'a, UEntranceFeature> {
    pub fn location(&mut self) -> &mut Vector {
        self.get_mut("Location")
    }

    pub fn direction(&mut self) -> &mut Rotator {
        self.get_mut("Direction")
    }

    pub fn entrance_type(&mut self) -> &mut String {
        self.get_mut("EntranceType")
    }

    pub fn priority(&mut self) -> &mut String {
        self.get_mut("Priority")
    }
}

impl<'a> TypedPropertiesRef<'a, UEntranceFeature> {
    pub fn location(&self) -> &Vector {
        self.get("Location")
    }

    pub fn direction(&self) -> &Rotator {
        self.get("Direction")
    }

    pub fn entrance_type(&self) -> &String {
        self.get("EntranceType")
    }

    pub fn priority(&self) -> &String {
        self.get("Priority")
    }
}

impl<'a> TypedPropertiesMut<'a, USpawnActorFeature> {
    pub fn location(&mut self) -> &mut Vector {
        self.get_mut("Location")
    }

    pub fn actor_to_spawn(&mut self) -> &mut ObjectRef {
        self.get_mut("ActorToSpawn")
    }

    pub fn adjustment_direction(&mut self) -> &mut Vector {
        self.get_mut("AdjustmentDirection")
    }

    pub fn adjustment(&mut self) -> &mut String {
        self.get_mut("Adjustment")
    }

    pub fn scale_min(&mut self) -> &mut Vector {
        self.get_mut("ScaleMin")
    }

    pub fn scale_max(&mut self) -> &mut Vector {
        self.get_mut("ScaleMax")
    }

    pub fn rotation_delta(&mut self) -> &mut Rotator {
        self.get_mut("RotationDelta")
    }
}

impl<'a> TypedPropertiesRef<'a, USpawnActorFeature> {
    pub fn location(&self) -> &Vector {
        self.get("Location")
    }

    pub fn actor_to_spawn(&self) -> Option<&ObjectRef> {
        self.try_get("ActorToSpawn")
    }

    pub fn adjustment_direction(&self) -> &Vector {
        self.get("AdjustmentDirection")
    }

    pub fn adjustment(&self) -> &String {
        self.get("Adjustment")
    }

    pub fn scale_min(&self) -> &Vector {
        self.get("ScaleMin")
    }

    pub fn scale_max(&self) -> &Vector {
        self.get("ScaleMax")
    }

    pub fn rotation_delta(&self) -> &Rotator {
        self.get("RotationDelta")
    }
}

impl<'a> TypedPropertiesMut<'a, UDropPodCalldownLocationFeature> {
    pub fn location(&mut self) -> &mut Vector {
        self.get_mut("Location")
    }

    pub fn calldown_class(&mut self) -> &mut ObjectRef {
        self.get_mut("CalldownClass")
    }
}

impl<'a> TypedPropertiesRef<'a, UDropPodCalldownLocationFeature> {
    pub fn location(&self) -> &Vector {
        self.get("Location")
    }

    pub fn calldown_class(&self) -> Option<&ObjectRef> {
        self.try_get("CalldownClass")
    }
}

impl<'a> TypedPropertiesMut<'a, UResourceFeature> {
    pub fn location(&mut self) -> &mut Vector {
        self.get_mut("Location")
    }

    pub fn resource(&mut self) -> Option<&mut Properties<AssetArchiveType>> {
        self.try_get_mut("Resource")
    }

    pub fn base_amount(&mut self) -> &mut f32 {
        &mut self.get_mut::<Float>("BaseAmount").0
    }
}

impl<'a> TypedPropertiesRef<'a, UResourceFeature> {
    pub fn location(&self) -> &Vector {
        self.get("Location")
    }

    pub fn resource(&self) -> Option<&Properties<AssetArchiveType>> {
        self.try_get("Resource")
    }

    pub fn base_amount(&self) -> f32 {
        self.get::<Float>("BaseAmount").0
    }
}

impl<'a> TypedPropertiesMut<'a, URoomGenerator> {
    /// Get mutable reference to Bounds property
    pub fn bounds(&mut self) -> &mut f32 {
        &mut self.get_mut::<Float>("Bounds").0
    }

    /// Get mutable reference to RoomFeatures array (creates Object array if missing)
    pub fn room_features(&mut self) -> &mut ValueVec<AssetArchiveType> {
        let key = uesave::PropertyKey::from("RoomFeatures");

        // Insert empty Object array if not present
        if !self.properties().0.contains_key(&key) {
            self.properties_mut()
                .0
                .insert(key.clone(), Property::Array(ValueVec::Object(Vec::new())));
        }

        match self.properties_mut().0.get_mut(&key) {
            Some(Property::Array(arr)) => arr,
            Some(_) => panic!("Property 'RoomFeatures' exists but is not an Array"),
            None => unreachable!("Just inserted the property"),
        }
    }

    /// Get RoomFeatures as object array
    pub fn room_features_objects(&mut self) -> &mut Vec<ObjectRef> {
        match self.room_features() {
            ValueVec::Object(refs) => refs,
            _ => panic!("RoomFeatures must be an Object array"),
        }
    }

    /// Try to get mutable reference to CanOnlyBeUsedOnce (optional property)
    pub fn can_only_be_used_once(&mut self) -> Option<&mut bool> {
        self.try_get_mut("CanOnlyBeUsedOnce")
    }

    /// Get mutable reference to RoomTags (creates empty set if missing)
    pub fn room_tags(&mut self) -> &mut GameplayTagContainer {
        self.get_mut("RoomTags")
    }

    /// Try to get mutable reference to RoomTags (optional property)
    pub fn try_room_tags(&mut self) -> Option<&mut GameplayTagContainer> {
        self.try_get_mut("RoomTags")
    }

    /// Get mutable reference to MirrorSupport enum (creates if missing)
    pub fn mirror_support(&mut self) -> &mut String {
        self.get_mut("MirrorSupport")
    }

    /// Try to get mutable reference to MirrorSupport (optional property)
    pub fn try_mirror_support(&mut self) -> Option<&mut String> {
        self.try_get_mut("MirrorSupport")
    }
}

impl<'a> TypedPropertiesRef<'a, URoomGenerator> {
    /// Get reference to Bounds property
    pub fn bounds(&self) -> f32 {
        self.get::<Float>("Bounds").0
    }

    /// Get reference to RoomFeatures array
    pub fn room_features(&self) -> &ValueVec<AssetArchiveType> {
        self.get("RoomFeatures")
    }

    /// Get RoomFeatures as object array
    pub fn room_features_objects(&self) -> Option<&Vec<ObjectRef>> {
        match self.room_features() {
            ValueVec::Object(refs) => Some(refs),
            _ => None,
        }
    }

    /// Try to get reference to CanOnlyBeUsedOnce (optional property)
    pub fn can_only_be_used_once(&self) -> Option<&bool> {
        self.try_get("CanOnlyBeUsedOnce")
    }

    /// Get reference to RoomTags
    pub fn room_tags(&self) -> &GameplayTagContainer {
        self.get("RoomTags")
    }

    /// Try to get reference to RoomTags (optional property)
    pub fn try_room_tags(&self) -> Option<&GameplayTagContainer> {
        self.try_get("RoomTags")
    }

    /// Get reference to MirrorSupport enum
    pub fn mirror_support(&self) -> &String {
        self.get("MirrorSupport")
    }

    /// Try to get reference to MirrorSupport (optional property)
    pub fn try_mirror_support(&self) -> Option<&String> {
        self.try_get("MirrorSupport")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use asset_ser::{
        core::object_pool::{LoadedObject, ObjectPool, ObjectRef},
        object::UObject,
        saver::asset_saver,
        util::printer::ObjectPrinter,
    };

    /// Helper to create a RoomLinePoint struct using typed properties
    /// Demonstrates auto-creation of properties on mutable access
    fn create_room_line_point(
        mut typed: TypedPropertiesMut<'_, FRoomLinePoint>,
        x: f32,
        y: f32,
        z: f32,
        h_range: f32,
        v_range: f32,
    ) {
        // Set location - auto-creates Location property with default Vector(0,0,0)
        typed.location().x = x.into();
        typed.location().y = y.into();
        typed.location().z = z.into();

        // Set ranges - auto-creates Float properties with default 0.0
        *typed.h_range() = h_range;
        *typed.v_range() = v_range;
        *typed.cieling_height() = v_range;

        // Set other properties with reasonable defaults
        *typed.cieling_noise_range() = 0.0;
        *typed.wall_noise_range() = 0.0;
        *typed.floor_noise_range() = 0.0;
        *typed.height_scale() = 1.0;
        *typed.floor_depth() = 0.0;
        *typed.floor_angle() = 0.0;
    }

    #[test]
    fn test_create_room_generator_with_typed_properties() -> anyhow::Result<()> {
        let mut pool = ObjectPool::new();

        // Create root RoomGenerator object
        let root_obj = LoadedObject {
            name: "RMA_Test".into(),
            outer: None,
            class: ObjectRef::Unloaded("/Script/FSD.RoomGenerator".into()),
            template: Some(ObjectRef::Unloaded(
                "/Script/FSD.Default__RoomGenerator".into(),
            )),
            object: Box::new(UObject::default()),
        };
        let root_handle = pool.allocate(root_obj);

        // Create a FloodFillLine with points using typed properties API
        let mut line_properties = Properties::default();

        // Use typed API to build the line - demonstrates auto-creation and TypedArray
        {
            let mut typed_line = UFloodFillLine::from_properties_mut(&mut line_properties).unwrap();

            // Access typed points array - auto-creates empty array
            let mut points = typed_line.points();

            // Add points using the TypedArray API
            create_room_line_point(points.push_default(), 0.0, 0.0, 0.0, 500.0, 400.0);
            create_room_line_point(points.push_default(), 1000.0, 0.0, 0.0, 600.0, 450.0);
            create_room_line_point(points.push_default(), 2000.0, 500.0, 0.0, 700.0, 500.0);
            create_room_line_point(points.push_default(), 3000.0, 1000.0, 0.0, 700.0, 2000.0);
        }

        let line_obj = LoadedObject {
            name: "FloodFillLine_Typed".into(),
            outer: Some(ObjectRef::Loaded(root_handle)),
            class: ObjectRef::Unloaded("/Script/FSD.FloodFillLine".into()),
            template: Some(ObjectRef::Unloaded(
                "/Script/FSD.Default__FloodFillLine".into(),
            )),
            object: Box::new(UObject {
                properties: line_properties,
                ..Default::default()
            }),
        };
        let line_handle = pool.allocate(line_obj);

        // Create a FloodFillPillar with points using typed properties API
        let mut pillar_properties = Properties::default();

        {
            let mut typed_pillar =
                UFloodFillPillar::from_properties_mut(&mut pillar_properties).unwrap();

            // Set range scales
            typed_pillar.range_scale().min().0 = 0.8;
            typed_pillar.range_scale().max().0 = 1.2;

            typed_pillar.noise_range_scale().min().0 = 0.5;
            typed_pillar.noise_range_scale().max().0 = 1.5;

            typed_pillar.endcap_scale().min().0 = 0.9;
            typed_pillar.endcap_scale().max().0 = 1.1;

            // Add pillar points
            let mut points = typed_pillar.points();

            // First point
            {
                let mut point = points.push_default();
                point.location().x = 500.0.into();
                point.location().y = 500.0.into();
                point.location().z = 0.0.into();

                point.range().min().0 = 100.0;
                point.range().max().0 = 150.0;

                point.noise_range().min().0 = 50.0;
                point.noise_range().max().0 = 80.0;

                point.skew_factor().min().0 = 0.8;
                point.skew_factor().max().0 = 1.2;

                point.fill_amount().min().0 = 0.7;
                point.fill_amount().max().0 = 0.9;
            }

            // Second point
            {
                let mut point = points.push_default();
                point.location().x = 500.0.into();
                point.location().y = 500.0.into();
                point.location().z = 800.0.into();

                point.range().min().0 = 120.0;
                point.range().max().0 = 180.0;

                point.noise_range().min().0 = 60.0;
                point.noise_range().max().0 = 100.0;

                point.skew_factor().min().0 = 0.9;
                point.skew_factor().max().0 = 1.1;

                point.fill_amount().min().0 = 0.8;
                point.fill_amount().max().0 = 1.0;
            }
        }

        let pillar_obj = LoadedObject {
            name: "FloodFillPillar_Typed".into(),
            outer: Some(ObjectRef::Loaded(root_handle)),
            class: ObjectRef::Unloaded("/Script/FSD.FloodFillPillar".into()),
            template: Some(ObjectRef::Unloaded(
                "/Script/FSD.Default__FloodFillPillar".into(),
            )),
            object: Box::new(UObject {
                properties: pillar_properties,
                ..Default::default()
            }),
        };
        let pillar_handle = pool.allocate(pillar_obj);

        // Create an EntranceFeature using typed properties API
        let mut entrance_properties = Properties::default();

        {
            let mut typed_entrance =
                UEntranceFeature::from_properties_mut(&mut entrance_properties).unwrap();

            // Set location
            typed_entrance.location().x = 0.0.into();
            typed_entrance.location().y = 0.0.into();
            typed_entrance.location().z = 0.0.into();

            // Set direction (rotator)
            typed_entrance.direction().x = 0.0.into();
            typed_entrance.direction().y = 90.0.into(); // Yaw
            typed_entrance.direction().z = 0.0.into();

            // Set entrance type: EntranceAndExit, Entrance, Exit, or TreassureRoom
            *typed_entrance.entrance_type() = "ECaveEntranceType::EntranceAndExit".to_string();

            // Set priority: Primary or Secondary
            *typed_entrance.priority() = "ECaveEntrancePriority::Primary".to_string();
        }

        let entrance_obj = LoadedObject {
            name: "EntranceFeature_Typed".into(),
            outer: Some(ObjectRef::Loaded(root_handle)),
            class: ObjectRef::Unloaded("/Script/FSD.EntranceFeature".into()),
            template: Some(ObjectRef::Unloaded(
                "/Script/FSD.Default__EntranceFeature".into(),
            )),
            object: Box::new(UObject {
                properties: entrance_properties,
                ..Default::default()
            }),
        };
        let entrance_handle = pool.allocate(entrance_obj);

        // Create a SpawnActorFeature using typed properties API
        let mut spawn_actor_properties = Properties::default();

        {
            let mut typed_spawn =
                USpawnActorFeature::from_properties_mut(&mut spawn_actor_properties).unwrap();

            // Set location where actor will spawn
            typed_spawn.location().x = 1000.0.into();
            typed_spawn.location().y = 1000.0.into();
            typed_spawn.location().z = 0.0.into();

            // Set actor to spawn
            // *typed_spawn.actor_to_spawn() =
            //     ObjectRef::Unloaded("/Game/Items/SomeActor.SomeActor_C".into());

            // Set adjustment direction (direction to adjust spawn position)
            typed_spawn.adjustment_direction().x = 0.0.into();
            typed_spawn.adjustment_direction().y = 0.0.into();
            typed_spawn.adjustment_direction().z = (-1.0).into();

            // Set adjustment type: None, Cieling, Wall, or Floor
            *typed_spawn.adjustment() = "EItemAdjustmentType::Floor".to_string();

            // Set scale range
            typed_spawn.scale_min().x = 0.8.into();
            typed_spawn.scale_min().y = 0.8.into();
            typed_spawn.scale_min().z = 0.8.into();

            typed_spawn.scale_max().x = 1.2.into();
            typed_spawn.scale_max().y = 1.2.into();
            typed_spawn.scale_max().z = 1.2.into();

            // Set rotation delta
            typed_spawn.rotation_delta().x = 0.0.into();
            typed_spawn.rotation_delta().y = 0.0.into();
            typed_spawn.rotation_delta().z = 0.0.into();
        }

        let spawn_actor_obj = LoadedObject {
            name: "SpawnActorFeature_Typed".into(),
            outer: Some(ObjectRef::Loaded(root_handle)),
            class: ObjectRef::Unloaded("/Script/FSD.SpawnActorFeature".into()),
            template: Some(ObjectRef::Unloaded(
                "/Script/FSD.Default__SpawnActorFeature".into(),
            )),
            object: Box::new(UObject {
                properties: spawn_actor_properties,
                ..Default::default()
            }),
        };
        let spawn_actor_handle = pool.allocate(spawn_actor_obj);

        // Create a DropPodCalldownLocationFeature using typed properties API
        let mut drop_pod_properties = Properties::default();

        {
            let mut typed_drop_pod =
                UDropPodCalldownLocationFeature::from_properties_mut(&mut drop_pod_properties)
                    .unwrap();

            // Set location for drop pod calldown
            typed_drop_pod.location().x = 500.0.into();
            typed_drop_pod.location().y = (-500.0).into();
            typed_drop_pod.location().z = 0.0.into();

            // Set calldown class (optional, but auto-creates on mutable access)
            // *typed_drop_pod.calldown_class() =
            //     ObjectRef::Unloaded("/Game/DropPod/BP_DropPod.BP_DropPod_C".into());
        }

        let drop_pod_obj = LoadedObject {
            name: "DropPodCalldownLocation_Typed".into(),
            outer: Some(ObjectRef::Loaded(root_handle)),
            class: ObjectRef::Unloaded("/Script/FSD.DropPodCalldownLocationFeature".into()),
            template: Some(ObjectRef::Unloaded(
                "/Script/FSD.Default__DropPodCalldownLocationFeature".into(),
            )),
            object: Box::new(UObject {
                properties: drop_pod_properties,
                ..Default::default()
            }),
        };
        let drop_pod_handle = pool.allocate(drop_pod_obj);

        // Create a FloodFillBox using typed properties API
        let mut box_properties = Properties::default();

        {
            let mut typed_box = UFloodFillBox::from_properties_mut(&mut box_properties).unwrap();

            // Set position (center of the box)
            typed_box.position().x = 2000.0.into();
            typed_box.position().y = 2000.0.into();
            typed_box.position().z = 400.0.into();

            // Set extends (half-size of the box)
            typed_box.extends().x = 500.0.into();
            typed_box.extends().y = 500.0.into();
            typed_box.extends().z = 300.0.into();

            // Set rotation
            typed_box.rotation().x = 0.0.into();
            typed_box.rotation().y = 45.0.into(); // Rotate 45 degrees
            typed_box.rotation().z = 0.0.into();

            // Set whether this box carves out space (removes terrain) or adds it
            *typed_box.is_carver() = false;

            // Set noise range for procedural variation
            *typed_box.noise_range() = 100.0;

            // Noise settings are optional - not setting them here
        }

        let box_obj = LoadedObject {
            name: "FloodFillBox_Typed".into(),
            outer: Some(ObjectRef::Loaded(root_handle)),
            class: ObjectRef::Unloaded("/Script/FSD.FloodFillBox".into()),
            template: Some(ObjectRef::Unloaded(
                "/Script/FSD.Default__FloodFillBox".into(),
            )),
            object: Box::new(UObject {
                properties: box_properties,
                ..Default::default()
            }),
        };
        let box_handle = pool.allocate(box_obj);

        // Create a ResourceFeature using typed properties API
        let mut resource_properties = Properties::default();

        {
            let mut typed_resource =
                UResourceFeature::from_properties_mut(&mut resource_properties).unwrap();

            // Set location where resource will spawn
            typed_resource.location().x = 1500.0.into();
            typed_resource.location().y = 1500.0.into();
            typed_resource.location().z = 200.0.into();

            // Set base amount of resource (e.g., minerals/gold)
            *typed_resource.base_amount() = 250.0;

            // Resource data is optional - UResourceData is an empty struct
            // Not setting it here as it doesn't contain any fields
        }

        let resource_obj = LoadedObject {
            name: "ResourceFeature_Typed".into(),
            outer: Some(ObjectRef::Loaded(root_handle)),
            class: ObjectRef::Unloaded("/Script/FSD.ResourceFeature".into()),
            template: Some(ObjectRef::Unloaded(
                "/Script/FSD.Default__ResourceFeature".into(),
            )),
            object: Box::new(UObject {
                properties: resource_properties,
                ..Default::default()
            }),
        };
        let _resource_handle = pool.allocate(resource_obj);

        // Add all features to the root's RoomFeatures using typed properties
        let root = pool.get_mut(root_handle).unwrap();
        let mut typed_root = URoomGenerator::from_properties_mut(root.properties_mut()).unwrap();

        // Set bounds using typed API (auto-creates if missing)
        *typed_root.bounds() = 5000.0;

        // Set room tags using typed API
        let tags = &mut typed_root.room_tags().gameplay_tags;
        tags.push(uesave::GameplayTag {
            name: "Rooms.Cave.Test".to_string(),
        });
        tags.push(uesave::GameplayTag {
            name: "Rooms.Large".to_string(),
        });
        tags.push(uesave::GameplayTag {
            name: "Rooms.Complex".to_string(),
        });

        // Set mirror support
        *typed_root.mirror_support() = "ERoomMirroringSupport::MirrorBoth".to_string();

        // Add room features using typed API (auto-creates empty Object array)
        let features = typed_root.room_features_objects();
        features.push(ObjectRef::Loaded(line_handle));
        features.push(ObjectRef::Loaded(pillar_handle));
        features.push(ObjectRef::Loaded(entrance_handle));
        features.push(ObjectRef::Loaded(spawn_actor_handle));
        features.push(ObjectRef::Loaded(drop_pod_handle));
        features.push(ObjectRef::Loaded(box_handle));
        // features.push(ObjectRef::Loaded(resource_handle));

        println!(
            "{}",
            ObjectPrinter::new(&pool).print_object(root_handle).unwrap()
        );

        // Save the asset
        let output_path = std::path::Path::new(
            "new_mod_P/FSD/Content/_AssemblyStorm/SandboxUtilities/MapGen/RMA_Test.uasset",
        );

        let jmap_path = std::path::Path::new("fsd.jmap");
        let jmap_data = std::fs::read_to_string(jmap_path)?;
        let jmap: jmap::Jmap = serde_json::from_str(&jmap_data)?;

        println!("Saving asset to {:?}", output_path);

        asset_saver::save_asset(
            output_path,
            &pool,
            vec![root_handle],
            asset_ser::AssetVersionInfo {
                package_file_version_ue4: 522,
                package_file_version_ue5: 0,
                engine_version_major: 4,
                engine_version_minor: 27,
                engine_version_patch: 0,
            },
            "RMA_Test".to_string(),
            &jmap,
        )?;

        Ok(())
    }
}
