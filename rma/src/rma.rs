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
struct RoomFeatureBase {
    room_features: Vec<RoomFeature>,
}

#[derive(Debug, Serialize)]
enum RoomFeature {
    FloodFillBox,
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

#[derive(Debug, Default, Serialize, FromProperty, FromProperties)]
struct FRandRange {
    min: f32,
    max: f32,
}

/*
impl<C: Read + Seek> FromProperty<C> for FRandRange {
    fn from_property(asset: &Asset<C>, property: &Property) -> Result<Self> {
        let mut read_properties = HashSet::new();
        let res = match property {
            Property::StructProperty(property) => {
                FRandRange::from_properties(asset, &property.value, &mut read_properties)?
            }
            _ => bail!("sdafdsaf")
        };
        assert_eq!(read_properties, ["asdf".into()].into());
        Ok(res)
    }
}
*/

#[derive(Debug, Default, Serialize, FromProperty, FromProperties)]
struct FRandLinePoint {
    location: FVector,
    range: FRandRange,
    noise_range: FRandRange,
    skew_factor: FRandRange,
    fill_amount: FRandRange,
}

#[derive(Debug, Default, Serialize, FromExport, FromProperties)]
struct FloodFillPillar {
    #[serde(flatten)]
    base: RoomFeatureBase,
    noise_override: Option<UFloodFillSettings>,
    points: Vec<FRandLinePoint>,
    range_scale: FRandRange,
    noise_range_scale: FRandRange,
    endcap_scale: FRandRange,
}

#[derive(Debug, Default, Serialize, FromExport, FromProperties)]
struct RandomSelector {
    #[serde(flatten)]
    base: RoomFeatureBase,
    min: i32,
    max: i32,
}

#[derive(Debug, Default, Serialize)]
struct FVector {
    x: f32,
    y: f32,
    z: f32,
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

#[derive(Debug, Default, Serialize)]
struct FRotator {
    pitch: f32,
    yaw: f32,
    roll: f32,
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
enum ECaveEntranceType {
    #[default]
    EntranceAndExit,
    Entrance,
    Exit,
    TreassureRoom,
}
impl<C: Read + Seek> FromProperty<C> for ECaveEntranceType {
    fn from_property(_asset: &Asset<C>, property: &Property) -> Result<Self> {
        todo!("{:#?}", property);
    }
}

#[derive(Debug, Default, Serialize)]
enum ECaveEntrancePriority {
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
struct EntranceFeature {
    #[serde(flatten)]
    base: RoomFeatureBase,
    location: FVector,
    direction: FRotator,
    entrance_type: ECaveEntranceType,
    priority: ECaveEntrancePriority,
}

#[derive(Debug, Default, Serialize, FromProperty, FromProperties)]
struct FRoomLinePoint {
    location: FVector,
    h_range: f32,
    v_range: f32,
    cieling_noise_range: f32,
    wall_noise_range: f32,
    floor_noise_range: f32,
    cielingheight: f32,
    height_scale: f32,
    floor_depth: f32,
    floor_angle: f32,
}

#[derive(Debug, Default, Serialize, FromProperty, FromProperties)]
struct FLayeredNoise {
    noise: UFloodFillSettings,
    scale: f32,
}

#[derive(Debug, Default, Serialize, FromExport, FromProperties)]
struct UFloodFillSettings {
    noise_size: FVector,
    freq_multiplier: f32,
    amplitude_multiplier: f32,
    min_value: f32,
    max_value: f32,
    turbulence: bool,
    invert: bool,
    octaves: i32,
    noise_layers: Vec<FLayeredNoise>,
}

#[derive(Debug, Default, Serialize, FromExport, FromProperties)]
struct FloodFillLine {
    #[serde(flatten)]
    base: RoomFeatureBase,
    wall_noise_override: Option<UFloodFillSettings>,
    ceiling_noise_override: Option<UFloodFillSettings>,
    flood_noise_override: Option<UFloodFillSettings>,
    use_detailed_noise: bool,
    points: Vec<FRoomLinePoint>,
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

impl<C: Read + Seek> FromProperty<C> for RoomFeature {
    fn from_property(asset: &Asset<C>, property: &Property) -> Result<Self> {
        from_object_property(asset, property)
    }
}

#[derive(Debug, Default, Serialize)]
enum ERoomMirroringSupport {
    #[default]
    NotAllowed,
    MirrorAroundX,
    MirrorAroundY,
    MirrorBoth,
}
impl<C: Read + Seek> FromProperty<C> for ERoomMirroringSupport {
    fn from_property(_asset: &Asset<C>, property: &Property) -> Result<Self> {
        todo!("{property:?}");
        /*
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
        */
    }
}

#[derive(Debug, Default, Serialize)]
struct FGameplayTagContainer {
    tags: Vec<String>,
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
struct RoomGeneratorBase {
    bounds: f32,
    can_only_be_used_once: bool,
    mirror_support: ERoomMirroringSupport,
    room_tags: FGameplayTagContainer,
}

#[derive(Debug, Default, Serialize, FromExport, FromProperties)]
pub struct RoomGenerator {
    #[serde(flatten)]
    base: RoomGeneratorBase,
    room_features: Vec<RoomFeature>,
}
