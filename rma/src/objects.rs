use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

pub type UClass = String;

#[derive(Debug, Clone, Default)]
pub struct FVector {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Serialize for FVector {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        (self.x, self.y, self.z).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for FVector {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (x, y, z) = <(f32, f32, f32)>::deserialize(deserializer)?;
        Ok(FVector { x, y, z })
    }
}

#[derive(Debug, Clone, Default)]
pub struct FRotator {
    pub pitch: f32,
    pub yaw: f32,
    pub roll: f32,
}

impl Serialize for FRotator {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        (self.pitch, self.yaw, self.roll).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for FRotator {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (pitch, yaw, roll) = <(f32, f32, f32)>::deserialize(deserializer)?;
        Ok(FRotator { pitch, yaw, roll })
    }
}

/// Quaternion rotation
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FQuat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Default for FQuat {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0, // Identity quaternion
        }
    }
}

impl FQuat {
    /// Create a quaternion from euler angles (in degrees)
    pub fn from_euler(pitch: f32, yaw: f32, roll: f32) -> Self {
        // Convert degrees to radians
        let pitch = pitch.to_radians() * 0.5;
        let yaw = yaw.to_radians() * 0.5;
        let roll = roll.to_radians() * 0.5;

        let (sp, cp) = pitch.sin_cos();
        let (sy, cy) = yaw.sin_cos();
        let (sr, cr) = roll.sin_cos();

        Self {
            x: cr * sp * cy + sr * cp * sy,
            y: cr * cp * sy - sr * sp * cy,
            z: sr * cp * cy - cr * sp * sy,
            w: cr * cp * cy + sr * sp * sy,
        }
    }

    /// Convert quaternion to euler angles (pitch, yaw, roll) in degrees
    pub fn to_euler(&self) -> (f32, f32, f32) {
        let sinr_cosp = 2.0 * (self.w * self.x + self.y * self.z);
        let cosr_cosp = 1.0 - 2.0 * (self.x * self.x + self.y * self.y);
        let roll = sinr_cosp.atan2(cosr_cosp);

        let sinp = 2.0 * (self.w * self.y - self.z * self.x);
        let pitch = if sinp.abs() >= 1.0 {
            std::f32::consts::FRAC_PI_2.copysign(sinp)
        } else {
            sinp.asin()
        };

        let siny_cosp = 2.0 * (self.w * self.z + self.x * self.y);
        let cosy_cosp = 1.0 - 2.0 * (self.y * self.y + self.z * self.z);
        let yaw = siny_cosp.atan2(cosy_cosp);

        (pitch.to_degrees(), yaw.to_degrees(), roll.to_degrees())
    }
}

impl From<FRotator> for FQuat {
    fn from(r: FRotator) -> Self {
        Self::from_euler(r.pitch, r.yaw, r.roll)
    }
}

impl From<FQuat> for FRotator {
    fn from(q: FQuat) -> Self {
        let (pitch, yaw, roll) = q.to_euler();
        FRotator { pitch, yaw, roll }
    }
}

impl Serialize for FQuat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        (self.x, self.y, self.z, self.w).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for FQuat {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (x, y, z, w) = <(f32, f32, f32, f32)>::deserialize(deserializer)?;
        Ok(FQuat { x, y, z, w })
    }
}

/// Transform (translation + rotation + scale)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[allow(non_snake_case)]
pub struct FTransform {
    pub translation: FVector,
    pub rotation: FQuat,
    pub Scale3D: FVector,
}

impl Default for FTransform {
    fn default() -> Self {
        Self {
            translation: FVector::default(),
            rotation: FQuat::default(),
            Scale3D: FVector {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct FGameplayTagQuery {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct FGameplayTagContainer(pub BTreeSet<String>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ECaveEntranceType {
    EntranceAndExit = 0,
    Entrance = 1,
    Exit = 2,
    TreassureRoom = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ECaveEntrancePriority {
    Primary = 0,
    Secondary = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum EItemAdjustmentType {
    None = 0,
    Cieling = 1,
    Wall = 2,
    Floor = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ERoomMirroringSupport {
    NotAllowed = 0,
    MirrorAroundX = 1,
    MirrorAroundY = 2,
    MirrorBoth = 3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct FRandRange {
    pub min: f32,
    pub max: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct FRandLinePoint {
    pub location: FVector,
    pub range: FRandRange,
    pub noise_range: FRandRange,
    pub skew_factor: FRandRange,
    pub fill_amount: FRandRange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct FRoomLinePoint {
    pub location: FVector,
    pub h_range: f32,
    pub v_range: f32,
    pub cieling_noise_range: f32,
    pub wall_noise_range: f32,
    pub floor_noise_range: f32,
    pub cieling_height: f32,
    pub height_scale: f32,
    pub floor_depth: f32,
    pub floor_angle: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct FLayeredNoise {
    pub noise: Option<Box<UFloodFillSettings>>,
    pub scale: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct FPillarSegment {
    pub scale: f32,
    pub height_offset: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UFloodFillSettings {
    pub noise_size: FVector,
    pub freq_multiplier: f32,
    pub amplitude_multiplier: f32,
    pub min_value: f32,
    pub max_value: f32,
    pub turbulence: bool,
    pub invert: bool,
    pub octaves: i32,
    pub noise_layers: Vec<FLayeredNoise>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UPillarSettings {
    pub pillar_segments: Vec<FPillarSegment>,
    pub point_range: FRandRange,
    pub point_noise_range: FRandRange,
    pub pillar_range_scale: FRandRange,
    pub pillar_noise_range_scale: FRandRange,
    pub noise: Option<Box<UFloodFillSettings>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UResourceData {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct URoomGeneratorBase {
    pub bounds: f32,
    pub can_only_be_used_once: bool,
    pub mirror_support: ERoomMirroringSupport,
    pub room_tags: FGameplayTagContainer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct URoomGenerator {
    pub base: URoomGeneratorBase,
    pub room_features: Vec<URoomFeature>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct URoomGeneratorGroup {
    pub rooms: Vec<URoomGenerator>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct FRoomGeneratorGroupInstance {
    pub rooms: Vec<URoomGenerator>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct URoomFeature {
    pub children: Vec<URoomFeature>,
    #[serde(flatten)]
    pub feature_type: URoomFeatureType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", tag = "Type")]
pub enum URoomFeatureType {
    DropPodCalldownLocation(UDropPodCalldownLocationFeature),
    Entrance(UEntranceFeature),
    FloodFillBox(UFloodFillBox),
    FloodFillLine(UFloodFillLine),
    FloodFillPillar(UFloodFillPillar),
    FloodFillProceduralPillar(UFloodFillProceduralPillar),
    RandomSelector(URandomSelector),
    RandomSubRoom(URandomSubRoomFeature),
    Resource(UResourceFeature),
    SpawnActor(USpawnActorFeature),
    SpawnTrigger(USpawnTriggerFeature),
    SubRoom(USubRoomFeature),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UDropPodCalldownLocationFeature {
    pub location: FVector,
    pub calldown_class: Option<UClass>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UEntranceFeature {
    pub location: FVector,
    pub direction: FRotator,
    pub entrance_type: ECaveEntranceType,
    pub priority: ECaveEntrancePriority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UFloodFillBox {
    pub noise: Option<Box<UFloodFillSettings>>,
    pub position: FVector,
    pub extends: FVector,
    pub rotation: FRotator,
    pub is_carver: bool,
    pub noise_range: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UFloodFillLine {
    pub wall_noise_override: Option<Box<UFloodFillSettings>>,
    pub ceiling_noise_override: Option<Box<UFloodFillSettings>>,
    pub floor_noise_override: Option<Box<UFloodFillSettings>>,
    pub use_detail_noise: bool,
    pub points: Vec<FRoomLinePoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UFloodFillPillar {
    pub noise_override: Option<Box<UFloodFillSettings>>,
    pub points: Vec<FRandLinePoint>,
    pub range_scale: FRandRange,
    pub noise_range_scale: FRandRange,
    pub endcap_scale: FRandRange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UFloodFillProceduralPillar {
    pub points: Vec<FVector>,
    pub pillar_settings: Option<Box<UPillarSettings>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct URandomSelector {
    pub min: i32,
    pub max: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct URandomSubRoomFeature {
    pub room_group: Option<Box<URoomGeneratorGroup>>,
    pub tag_query: FGameplayTagQuery,
    pub layer: i32,
    pub location: FVector,
    pub rotation: FRotator,
    pub scale: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UResourceFeature {
    pub location: FVector,
    pub resource: Option<Box<UResourceData>>,
    pub base_amount: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct USpawnActorFeature {
    pub location: FVector,
    pub actor_to_spawn: Option<UClass>,
    pub adjustment_direction: FVector,
    pub adjustment: EItemAdjustmentType,
    pub scale_min: FVector,
    pub scale_max: FVector,
    pub rotation_delta: FRotator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct USpawnTriggerFeature {
    pub trigger_class: Option<UClass>,
    pub transform: FTransform,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct USubRoomFeature {
    pub room_generator: Option<Box<URoomGenerator>>,
    pub location: FVector,
    pub rotation: FRotator,
    pub scale: f32,
}
