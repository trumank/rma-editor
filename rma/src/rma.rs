use rma_lib::{from_object_property, FromExport, FromProperties, FromProperty};

use anyhow::{bail, Result};
use serde::Serialize;
use unreal_asset::exports::{ExportBaseTrait, ExportNormalTrait};
use unreal_asset::properties::Property;
use unreal_asset::reader::ArchiveTrait;
use unreal_asset::types::PackageIndex;
use unreal_asset::Asset;

use std::io::{Read, Seek};

#[derive(Debug, Default, Serialize, FromExport, FromProperties)]
pub struct RoomFeatureBase {
    pub room_features: Vec<RoomFeature>,
}

#[derive(Debug, Serialize)]
pub enum RoomFeature {
    FloodFillBox(FloodFillBox),
    FloodFillProceduralPillar,
    SpawnTriggerFeature,
    FloodFillPillar(FloodFillPillar),
    RandomSelector(RandomSelector),
    EntranceFeature(EntranceFeature),
    RandomSubRoomFeature,
    SpawnActorFeature,
    FloodFillLine(FloodFillLine),
    ResourceFeature,
    SubRoomFeature,
    DropPodCalldownLocationFeature,
}

impl RoomFeature {
    pub fn name(&self) -> &'static str {
        match self {
            RoomFeature::FloodFillBox(_) => "FloodFillBox",
            RoomFeature::FloodFillProceduralPillar => "FloodFillProceduralPillar",
            RoomFeature::SpawnTriggerFeature => "SpawnTriggerFeature ",
            RoomFeature::FloodFillPillar(_) => "FloodFillPillar",
            RoomFeature::RandomSelector(_) => "RandomSelector",
            RoomFeature::EntranceFeature(_) => "EntranceFeature",
            RoomFeature::RandomSubRoomFeature => "RandomSubRoomFeature",
            RoomFeature::SpawnActorFeature => "SpawnActorFeature ",
            RoomFeature::FloodFillLine(_) => "FloodFillLine",
            RoomFeature::ResourceFeature => "ResourceFeature ",
            RoomFeature::SubRoomFeature => "SubRoomFeature ",
            RoomFeature::DropPodCalldownLocationFeature => "DropPodCalldownLocationFeature",
        }
    }
    pub fn base(&self) -> &RoomFeatureBase {
        match self {
            RoomFeature::FloodFillBox(f) => &f.base,
            RoomFeature::FloodFillProceduralPillar => todo!(),
            RoomFeature::SpawnTriggerFeature => todo!(),
            RoomFeature::FloodFillPillar(f) => &f.base,
            RoomFeature::RandomSelector(f) => &f.base,
            RoomFeature::EntranceFeature(f) => &f.base,
            RoomFeature::RandomSubRoomFeature => todo!(),
            RoomFeature::SpawnActorFeature => todo!(),
            RoomFeature::FloodFillLine(f) => &f.base,
            RoomFeature::ResourceFeature => todo!(),
            RoomFeature::SubRoomFeature => todo!(),
            RoomFeature::DropPodCalldownLocationFeature => todo!(),
        }
    }
}

impl<C: Seek + Read> FromExport<C> for RoomFeature {
    fn from_export(asset: &Asset<C>, package_index: PackageIndex) -> Result<Self> {
        let export = asset.get_export(package_index).unwrap();
        let name = asset
            .get_import(export.get_base_export().class_index)
            .unwrap()
            .object_name
            .get_owned_content();

        let res = match name.as_str() {
            "FloodFillBox" => {
                RoomFeature::FloodFillBox(FromExport::from_export(asset, package_index)?)
            }
            "FloodFillPillar" => {
                RoomFeature::FloodFillPillar(FromExport::from_export(asset, package_index)?)
            }
            "RandomSelector" => {
                RoomFeature::RandomSelector(FromExport::from_export(asset, package_index)?)
            }
            "EntranceFeature" => {
                RoomFeature::EntranceFeature(FromExport::from_export(asset, package_index)?)
            }
            "FloodFillLine" => {
                RoomFeature::FloodFillLine(FromExport::from_export(asset, package_index)?)
            }
            _ => unimplemented!("{}", name),
        };
        Ok(res)
    }
}

#[derive(Debug, Default, Serialize, FromExport, FromProperties)]
pub struct FloodFillBox {
    base: RoomFeatureBase,
    noise: (), // TODO import Option<UFloodFillSettings>,
    position: FVector,
    extends: FVector,
    rotation: FRotator,
    is_carver: bool,
    noise_range: f32,
}

#[derive(Debug, Default, Serialize, FromProperty, FromProperties)]
pub struct FRandRange {
    pub min: f32,
    pub max: f32,
}

#[derive(Debug, Default, Serialize, FromProperty, FromProperties)]
pub struct FRandLinePoint {
    pub location: FVector,
    pub range: FRandRange,
    pub noise_range: FRandRange,
    pub skew_factor: FRandRange,
    pub fill_amount: FRandRange,
}

#[derive(Debug, Default, Serialize, FromExport, FromProperties)]
pub struct FloodFillPillar {
    #[serde(flatten)]
    pub base: RoomFeatureBase,
    pub noise_override: Option<UFloodFillSettings>,
    pub points: Vec<FRandLinePoint>,
    pub range_scale: FRandRange,
    pub noise_range_scale: FRandRange,
    pub endcap_scale: FRandRange,
}

#[derive(Debug, Default, Serialize, FromExport, FromProperties)]
pub struct RandomSelector {
    #[serde(flatten)]
    pub base: RoomFeatureBase,
    pub min: i32,
    pub max: i32,
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct FVector {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl<C: Read + Seek> FromProperty<C> for FVector {
    fn from_property(_asset: &Asset<C>, property: &Property) -> Result<Self> {
        match property {
            Property::StructProperty(property) => match &property.value[0] {
                Property::VectorProperty(property) => Ok(Self {
                    x: property.value.x.0 as f32,
                    y: property.value.y.0 as f32,
                    z: property.value.z.0 as f32,
                }),
                _ => bail!("{property:?}"),
            },
            _ => bail!("{property:?}"),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct FRotator {
    pub pitch: f32,
    pub yaw: f32,
    pub roll: f32,
}

impl<C: Read + Seek> FromProperty<C> for FRotator {
    fn from_property(_asset: &Asset<C>, property: &Property) -> Result<Self> {
        match property {
            Property::StructProperty(property) => match &property.value[0] {
                Property::RotatorProperty(property) => Ok(Self {
                    pitch: property.value.x.0 as f32,
                    yaw: property.value.y.0 as f32,
                    roll: property.value.z.0 as f32,
                }),
                _ => bail!("{property:?}"),
            },
            _ => bail!("{property:?}"),
        }
    }
}

#[derive(Debug, Default, Serialize)]
pub enum ECaveEntranceType {
    #[default]
    EntranceAndExit,
    Entrance,
    Exit,
    TreassureRoom,
}
impl<C: Read + Seek> FromProperty<C> for ECaveEntranceType {
    fn from_property(_asset: &Asset<C>, property: &Property) -> Result<Self> {
        match property {
            Property::EnumProperty(property) => property.value.as_ref().unwrap().get_content(|c| {
                Ok(match c {
                    "ECaveEntranceType::EntranceAndExit" => ECaveEntranceType::EntranceAndExit,
                    "ECaveEntranceType::Entrance" => ECaveEntranceType::Entrance,
                    "ECaveEntranceType::Exit" => ECaveEntranceType::Exit,
                    "ECaveEntranceType::TreassureRoom" => ECaveEntranceType::TreassureRoom,
                    _ => bail!("unknown variant {}", c),
                })
            }),
            _ => bail!("{property:?}"),
        }
    }
}

#[derive(Debug, Default, Serialize)]
pub enum ECaveEntrancePriority {
    #[default]
    Primary,
    Secondary,
}

impl<C: Read + Seek> FromProperty<C> for ECaveEntrancePriority {
    fn from_property(_asset: &Asset<C>, property: &Property) -> Result<Self> {
        match property {
            Property::EnumProperty(property) => property.value.as_ref().unwrap().get_content(|c| {
                Ok(match c {
                    "ECaveEntrancePriority::Primary" => ECaveEntrancePriority::Primary,
                    "ECaveEntrancePriority::Secondary" => ECaveEntrancePriority::Secondary,
                    _ => bail!("unknown variant {}", c),
                })
            }),
            _ => bail!("{property:?}"),
        }
    }
}

#[derive(Debug, Default, Serialize, FromExport, FromProperties)]
pub struct EntranceFeature {
    #[serde(flatten)]
    pub base: RoomFeatureBase,
    pub location: FVector,
    pub direction: FRotator,
    pub entrance_type: ECaveEntranceType,
    pub priority: ECaveEntrancePriority,
}

#[derive(Debug, Default, Serialize, FromProperty, FromProperties)]
pub struct FRoomLinePoint {
    pub location: FVector,
    pub h_range: f32,
    pub v_range: f32,
    pub cieling_noise_range: f32,
    pub wall_noise_range: f32,
    pub floor_noise_range: f32,
    pub cielingheight: f32,
    pub height_scale: f32,
    pub floor_depth: f32,
    pub floor_angle: f32,
}

#[derive(Debug, Default, Serialize, FromProperty, FromProperties)]
pub struct FLayeredNoise {
    pub noise: UFloodFillSettings,
    pub scale: f32,
}

#[derive(Debug, Default, Serialize, FromExport, FromProperties)]
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

#[derive(Debug, Default, Serialize, FromExport, FromProperties)]
pub struct FloodFillLine {
    #[serde(flatten)]
    pub base: RoomFeatureBase,
    pub wall_noise_override: Option<UFloodFillSettings>,
    pub ceiling_noise_override: Option<UFloodFillSettings>,
    pub flood_noise_override: Option<UFloodFillSettings>,
    pub use_detailed_noise: bool,
    pub points: Vec<FRoomLinePoint>,
}

impl<C: Read + Seek> FromProperty<C> for RoomFeature {
    fn from_property(asset: &Asset<C>, property: &Property) -> Result<Self> {
        from_object_property(asset, property)
    }
}

#[derive(Debug, Default, Serialize)]
pub enum ERoomMirroringSupport {
    #[default]
    NotAllowed,
    MirrorAroundX,
    MirrorAroundY,
    MirrorBoth,
}
impl<C: Read + Seek> FromProperty<C> for ERoomMirroringSupport {
    fn from_property(_asset: &Asset<C>, property: &Property) -> Result<Self> {
        match property {
            Property::EnumProperty(property) => property.value.as_ref().unwrap().get_content(|c| {
                Ok(match c {
                    "ERoomMirroringSupport::NotAllowed" => ERoomMirroringSupport::NotAllowed,
                    "ERoomMirroringSupport::MirrorAroundX" => ERoomMirroringSupport::MirrorAroundX,
                    "ERoomMirroringSupport::MirrorAroundY" => ERoomMirroringSupport::MirrorAroundY,
                    "ERoomMirroringSupport::MirrorBoth" => ERoomMirroringSupport::MirrorBoth,
                    _ => bail!("unknown variant {}", c),
                })
            }),
            _ => bail!("{property:?}"),
        }
    }
}

#[derive(Debug, Default, Serialize)]
pub struct FGameplayTagContainer {
    pub tags: Vec<String>,
}
impl<C: Read + Seek> FromProperty<C> for FGameplayTagContainer {
    fn from_property(_asset: &Asset<C>, property: &Property) -> Result<Self> {
        match property {
            Property::StructProperty(property) => match &property.value[0] {
                Property::GameplayTagContainerProperty(property) => Ok(Self {
                    tags: property
                        .value
                        .iter()
                        .map(|n| n.get_owned_content())
                        .collect(),
                }),
                _ => bail!("{property:?}"),
            },
            _ => bail!("{property:?}"),
        }
    }
}

#[derive(Debug, Default, Serialize, FromExport, FromProperties)]
pub struct RoomGeneratorBase {
    pub bounds: f32,
    pub can_only_be_used_once: bool,
    pub mirror_support: ERoomMirroringSupport,
    pub room_tags: FGameplayTagContainer,
}

#[derive(Debug, Default, Serialize, FromExport, FromProperties)]
pub struct RoomGenerator {
    #[serde(flatten)]
    pub base: RoomGeneratorBase,
    pub room_features: Vec<RoomFeature>,
}
