//! RMA (Room Generator) types using asset_ser's TypedProperties pattern
//!
//! This module provides typed accessors for Deep Rock Galactic room generator assets.

pub use crate::typed_properties::{
    TypedArray, TypedArrayMut, TypedProperties, TypedPropertiesMut, TypedPropertiesRef,
};
use asset_ser::core::object_pool::{AssetArchiveType, ObjectHandle, ObjectPool, ObjectRef};
use ordered_float::OrderedFloat;
use serde::Serialize;
use uesave::{Float, GameplayTagContainer, Properties, Property, Rotator, ValueVec, Vector};

// ============================================================================
// Primitive Types
// ============================================================================

/// Wrapper around uesave::Vector for compatibility with existing code
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct FVector {
    pub x: OrderedFloat<f32>,
    pub y: OrderedFloat<f32>,
    pub z: OrderedFloat<f32>,
}

impl From<&Vector> for FVector {
    fn from(v: &Vector) -> Self {
        Self {
            x: (v.x.0 as f32).into(),
            y: (v.y.0 as f32).into(),
            z: (v.z.0 as f32).into(),
        }
    }
}

impl From<FVector> for Vector {
    fn from(v: FVector) -> Self {
        Self {
            x: (v.x.0 as f64).into(),
            y: (v.y.0 as f64).into(),
            z: (v.z.0 as f64).into(),
        }
    }
}

/// Wrapper around uesave::Rotator for compatibility with existing code
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct FRotator {
    pub pitch: OrderedFloat<f32>,
    pub yaw: OrderedFloat<f32>,
    pub roll: OrderedFloat<f32>,
}

impl From<&Rotator> for FRotator {
    fn from(r: &Rotator) -> Self {
        Self {
            pitch: (r.x.0 as f32).into(),
            yaw: (r.y.0 as f32).into(),
            roll: (r.z.0 as f32).into(),
        }
    }
}

impl From<FRotator> for Rotator {
    fn from(r: FRotator) -> Self {
        Self {
            x: (r.pitch.0 as f64).into(),
            y: (r.yaw.0 as f64).into(),
            z: (r.roll.0 as f64).into(),
        }
    }
}

/// Transform (translation + rotation + scale)
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[allow(non_snake_case)]
pub struct FTransform {
    pub translation: FVector,
    pub rotation: FQuat,
    pub Scale3D: FVector,
}

/// Quaternion rotation
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct FQuat {
    pub x: OrderedFloat<f32>,
    pub y: OrderedFloat<f32>,
    pub z: OrderedFloat<f32>,
    pub w: OrderedFloat<f32>,
}

impl FQuat {
    pub fn from_euler(pitch: f32, yaw: f32, roll: f32) -> Self {
        // Convert degrees to radians
        let pitch = pitch.to_radians() * 0.5;
        let yaw = yaw.to_radians() * 0.5;
        let roll = roll.to_radians() * 0.5;

        let (sp, cp) = pitch.sin_cos();
        let (sy, cy) = yaw.sin_cos();
        let (sr, cr) = roll.sin_cos();

        Self {
            x: (cr * sp * cy + sr * cp * sy).into(),
            y: (cr * cp * sy - sr * sp * cy).into(),
            z: (sr * cp * cy - cr * sp * sy).into(),
            w: (cr * cp * cy + sr * sp * sy).into(),
        }
    }

    pub fn to_euler(&self) -> (f32, f32, f32) {
        let sinr_cosp = 2.0 * (self.w.0 * self.x.0 + self.y.0 * self.z.0);
        let cosr_cosp = 1.0 - 2.0 * (self.x.0 * self.x.0 + self.y.0 * self.y.0);
        let roll = sinr_cosp.atan2(cosr_cosp);

        let sinp = 2.0 * (self.w.0 * self.y.0 - self.z.0 * self.x.0);
        let pitch = if sinp.abs() >= 1.0 {
            std::f32::consts::FRAC_PI_2.copysign(sinp)
        } else {
            sinp.asin()
        };

        let siny_cosp = 2.0 * (self.w.0 * self.z.0 + self.x.0 * self.y.0);
        let cosy_cosp = 1.0 - 2.0 * (self.y.0 * self.y.0 + self.z.0 * self.z.0);
        let yaw = siny_cosp.atan2(cosy_cosp);

        (pitch.to_degrees(), yaw.to_degrees(), roll.to_degrees())
    }
}

// ============================================================================
// TypedProperties Marker Structs
// ============================================================================

/// FRandRange - min/max float range
pub struct FRandRange;

impl TypedProperties for FRandRange {
    const STRUCT_TYPE: &'static str = "RandRange";
}

impl<'a> TypedPropertiesRef<'a, FRandRange> {
    pub fn min(&self) -> f32 {
        self.try_get::<Float>("Min").map(|f| f.0).unwrap_or(0.0)
    }
    pub fn max(&self) -> f32 {
        self.try_get::<Float>("Max").map(|f| f.0).unwrap_or(0.0)
    }
}

impl<'a> TypedPropertiesMut<'a, FRandRange> {
    pub fn min(&mut self) -> &mut f32 {
        &mut self.get_mut::<Float>("Min").0
    }
    pub fn max(&mut self) -> &mut f32 {
        &mut self.get_mut::<Float>("Max").0
    }
}

/// FRandLinePoint - pillar point with ranges
pub struct FRandLinePoint;

impl TypedProperties for FRandLinePoint {
    const STRUCT_TYPE: &'static str = "RandLinePoint";
}

impl<'a> TypedPropertiesRef<'a, FRandLinePoint> {
    pub fn location(&self) -> FVector {
        self.try_get::<Vector>("Location")
            .map(FVector::from)
            .unwrap_or_default()
    }
    pub fn range(&self) -> TypedPropertiesRef<'_, FRandRange> {
        let props = self.get::<Properties<AssetArchiveType>>("Range");
        FRandRange::from_properties(props).unwrap()
    }
    pub fn noise_range(&self) -> TypedPropertiesRef<'_, FRandRange> {
        let props = self.get::<Properties<AssetArchiveType>>("NoiseRange");
        FRandRange::from_properties(props).unwrap()
    }
    pub fn skew_factor(&self) -> TypedPropertiesRef<'_, FRandRange> {
        let props = self.get::<Properties<AssetArchiveType>>("SkewFactor");
        FRandRange::from_properties(props).unwrap()
    }
    pub fn fill_amount(&self) -> TypedPropertiesRef<'_, FRandRange> {
        let props = self.get::<Properties<AssetArchiveType>>("FillAmount");
        FRandRange::from_properties(props).unwrap()
    }
}

impl<'a> TypedPropertiesMut<'a, FRandLinePoint> {
    pub fn location(&mut self) -> &mut Vector {
        self.get_mut("Location")
    }
    pub fn range(&mut self) -> TypedPropertiesMut<'_, FRandRange> {
        let props = self.get_mut::<Properties<AssetArchiveType>>("Range");
        FRandRange::from_properties_mut(props).unwrap()
    }
    pub fn noise_range(&mut self) -> TypedPropertiesMut<'_, FRandRange> {
        let props = self.get_mut::<Properties<AssetArchiveType>>("NoiseRange");
        FRandRange::from_properties_mut(props).unwrap()
    }
    pub fn skew_factor(&mut self) -> TypedPropertiesMut<'_, FRandRange> {
        let props = self.get_mut::<Properties<AssetArchiveType>>("SkewFactor");
        FRandRange::from_properties_mut(props).unwrap()
    }
    pub fn fill_amount(&mut self) -> TypedPropertiesMut<'_, FRandRange> {
        let props = self.get_mut::<Properties<AssetArchiveType>>("FillAmount");
        FRandRange::from_properties_mut(props).unwrap()
    }
}

/// FRoomLinePoint - line point with ranges
pub struct FRoomLinePoint;

impl TypedProperties for FRoomLinePoint {
    const STRUCT_TYPE: &'static str = "RoomLinePoint";
}

impl<'a> TypedPropertiesRef<'a, FRoomLinePoint> {
    pub fn location(&self) -> FVector {
        self.try_get::<Vector>("Location")
            .map(FVector::from)
            .unwrap_or_default()
    }
    pub fn h_range(&self) -> f32 {
        self.try_get::<Float>("HRange").map(|f| f.0).unwrap_or(0.0)
    }
    pub fn v_range(&self) -> f32 {
        self.try_get::<Float>("VRange").map(|f| f.0).unwrap_or(0.0)
    }
    pub fn cieling_noise_range(&self) -> f32 {
        self.try_get::<Float>("CielingNoiseRange")
            .map(|f| f.0)
            .unwrap_or(0.0)
    }
    pub fn wall_noise_range(&self) -> f32 {
        self.try_get::<Float>("WallNoiseRange")
            .map(|f| f.0)
            .unwrap_or(0.0)
    }
    pub fn floor_noise_range(&self) -> f32 {
        self.try_get::<Float>("FloorNoiseRange")
            .map(|f| f.0)
            .unwrap_or(0.0)
    }
    pub fn cieling_height(&self) -> f32 {
        self.try_get::<Float>("Cielingheight")
            .map(|f| f.0)
            .unwrap_or(0.0)
    }
    pub fn height_scale(&self) -> f32 {
        self.try_get::<Float>("HeightScale")
            .map(|f| f.0)
            .unwrap_or(1.0)
    }
    pub fn floor_depth(&self) -> f32 {
        self.try_get::<Float>("FloorDepth")
            .map(|f| f.0)
            .unwrap_or(0.0)
    }
    pub fn floor_angle(&self) -> f32 {
        self.try_get::<Float>("FloorAngle")
            .map(|f| f.0)
            .unwrap_or(0.0)
    }
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

// ============================================================================
// Room Feature TypedProperties Markers
// ============================================================================

/// FloodFillBox feature
pub struct UFloodFillBox;

impl TypedProperties for UFloodFillBox {
    const STRUCT_TYPE: &'static str = "FloodFillBox";
}

impl<'a> TypedPropertiesRef<'a, UFloodFillBox> {
    pub fn position(&self) -> FVector {
        self.try_get::<Vector>("Position")
            .map(FVector::from)
            .unwrap_or_default()
    }
    pub fn extends(&self) -> FVector {
        self.try_get::<Vector>("Extends")
            .map(FVector::from)
            .unwrap_or_default()
    }
    pub fn rotation(&self) -> FRotator {
        self.try_get::<Rotator>("Rotation")
            .map(FRotator::from)
            .unwrap_or_default()
    }
    pub fn is_carver(&self) -> bool {
        self.try_get::<bool>("IsCarver").copied().unwrap_or(false)
    }
    pub fn noise_range(&self) -> f32 {
        self.try_get::<Float>("NoiseRange")
            .map(|f| f.0)
            .unwrap_or(0.0)
    }
    pub fn room_features(&self) -> Option<&Vec<ObjectRef>> {
        self.try_get::<ValueVec<AssetArchiveType>>("RoomFeatures")
            .and_then(|v| match v {
                ValueVec::Object(refs) => Some(refs),
                _ => None,
            })
    }
}

impl<'a> TypedPropertiesMut<'a, UFloodFillBox> {
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
    pub fn room_features_objects(&mut self) -> &mut Vec<ObjectRef> {
        let key = uesave::PropertyKey::from("RoomFeatures");
        if !self.properties().0.contains_key(&key) {
            self.properties_mut()
                .0
                .insert(key.clone(), Property::Array(ValueVec::Object(Vec::new())));
        }
        match self.properties_mut().0.get_mut(&key) {
            Some(Property::Array(ValueVec::Object(refs))) => refs,
            _ => panic!("RoomFeatures must be an Object array"),
        }
    }
}

/// FloodFillPillar feature
pub struct UFloodFillPillar;

impl TypedProperties for UFloodFillPillar {
    const STRUCT_TYPE: &'static str = "FloodFillPillar";
}

impl<'a> TypedPropertiesRef<'a, UFloodFillPillar> {
    pub fn points(&self) -> TypedArray<'_, FRandLinePoint> {
        let vec = self.get::<ValueVec<AssetArchiveType>>("Points");
        TypedArray::from_value_vec(vec).expect("Points must be a Struct array")
    }
    pub fn range_scale(&self) -> TypedPropertiesRef<'_, FRandRange> {
        let props = self.get::<Properties<AssetArchiveType>>("RangeScale");
        FRandRange::from_properties(props).unwrap()
    }
    pub fn noise_range_scale(&self) -> TypedPropertiesRef<'_, FRandRange> {
        let props = self.get::<Properties<AssetArchiveType>>("NoiseRangeScale");
        FRandRange::from_properties(props).unwrap()
    }
    pub fn endcap_scale(&self) -> TypedPropertiesRef<'_, FRandRange> {
        let props = self.get::<Properties<AssetArchiveType>>("EndcapScale");
        FRandRange::from_properties(props).unwrap()
    }
    pub fn room_features(&self) -> Option<&Vec<ObjectRef>> {
        self.try_get::<ValueVec<AssetArchiveType>>("RoomFeatures")
            .and_then(|v| match v {
                ValueVec::Object(refs) => Some(refs),
                _ => None,
            })
    }
}

impl<'a> TypedPropertiesMut<'a, UFloodFillPillar> {
    pub fn points(&mut self) -> TypedArrayMut<'_, FRandLinePoint> {
        let vec = self.get_mut::<ValueVec<AssetArchiveType>>("Points");
        TypedArrayMut::from_value_vec(vec).expect("Points must be a Struct array")
    }
    pub fn range_scale(&mut self) -> TypedPropertiesMut<'_, FRandRange> {
        let props = self.get_mut::<Properties<AssetArchiveType>>("RangeScale");
        FRandRange::from_properties_mut(props).unwrap()
    }
    pub fn noise_range_scale(&mut self) -> TypedPropertiesMut<'_, FRandRange> {
        let props = self.get_mut::<Properties<AssetArchiveType>>("NoiseRangeScale");
        FRandRange::from_properties_mut(props).unwrap()
    }
    pub fn endcap_scale(&mut self) -> TypedPropertiesMut<'_, FRandRange> {
        let props = self.get_mut::<Properties<AssetArchiveType>>("EndcapScale");
        FRandRange::from_properties_mut(props).unwrap()
    }
    pub fn room_features_objects(&mut self) -> &mut Vec<ObjectRef> {
        let key = uesave::PropertyKey::from("RoomFeatures");
        if !self.properties().0.contains_key(&key) {
            self.properties_mut()
                .0
                .insert(key.clone(), Property::Array(ValueVec::Object(Vec::new())));
        }
        match self.properties_mut().0.get_mut(&key) {
            Some(Property::Array(ValueVec::Object(refs))) => refs,
            _ => panic!("RoomFeatures must be an Object array"),
        }
    }
}

/// FloodFillLine feature
pub struct UFloodFillLine;

impl TypedProperties for UFloodFillLine {
    const STRUCT_TYPE: &'static str = "FloodFillLine";
}

impl<'a> TypedPropertiesRef<'a, UFloodFillLine> {
    pub fn points(&self) -> TypedArray<'_, FRoomLinePoint> {
        let vec = self.get::<ValueVec<AssetArchiveType>>("Points");
        TypedArray::from_value_vec(vec).expect("Points must be a Struct array")
    }
    pub fn use_detail_noise(&self) -> bool {
        self.try_get::<bool>("UseDetailNoise")
            .copied()
            .unwrap_or(false)
    }
    pub fn room_features(&self) -> Option<&Vec<ObjectRef>> {
        self.try_get::<ValueVec<AssetArchiveType>>("RoomFeatures")
            .and_then(|v| match v {
                ValueVec::Object(refs) => Some(refs),
                _ => None,
            })
    }
}

impl<'a> TypedPropertiesMut<'a, UFloodFillLine> {
    pub fn points(&mut self) -> TypedArrayMut<'_, FRoomLinePoint> {
        let vec = self.get_mut::<ValueVec<AssetArchiveType>>("Points");
        TypedArrayMut::from_value_vec(vec).expect("Points must be a Struct array")
    }
    pub fn use_detail_noise(&mut self) -> &mut bool {
        self.get_mut("UseDetailNoise")
    }
    pub fn room_features_objects(&mut self) -> &mut Vec<ObjectRef> {
        let key = uesave::PropertyKey::from("RoomFeatures");
        if !self.properties().0.contains_key(&key) {
            self.properties_mut()
                .0
                .insert(key.clone(), Property::Array(ValueVec::Object(Vec::new())));
        }
        match self.properties_mut().0.get_mut(&key) {
            Some(Property::Array(ValueVec::Object(refs))) => refs,
            _ => panic!("RoomFeatures must be an Object array"),
        }
    }
}

/// EntranceFeature
pub struct UEntranceFeature;

impl TypedProperties for UEntranceFeature {
    const STRUCT_TYPE: &'static str = "EntranceFeature";
}

impl<'a> TypedPropertiesRef<'a, UEntranceFeature> {
    pub fn location(&self) -> FVector {
        self.try_get::<Vector>("Location")
            .map(FVector::from)
            .unwrap_or_default()
    }
    pub fn direction(&self) -> FRotator {
        self.try_get::<Rotator>("Direction")
            .map(FRotator::from)
            .unwrap_or_default()
    }
    pub fn entrance_type(&self) -> &str {
        self.try_get::<String>("EntranceType")
            .map(|s| s.as_str())
            .unwrap_or("ECaveEntranceType::EntranceAndExit")
    }
    pub fn priority(&self) -> &str {
        self.try_get::<String>("Priority")
            .map(|s| s.as_str())
            .unwrap_or("ECaveEntrancePriority::Primary")
    }
    pub fn room_features(&self) -> Option<&Vec<ObjectRef>> {
        self.try_get::<ValueVec<AssetArchiveType>>("RoomFeatures")
            .and_then(|v| match v {
                ValueVec::Object(refs) => Some(refs),
                _ => None,
            })
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
    pub fn room_features_objects(&mut self) -> &mut Vec<ObjectRef> {
        let key = uesave::PropertyKey::from("RoomFeatures");
        if !self.properties().0.contains_key(&key) {
            self.properties_mut()
                .0
                .insert(key.clone(), Property::Array(ValueVec::Object(Vec::new())));
        }
        match self.properties_mut().0.get_mut(&key) {
            Some(Property::Array(ValueVec::Object(refs))) => refs,
            _ => panic!("RoomFeatures must be an Object array"),
        }
    }
}

/// SpawnActorFeature
pub struct USpawnActorFeature;

impl TypedProperties for USpawnActorFeature {
    const STRUCT_TYPE: &'static str = "SpawnActorFeature";
}

impl<'a> TypedPropertiesRef<'a, USpawnActorFeature> {
    pub fn location(&self) -> FVector {
        self.try_get::<Vector>("Location")
            .map(FVector::from)
            .unwrap_or_default()
    }
    pub fn actor_to_spawn(&self) -> Option<&ObjectRef> {
        self.try_get("ActorToSpawn")
    }
    pub fn adjustment_direction(&self) -> FVector {
        self.try_get::<Vector>("AdjustmentDirection")
            .map(FVector::from)
            .unwrap_or_default()
    }
    pub fn adjustment(&self) -> &str {
        self.try_get::<String>("Adjustment")
            .map(|s| s.as_str())
            .unwrap_or("EItemAdjustmentType::None")
    }
    pub fn scale_min(&self) -> FVector {
        self.try_get::<Vector>("ScaleMin")
            .map(FVector::from)
            .unwrap_or(FVector {
                x: 1.0.into(),
                y: 1.0.into(),
                z: 1.0.into(),
            })
    }
    pub fn scale_max(&self) -> FVector {
        self.try_get::<Vector>("ScaleMax")
            .map(FVector::from)
            .unwrap_or(FVector {
                x: 1.0.into(),
                y: 1.0.into(),
                z: 1.0.into(),
            })
    }
    pub fn rotation_delta(&self) -> FRotator {
        self.try_get::<Rotator>("RotationDelta")
            .map(FRotator::from)
            .unwrap_or_default()
    }
    pub fn room_features(&self) -> Option<&Vec<ObjectRef>> {
        self.try_get::<ValueVec<AssetArchiveType>>("RoomFeatures")
            .and_then(|v| match v {
                ValueVec::Object(refs) => Some(refs),
                _ => None,
            })
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
    pub fn room_features_objects(&mut self) -> &mut Vec<ObjectRef> {
        let key = uesave::PropertyKey::from("RoomFeatures");
        if !self.properties().0.contains_key(&key) {
            self.properties_mut()
                .0
                .insert(key.clone(), Property::Array(ValueVec::Object(Vec::new())));
        }
        match self.properties_mut().0.get_mut(&key) {
            Some(Property::Array(ValueVec::Object(refs))) => refs,
            _ => panic!("RoomFeatures must be an Object array"),
        }
    }
}

/// DropPodCalldownLocationFeature
pub struct UDropPodCalldownLocationFeature;

impl TypedProperties for UDropPodCalldownLocationFeature {
    const STRUCT_TYPE: &'static str = "DropPodCalldownLocationFeature";
}

impl<'a> TypedPropertiesRef<'a, UDropPodCalldownLocationFeature> {
    pub fn location(&self) -> FVector {
        self.try_get::<Vector>("Location")
            .map(FVector::from)
            .unwrap_or_default()
    }
    pub fn calldown_class(&self) -> Option<&ObjectRef> {
        self.try_get("CalldownClass")
    }
    pub fn room_features(&self) -> Option<&Vec<ObjectRef>> {
        self.try_get::<ValueVec<AssetArchiveType>>("RoomFeatures")
            .and_then(|v| match v {
                ValueVec::Object(refs) => Some(refs),
                _ => None,
            })
    }
}

impl<'a> TypedPropertiesMut<'a, UDropPodCalldownLocationFeature> {
    pub fn location(&mut self) -> &mut Vector {
        self.get_mut("Location")
    }
    pub fn calldown_class(&mut self) -> &mut ObjectRef {
        self.get_mut("CalldownClass")
    }
    pub fn room_features_objects(&mut self) -> &mut Vec<ObjectRef> {
        let key = uesave::PropertyKey::from("RoomFeatures");
        if !self.properties().0.contains_key(&key) {
            self.properties_mut()
                .0
                .insert(key.clone(), Property::Array(ValueVec::Object(Vec::new())));
        }
        match self.properties_mut().0.get_mut(&key) {
            Some(Property::Array(ValueVec::Object(refs))) => refs,
            _ => panic!("RoomFeatures must be an Object array"),
        }
    }
}

/// ResourceFeature
pub struct UResourceFeature;

impl TypedProperties for UResourceFeature {
    const STRUCT_TYPE: &'static str = "ResourceFeature";
}

impl<'a> TypedPropertiesRef<'a, UResourceFeature> {
    pub fn location(&self) -> FVector {
        self.try_get::<Vector>("Location")
            .map(FVector::from)
            .unwrap_or_default()
    }
    pub fn base_amount(&self) -> f32 {
        self.try_get::<Float>("BaseAmount")
            .map(|f| f.0)
            .unwrap_or(0.0)
    }
    pub fn room_features(&self) -> Option<&Vec<ObjectRef>> {
        self.try_get::<ValueVec<AssetArchiveType>>("RoomFeatures")
            .and_then(|v| match v {
                ValueVec::Object(refs) => Some(refs),
                _ => None,
            })
    }
}

impl<'a> TypedPropertiesMut<'a, UResourceFeature> {
    pub fn location(&mut self) -> &mut Vector {
        self.get_mut("Location")
    }
    pub fn base_amount(&mut self) -> &mut f32 {
        &mut self.get_mut::<Float>("BaseAmount").0
    }
    pub fn room_features_objects(&mut self) -> &mut Vec<ObjectRef> {
        let key = uesave::PropertyKey::from("RoomFeatures");
        if !self.properties().0.contains_key(&key) {
            self.properties_mut()
                .0
                .insert(key.clone(), Property::Array(ValueVec::Object(Vec::new())));
        }
        match self.properties_mut().0.get_mut(&key) {
            Some(Property::Array(ValueVec::Object(refs))) => refs,
            _ => panic!("RoomFeatures must be an Object array"),
        }
    }
}

/// RandomSelector feature
pub struct URandomSelector;

impl TypedProperties for URandomSelector {
    const STRUCT_TYPE: &'static str = "RandomSelector";
}

impl<'a> TypedPropertiesRef<'a, URandomSelector> {
    pub fn min(&self) -> i32 {
        self.try_get::<i32>("Min").copied().unwrap_or(0)
    }
    pub fn max(&self) -> i32 {
        self.try_get::<i32>("Max").copied().unwrap_or(0)
    }
    pub fn room_features(&self) -> Option<&Vec<ObjectRef>> {
        self.try_get::<ValueVec<AssetArchiveType>>("RoomFeatures")
            .and_then(|v| match v {
                ValueVec::Object(refs) => Some(refs),
                _ => None,
            })
    }
}

impl<'a> TypedPropertiesMut<'a, URandomSelector> {
    pub fn min(&mut self) -> &mut i32 {
        self.get_mut("Min")
    }
    pub fn max(&mut self) -> &mut i32 {
        self.get_mut("Max")
    }
    pub fn room_features_objects(&mut self) -> &mut Vec<ObjectRef> {
        let key = uesave::PropertyKey::from("RoomFeatures");
        if !self.properties().0.contains_key(&key) {
            self.properties_mut()
                .0
                .insert(key.clone(), Property::Array(ValueVec::Object(Vec::new())));
        }
        match self.properties_mut().0.get_mut(&key) {
            Some(Property::Array(ValueVec::Object(refs))) => refs,
            _ => panic!("RoomFeatures must be an Object array"),
        }
    }
}

/// SpawnTriggerFeature
pub struct USpawnTriggerFeature;

impl TypedProperties for USpawnTriggerFeature {
    const STRUCT_TYPE: &'static str = "SpawnTriggerFeature";
}

impl<'a> TypedPropertiesRef<'a, USpawnTriggerFeature> {
    pub fn room_features(&self) -> Option<&Vec<ObjectRef>> {
        self.try_get::<ValueVec<AssetArchiveType>>("RoomFeatures")
            .and_then(|v| match v {
                ValueVec::Object(refs) => Some(refs),
                _ => None,
            })
    }
}

impl<'a> TypedPropertiesMut<'a, USpawnTriggerFeature> {
    pub fn room_features_objects(&mut self) -> &mut Vec<ObjectRef> {
        let key = uesave::PropertyKey::from("RoomFeatures");
        if !self.properties().0.contains_key(&key) {
            self.properties_mut()
                .0
                .insert(key.clone(), Property::Array(ValueVec::Object(Vec::new())));
        }
        match self.properties_mut().0.get_mut(&key) {
            Some(Property::Array(ValueVec::Object(refs))) => refs,
            _ => panic!("RoomFeatures must be an Object array"),
        }
    }
}

/// RoomGenerator root object
pub struct URoomGenerator;

impl TypedProperties for URoomGenerator {
    const STRUCT_TYPE: &'static str = "RoomGenerator";
}

impl<'a> TypedPropertiesRef<'a, URoomGenerator> {
    pub fn bounds(&self) -> f32 {
        self.try_get::<Float>("Bounds").map(|f| f.0).unwrap_or(0.0)
    }
    pub fn can_only_be_used_once(&self) -> bool {
        self.try_get::<bool>("CanOnlyBeUsedOnce")
            .copied()
            .unwrap_or(false)
    }
    pub fn mirror_support(&self) -> &str {
        self.try_get::<String>("MirrorSupport")
            .map(|s| s.as_str())
            .unwrap_or("ERoomMirroringSupport::NotAllowed")
    }
    pub fn room_tags(&self) -> Option<&GameplayTagContainer> {
        self.try_get("RoomTags")
    }
    pub fn room_features(&self) -> Option<&Vec<ObjectRef>> {
        self.try_get::<ValueVec<AssetArchiveType>>("RoomFeatures")
            .and_then(|v| match v {
                ValueVec::Object(refs) => Some(refs),
                _ => None,
            })
    }
}

impl<'a> TypedPropertiesMut<'a, URoomGenerator> {
    pub fn bounds(&mut self) -> &mut f32 {
        &mut self.get_mut::<Float>("Bounds").0
    }
    pub fn can_only_be_used_once(&mut self) -> &mut bool {
        self.get_mut("CanOnlyBeUsedOnce")
    }
    pub fn mirror_support(&mut self) -> &mut String {
        self.get_mut("MirrorSupport")
    }
    pub fn room_tags(&mut self) -> &mut GameplayTagContainer {
        self.get_mut("RoomTags")
    }
    pub fn room_features_objects(&mut self) -> &mut Vec<ObjectRef> {
        let key = uesave::PropertyKey::from("RoomFeatures");
        if !self.properties().0.contains_key(&key) {
            self.properties_mut()
                .0
                .insert(key.clone(), Property::Array(ValueVec::Object(Vec::new())));
        }
        match self.properties_mut().0.get_mut(&key) {
            Some(Property::Array(ValueVec::Object(refs))) => refs,
            _ => panic!("RoomFeatures must be an Object array"),
        }
    }
}

// ============================================================================
// RoomFeature Enum - for iterating heterogeneous features
// ============================================================================

/// Enum wrapper for different room feature types
#[derive(Debug, Clone)]
pub enum RoomFeature {
    FloodFillBox(ObjectHandle),
    FloodFillPillar(ObjectHandle),
    FloodFillLine(ObjectHandle),
    EntranceFeature(ObjectHandle),
    SpawnActorFeature(ObjectHandle),
    DropPodCalldownLocationFeature(ObjectHandle),
    ResourceFeature(ObjectHandle),
    RandomSelector(ObjectHandle),
    SpawnTriggerFeature(ObjectHandle),
    Unknown(ObjectHandle, String),
}

impl RoomFeature {
    /// Create a RoomFeature from an ObjectHandle by inspecting its class
    pub fn from_handle(pool: &ObjectPool, handle: ObjectHandle) -> Self {
        let obj = pool.get(handle).expect("Invalid handle");
        let class_path = match &obj.class {
            ObjectRef::Loaded(h) => pool.build_path(*h).to_string(),
            ObjectRef::Unloaded(p) => p.to_string(),
        };

        // Extract class name from path like "/Script/FSD.FloodFillBox"
        let class_name = class_path.rsplit('.').next().unwrap_or(&class_path);

        match class_name {
            "FloodFillBox" => RoomFeature::FloodFillBox(handle),
            "FloodFillPillar" => RoomFeature::FloodFillPillar(handle),
            "FloodFillLine" => RoomFeature::FloodFillLine(handle),
            "EntranceFeature" => RoomFeature::EntranceFeature(handle),
            "SpawnActorFeature" => RoomFeature::SpawnActorFeature(handle),
            "DropPodCalldownLocationFeature" => RoomFeature::DropPodCalldownLocationFeature(handle),
            "ResourceFeature" => RoomFeature::ResourceFeature(handle),
            "RandomSelector" => RoomFeature::RandomSelector(handle),
            "SpawnTriggerFeature" => RoomFeature::SpawnTriggerFeature(handle),
            _ => RoomFeature::Unknown(handle, class_name.to_string()),
        }
    }

    /// Get the handle for this feature
    pub fn handle(&self) -> ObjectHandle {
        match self {
            RoomFeature::FloodFillBox(h) => *h,
            RoomFeature::FloodFillPillar(h) => *h,
            RoomFeature::FloodFillLine(h) => *h,
            RoomFeature::EntranceFeature(h) => *h,
            RoomFeature::SpawnActorFeature(h) => *h,
            RoomFeature::DropPodCalldownLocationFeature(h) => *h,
            RoomFeature::ResourceFeature(h) => *h,
            RoomFeature::RandomSelector(h) => *h,
            RoomFeature::SpawnTriggerFeature(h) => *h,
            RoomFeature::Unknown(h, _) => *h,
        }
    }

    /// Get the feature name
    pub fn name(&self) -> &str {
        match self {
            RoomFeature::FloodFillBox(_) => "FloodFillBox",
            RoomFeature::FloodFillPillar(_) => "FloodFillPillar",
            RoomFeature::FloodFillLine(_) => "FloodFillLine",
            RoomFeature::EntranceFeature(_) => "EntranceFeature",
            RoomFeature::SpawnActorFeature(_) => "SpawnActorFeature",
            RoomFeature::DropPodCalldownLocationFeature(_) => "DropPodCalldownLocationFeature",
            RoomFeature::ResourceFeature(_) => "ResourceFeature",
            RoomFeature::RandomSelector(_) => "RandomSelector",
            RoomFeature::SpawnTriggerFeature(_) => "SpawnTriggerFeature",
            RoomFeature::Unknown(_, name) => name,
        }
    }

    /// Get child room features from this feature
    pub fn get_child_features(&self, pool: &ObjectPool) -> Vec<RoomFeature> {
        let obj = pool.get(self.handle()).expect("Invalid handle");
        let props = obj.properties();

        let key = uesave::PropertyKey::from("RoomFeatures");
        match props.0.get(&key) {
            Some(Property::Array(ValueVec::Object(refs))) => refs
                .iter()
                .filter_map(|r| match r {
                    ObjectRef::Loaded(h) => Some(RoomFeature::from_handle(pool, *h)),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }
}

// ============================================================================
// Helper functions
// ============================================================================

/// Load all room features from a RoomGenerator root object
pub fn load_room_features(pool: &ObjectPool, root_handle: ObjectHandle) -> Vec<RoomFeature> {
    let obj = pool.get(root_handle).expect("Invalid root handle");
    let props = obj.properties();

    let key = uesave::PropertyKey::from("RoomFeatures");
    match props.0.get(&key) {
        Some(Property::Array(ValueVec::Object(refs))) => refs
            .iter()
            .filter_map(|r| match r {
                ObjectRef::Loaded(h) => Some(RoomFeature::from_handle(pool, *h)),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}
