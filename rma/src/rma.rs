use ordered_float::OrderedFloat;
use rma_lib::{
    from_object_property, get_import, resolve_package_index, BaseExportGetter, CtxSer, FromExport,
    FromProperties, FromProperty, ImportChain, ToExport, ToProperties, ToProperty,
};

use anyhow::{bail, Result};
use serde::Serialize;
use unreal_asset::exports::{BaseExport, ExportBaseTrait, ExportNormalTrait};
use unreal_asset::flags::EObjectFlags;
use unreal_asset::properties::enum_property::EnumProperty;
use unreal_asset::properties::gameplay_tag_container_property::GameplayTagContainerProperty;
use unreal_asset::properties::object_property::ObjectProperty;
use unreal_asset::properties::str_property::NameProperty;
use unreal_asset::properties::struct_property::StructProperty;
use unreal_asset::properties::vector_property::{QuatProperty, RotatorProperty, VectorProperty};
use unreal_asset::properties::Property;
use unreal_asset::reader::ArchiveTrait;
use unreal_asset::types::vector::{Vector, Vector4};
use unreal_asset::types::PackageIndex;
use unreal_asset::Asset;

use std::io::{Read, Seek};

const FSD_PACKAGE: ImportChain = ImportChain {
    outer: None,
    class_package: "/Script/CoreUObject",
    class_name: "Package",
    object_name: "/Script/FSD",
};

#[derive(
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Clone,
    Serialize,
    FromExport,
    FromProperties,
    ToProperties,
)]
pub struct RoomFeatureBase {
    pub room_features: Vec<RoomFeature>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Serialize)]
pub enum RoomFeature {
    FloodFillBox(FloodFillBox),
    FloodFillProceduralPillar,
    SpawnTriggerFeature(SpawnTriggerFeature),
    FloodFillPillar(FloodFillPillar),
    RandomSelector(RandomSelector),
    EntranceFeature(EntranceFeature),
    RandomSubRoomFeature,
    SpawnActorFeature(SpawnActorFeature),
    FloodFillLine(FloodFillLine),
    ResourceFeature(ResourceFeature),
    SubRoomFeature,
    DropPodCalldownLocationFeature(DropPodCalldownLocationFeature),
}

impl RoomFeature {
    pub fn name(&self) -> &'static str {
        match self {
            RoomFeature::FloodFillBox(_) => "FloodFillBox",
            RoomFeature::FloodFillProceduralPillar => "FloodFillProceduralPillar",
            RoomFeature::SpawnTriggerFeature(_) => "SpawnTriggerFeature ",
            RoomFeature::FloodFillPillar(_) => "FloodFillPillar",
            RoomFeature::RandomSelector(_) => "RandomSelector",
            RoomFeature::EntranceFeature(_) => "EntranceFeature",
            RoomFeature::RandomSubRoomFeature => "RandomSubRoomFeature",
            RoomFeature::SpawnActorFeature(_) => "SpawnActorFeature",
            RoomFeature::FloodFillLine(_) => "FloodFillLine",
            RoomFeature::ResourceFeature(_) => "ResourceFeature ",
            RoomFeature::SubRoomFeature => "SubRoomFeature ",
            RoomFeature::DropPodCalldownLocationFeature(_) => "DropPodCalldownLocationFeature",
        }
    }
    pub fn base(&self) -> &RoomFeatureBase {
        match self {
            RoomFeature::FloodFillBox(f) => &f.base,
            RoomFeature::FloodFillProceduralPillar => todo!(),
            RoomFeature::SpawnTriggerFeature(f) => &f.base,
            RoomFeature::FloodFillPillar(f) => &f.base,
            RoomFeature::RandomSelector(f) => &f.base,
            RoomFeature::EntranceFeature(f) => &f.base,
            RoomFeature::RandomSubRoomFeature => todo!(),
            RoomFeature::SpawnActorFeature(f) => &f.base,
            RoomFeature::FloodFillLine(f) => &f.base,
            RoomFeature::ResourceFeature(f) => &f.base,
            RoomFeature::SubRoomFeature => todo!(),
            RoomFeature::DropPodCalldownLocationFeature(f) => &f.base,
        }
    }
}

impl<C: Read + Seek> FromProperty<C> for RoomFeature {
    fn from_property(asset: &Asset<C>, property: &Property) -> Result<Self> {
        from_object_property(asset, property)
    }
}
impl<C: Read + Seek> ToProperty<C> for RoomFeature {
    fn get_type() -> Option<&'static str> {
        Some("ObjectProperty")
    }
    fn to_property(
        &self,
        ctx: &mut CtxSer<C>,
        name: unreal_asset::types::FName,
        ancestry: unreal_asset::unversioned::Ancestry,
    ) -> Result<Option<Property>> {
        Ok(Some(
            ObjectProperty {
                name,
                ancestry,
                property_guid: None,
                duplication_index: 0,
                value: self.to_export(ctx)?,
            }
            .into(),
        ))
    }
}

impl<C: Seek + Read> FromExport<C> for RoomFeature {
    fn from_export(asset: &Asset<C>, package_index: PackageIndex) -> Result<Self> {
        let export = resolve_package_index(asset, package_index)?;
        let name = asset
            .get_import(export.get_base_export().class_index)
            .unwrap()
            .object_name
            .get_owned_content();

        let res = match name.as_str() {
            "FloodFillBox" => {
                RoomFeature::FloodFillBox(FromExport::from_export(asset, package_index)?)
            }
            "SpawnTriggerFeature" => {
                RoomFeature::SpawnTriggerFeature(FromExport::from_export(asset, package_index)?)
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
            "SpawnActorFeature" => {
                RoomFeature::SpawnActorFeature(FromExport::from_export(asset, package_index)?)
            }
            "FloodFillLine" => {
                RoomFeature::FloodFillLine(FromExport::from_export(asset, package_index)?)
            }
            "ResourceFeature" => {
                RoomFeature::ResourceFeature(FromExport::from_export(asset, package_index)?)
            }
            "DropPodCalldownLocationFeature" => RoomFeature::DropPodCalldownLocationFeature(
                FromExport::from_export(asset, package_index)?,
            ),
            _ => unimplemented!("{}", name),
        };
        Ok(res)
    }
}
impl<C: Seek + Read> ToExport<C> for RoomFeature {
    fn to_export(&self, ctx: &mut CtxSer<C>) -> Result<PackageIndex> {
        match self {
            RoomFeature::FloodFillBox(_) => todo!(),
            RoomFeature::FloodFillProceduralPillar => todo!(),
            RoomFeature::SpawnTriggerFeature(_) => todo!(),
            RoomFeature::FloodFillPillar(f) => f.to_export(ctx),
            RoomFeature::RandomSelector(f) => f.to_export(ctx),
            RoomFeature::EntranceFeature(f) => f.to_export(ctx),
            RoomFeature::RandomSubRoomFeature => todo!(),
            RoomFeature::SpawnActorFeature(_) => todo!(),
            RoomFeature::FloodFillLine(f) => f.to_export(ctx),
            RoomFeature::ResourceFeature(_) => todo!(),
            RoomFeature::SubRoomFeature => todo!(),
            RoomFeature::DropPodCalldownLocationFeature(f) => f.to_export(ctx),
        }
    }
}

#[derive(
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Clone,
    Serialize,
    FromExport,
    FromProperties,
    ToProperties,
)]
pub struct FloodFillBox {
    #[serde(flatten)]
    pub base: RoomFeatureBase,
    pub noise: Option<bool>, // TODO import Option<UFloodFillSettings>,
    pub position: FVector,
    pub extends: FVector,
    pub rotation: FRotator,
    pub is_carver: bool,
    pub noise_range: OrderedFloat<f32>,
}

#[derive(
    Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone, Serialize, FromExport, FromProperties,
)]
pub struct SpawnTriggerFeature {
    #[serde(flatten)]
    pub base: RoomFeatureBase,
    pub trigger_class: (), //Option<TSubclassOf<AActor>>
    pub transform: FTransform,
    pub message: FName,
}

#[derive(
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Clone,
    Serialize,
    FromProperty,
    FromProperties,
    ToProperties,
    ToProperty,
)]
pub struct FRandRange {
    pub min: OrderedFloat<f32>,
    pub max: OrderedFloat<f32>,
}

#[derive(
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Clone,
    Serialize,
    FromProperty,
    FromProperties,
    ToProperty,
    ToProperties,
)]
pub struct FRandLinePoint {
    pub location: FVector,
    pub range: FRandRange,
    pub noise_range: FRandRange,
    pub skew_factor: FRandRange,
    pub fill_amount: FRandRange,
}

#[derive(
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Clone,
    Serialize,
    FromExport,
    FromProperties,
    ToExport,
    ToProperties,
)]
pub struct FloodFillPillar {
    #[serde(flatten)]
    pub base: RoomFeatureBase,
    pub noise_override: (), // Option<UFloodFillSettings>,
    pub points: Vec<FRandLinePoint>,
    pub range_scale: FRandRange,
    pub noise_range_scale: FRandRange,
    pub endcap_scale: FRandRange,
}
impl<C: Read + Seek> BaseExportGetter<C> for FloodFillPillar {
    fn base_export(&self, ctx: &mut CtxSer<C>) -> Result<BaseExport> {
        const NAME: &str = "FloodFillPillar";
        const CLASS: ImportChain = ImportChain {
            outer: Some(&FSD_PACKAGE),
            class_package: "/Script/CoreUObject",
            class_name: "Class",
            object_name: NAME,
        };
        const CDO: ImportChain = ImportChain {
            outer: Some(&FSD_PACKAGE),
            class_package: "/Script/FSD",
            class_name: NAME,
            object_name: "Default__FloodFillPillar",
        };
        let class_index = get_import(ctx.asset, &CLASS);
        let template_index = get_import(ctx.asset, &CDO);
        let object_name = ctx
            .asset
            .add_fname_with_number(NAME, ctx.name_counter.next(NAME));
        Ok(BaseExport {
            class_index: ctx.ser_dep(class_index),
            template_index: ctx.ser_dep(template_index),
            object_name,
            not_always_loaded_for_editor_game: true,
            ..Default::default()
        })
    }
}

#[derive(
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Clone,
    Serialize,
    FromExport,
    FromProperties,
    ToExport,
    ToProperties,
)]
pub struct RandomSelector {
    #[serde(flatten)]
    pub base: RoomFeatureBase,
    pub min: i32,
    pub max: i32,
}
impl<C: Read + Seek> BaseExportGetter<C> for RandomSelector {
    fn base_export(&self, ctx: &mut CtxSer<C>) -> Result<BaseExport> {
        const NAME: &str = "RandomSelector";
        const CLASS: ImportChain = ImportChain {
            outer: Some(&FSD_PACKAGE),
            class_package: "/Script/CoreUObject",
            class_name: "Class",
            object_name: NAME,
        };
        const CDO: ImportChain = ImportChain {
            outer: Some(&FSD_PACKAGE),
            class_package: "/Script/FSD",
            class_name: NAME,
            object_name: "Default__RandomSelector",
        };
        let class_index = get_import(ctx.asset, &CLASS);
        let template_index = get_import(ctx.asset, &CDO);
        let object_name = ctx
            .asset
            .add_fname_with_number(NAME, ctx.name_counter.next(NAME));
        Ok(BaseExport {
            class_index: ctx.ser_dep(class_index),
            template_index: ctx.ser_dep(template_index),
            object_name,
            not_always_loaded_for_editor_game: true,
            ..Default::default()
        })
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct FVector {
    pub x: OrderedFloat<f32>,
    pub y: OrderedFloat<f32>,
    pub z: OrderedFloat<f32>,
}

impl<C: Read + Seek> FromProperty<C> for FVector {
    fn from_property(_asset: &Asset<C>, property: &Property) -> Result<Self> {
        match property {
            Property::StructProperty(property) => match &property.value[0] {
                Property::VectorProperty(property) => Ok(Self {
                    x: (property.value.x.0 as f32).into(),
                    y: (property.value.y.0 as f32).into(),
                    z: (property.value.z.0 as f32).into(),
                }),
                _ => bail!("{property:?}"),
            },
            _ => bail!("{property:?}"),
        }
    }
}
impl<C: Read + Seek> ToProperty<C> for FVector {
    fn get_type() -> Option<&'static str> {
        Some("StructProperty")
    }
    fn to_property(
        &self,
        ctx: &mut CtxSer<C>,
        name: unreal_asset::types::FName,
        ancestry: unreal_asset::unversioned::Ancestry,
    ) -> Result<Option<Property>> {
        Ok(Some(Property::StructProperty(StructProperty {
            name: name.clone(),
            ancestry: ancestry.clone(),
            struct_type: Some(ctx.asset.add_fname("Vector")),
            struct_guid: Some(Default::default()),
            property_guid: None,
            duplication_index: 0,
            serialize_none: true,
            value: vec![VectorProperty {
                name,
                ancestry,
                property_guid: None,
                duplication_index: 0,
                value: Vector {
                    x: (self.x.0 as f64).into(),
                    y: (self.y.0 as f64).into(),
                    z: (self.z.0 as f64).into(),
                },
            }
            .into()],
        })))
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct FRotator {
    pub yaw: OrderedFloat<f32>,
    pub pitch: OrderedFloat<f32>,
    pub roll: OrderedFloat<f32>,
}

impl<C: Read + Seek> FromProperty<C> for FRotator {
    fn from_property(_asset: &Asset<C>, property: &Property) -> Result<Self> {
        match property {
            Property::StructProperty(property) => match &property.value[0] {
                Property::RotatorProperty(property) => Ok(Self {
                    pitch: (property.value.x.0 as f32).into(),
                    yaw: (property.value.y.0 as f32).into(),
                    roll: (property.value.z.0 as f32).into(),
                }),
                _ => bail!("{property:?}"),
            },
            _ => bail!("{property:?}"),
        }
    }
}
impl<C: Read + Seek> ToProperty<C> for FRotator {
    fn get_type() -> Option<&'static str> {
        Some("StructProperty")
    }
    fn to_property(
        &self,
        ctx: &mut CtxSer<C>,
        name: unreal_asset::types::FName,
        ancestry: unreal_asset::unversioned::Ancestry,
    ) -> Result<Option<Property>> {
        Ok(Some(Property::StructProperty(StructProperty {
            name: name.clone(),
            ancestry: ancestry.clone(),
            struct_type: Some(ctx.asset.add_fname("Rotator")),
            struct_guid: Some(Default::default()),
            property_guid: None,
            duplication_index: 0,
            serialize_none: true,
            value: vec![RotatorProperty {
                name,
                ancestry,
                property_guid: None,
                duplication_index: 0,
                value: Vector {
                    x: (self.pitch.0 as f64).into(),
                    y: (self.yaw.0 as f64).into(),
                    z: (self.roll.0 as f64).into(),
                },
            }
            .into()],
        })))
    }
}

#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    FromProperty,
    FromProperties,
)]
pub struct FTransform {
    pub translation: FVector,
    pub rotation: FQuat,
    pub Scale3D: FVector,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct FQuat {
    pub x: OrderedFloat<f32>,
    pub y: OrderedFloat<f32>,
    pub z: OrderedFloat<f32>,
    pub w: OrderedFloat<f32>,
}

impl<C: Read + Seek> FromProperty<C> for FQuat {
    fn from_property(_asset: &Asset<C>, property: &Property) -> Result<Self> {
        match property {
            Property::StructProperty(property) => match &property.value[0] {
                Property::QuatProperty(property) => Ok(Self {
                    x: (property.value.x.0 as f32).into(),
                    y: (property.value.y.0 as f32).into(),
                    z: (property.value.z.0 as f32).into(),
                    w: (property.value.z.0 as f32).into(),
                }),
                _ => bail!("{property:?}"),
            },
            _ => bail!("{property:?}"),
        }
    }
}
impl<C: Read + Seek> ToProperty<C> for FQuat {
    fn get_type() -> Option<&'static str> {
        Some("StructProperty")
    }
    fn to_property(
        &self,
        _ctx: &mut CtxSer<C>,
        name: unreal_asset::types::FName,
        ancestry: unreal_asset::unversioned::Ancestry,
    ) -> Result<Option<Property>> {
        Ok(Some(
            QuatProperty {
                name,
                ancestry,
                property_guid: None,
                duplication_index: 0,
                value: Vector4 {
                    x: (self.x.0 as f64).into(),
                    y: (self.y.0 as f64).into(),
                    z: (self.z.0 as f64).into(),
                    w: (self.w.0 as f64).into(),
                },
            }
            .into(),
        ))
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct FName(String);

impl<C: Read + Seek> FromProperty<C> for FName {
    fn from_property(_asset: &Asset<C>, property: &Property) -> Result<Self> {
        match property {
            Property::NameProperty(property) => Ok(Self(property.value.get_owned_content())),
            _ => bail!("{property:?}"),
        }
    }
}
impl<C: Read + Seek> ToProperty<C> for FName {
    fn get_type() -> Option<&'static str> {
        todo!()
    }
    fn to_property(
        &self,
        ctx: &mut CtxSer<C>,
        name: unreal_asset::types::FName,
        ancestry: unreal_asset::unversioned::Ancestry,
    ) -> Result<Option<Property>> {
        Ok(Some(
            NameProperty {
                name,
                ancestry,
                property_guid: None,
                duplication_index: 0,
                value: ctx.asset.add_fname(&self.0),
            }
            .into(),
        ))
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
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
impl<C: Read + Seek> ToProperty<C> for ECaveEntranceType {
    fn get_type() -> Option<&'static str> {
        todo!()
    }
    fn to_property(
        &self,
        ctx: &mut CtxSer<C>,
        name: unreal_asset::types::FName,
        ancestry: unreal_asset::unversioned::Ancestry,
    ) -> Result<Option<Property>> {
        if *self == Self::default() {
            return Ok(None);
        }
        Ok(Some(
            EnumProperty {
                name,
                ancestry,
                property_guid: None,
                duplication_index: 0,
                value: Some(ctx.asset.add_fname(match self {
                    ECaveEntranceType::EntranceAndExit => "ECaveEntranceType::EntranceAndExit",
                    ECaveEntranceType::Entrance => "ECaveEntranceType::Entrance",
                    ECaveEntranceType::Exit => "ECaveEntranceType::Exit",
                    ECaveEntranceType::TreassureRoom => "ECaveEntranceType::TreassureRoom",
                })),
                enum_type: todo!(),
                inner_type: todo!(),
            }
            .into(),
        ))
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum ECaveEntrancePriority {
    #[default]
    Primary,
    Secondary,
}

impl<C: Read + Seek> FromProperty<C> for ECaveEntrancePriority {
    fn from_property(_asset: &Asset<C>, property: &Property) -> Result<Self> {
        dbg!(&property);
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
impl<C: Read + Seek> ToProperty<C> for ECaveEntrancePriority {
    fn get_type() -> Option<&'static str> {
        todo!()
    }
    fn to_property(
        &self,
        ctx: &mut CtxSer<C>,
        name: unreal_asset::types::FName,
        ancestry: unreal_asset::unversioned::Ancestry,
    ) -> Result<Option<Property>> {
        if *self == Self::default() {
            return Ok(None);
        }
        Ok(Some(
            EnumProperty {
                name,
                ancestry,
                property_guid: None,
                duplication_index: 0,
                value: Some(ctx.asset.add_fname(match self {
                    ECaveEntrancePriority::Primary => "ECaveEntrancePriority::Primary",
                    ECaveEntrancePriority::Secondary => "ECaveEntrancePriority::Secondary",
                })),
                enum_type: Some(ctx.asset.add_fname("ECaveEntrancePriority")),
                inner_type: None,
            }
            .into(),
        ))
    }
}

#[derive(
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Clone,
    Serialize,
    FromExport,
    FromProperties,
    ToExport,
    ToProperties,
)]
pub struct EntranceFeature {
    #[serde(flatten)]
    pub base: RoomFeatureBase,
    pub location: FVector,
    pub direction: FRotator,
    pub entrance_type: ECaveEntranceType,
    pub priority: ECaveEntrancePriority,
}
impl<C: Read + Seek> BaseExportGetter<C> for EntranceFeature {
    fn base_export(&self, ctx: &mut CtxSer<C>) -> Result<BaseExport> {
        const NAME: &str = "EntranceFeature";
        const CLASS: ImportChain = ImportChain {
            outer: Some(&FSD_PACKAGE),
            class_package: "/Script/CoreUObject",
            class_name: "Class",
            object_name: NAME,
        };
        const CDO: ImportChain = ImportChain {
            outer: Some(&FSD_PACKAGE),
            class_package: "/Script/FSD",
            class_name: NAME,
            object_name: "Default__EntranceFeature",
        };
        let class_index = get_import(ctx.asset, &CLASS);
        let template_index = get_import(ctx.asset, &CDO);
        let object_name = ctx
            .asset
            .add_fname_with_number(NAME, ctx.name_counter.next(NAME));
        Ok(BaseExport {
            class_index: ctx.ser_dep(class_index),
            template_index: ctx.ser_dep(template_index),
            object_name,
            not_always_loaded_for_editor_game: true,
            ..Default::default()
        })
    }
}

#[derive(
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Clone,
    Serialize,
    FromProperty,
    FromProperties,
    ToProperty,
    ToProperties,
)]
pub struct FRoomLinePoint {
    pub location: FVector,
    pub h_range: OrderedFloat<f32>,
    pub v_range: OrderedFloat<f32>,
    pub cieling_noise_range: OrderedFloat<f32>,
    pub wall_noise_range: OrderedFloat<f32>,
    pub floor_noise_range: OrderedFloat<f32>,
    pub cielingheight: OrderedFloat<f32>,
    pub height_scale: OrderedFloat<f32>,
    pub floor_depth: OrderedFloat<f32>,
    pub floor_angle: OrderedFloat<f32>,
}

#[derive(Debug, Default, Serialize, FromProperty, FromProperties)]
pub struct FLayeredNoise {
    pub noise: (), // UFloodFillSettings,
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

#[derive(
    Debug,
    Default,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    FromExport,
    FromProperties,
    ToExport,
    ToProperties,
)]
pub struct FloodFillLine {
    #[serde(flatten)]
    pub base: RoomFeatureBase,
    pub wall_noise_override: (),    // Option<UFloodFillSettings>,
    pub ceiling_noise_override: (), // Option<UFloodFillSettings>,
    pub flood_noise_override: (),   // Option<UFloodFillSettings>,
    pub use_detail_noise: bool,
    pub points: Vec<FRoomLinePoint>,
}
impl<C: Read + Seek> BaseExportGetter<C> for FloodFillLine {
    fn base_export(&self, ctx: &mut CtxSer<C>) -> Result<BaseExport> {
        const NAME: &str = "FloodFillLine";
        const CLASS: ImportChain = ImportChain {
            outer: Some(&FSD_PACKAGE),
            class_package: "/Script/CoreUObject",
            class_name: "Class",
            object_name: NAME,
        };
        const CDO: ImportChain = ImportChain {
            outer: Some(&FSD_PACKAGE),
            class_package: "/Script/FSD",
            class_name: NAME,
            object_name: "Default__FloodFillLine",
        };
        let class_index = get_import(ctx.asset, &CLASS);
        let template_index = get_import(ctx.asset, &CDO);
        let object_name = ctx
            .asset
            .add_fname_with_number(NAME, ctx.name_counter.next(NAME));
        Ok(BaseExport {
            class_index: ctx.ser_dep(class_index),
            template_index: ctx.ser_dep(template_index),
            object_name,
            not_always_loaded_for_editor_game: true,
            ..Default::default()
        })
    }
}

#[derive(
    Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, FromExport, FromProperties,
)]
pub struct ResourceFeature {
    #[serde(flatten)]
    pub base: RoomFeatureBase,
    pub location: FVector,
    pub resource: (), // Option<UResourceData>,
    pub base_amount: OrderedFloat<f32>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum EItemAdjustmentType {
    #[default]
    None,
    Cieling,
    Wall,
    Floor,
}
impl<C: Read + Seek> FromProperty<C> for EItemAdjustmentType {
    fn from_property(_asset: &Asset<C>, property: &Property) -> Result<Self> {
        match property {
            Property::EnumProperty(property) => property.value.as_ref().unwrap().get_content(|c| {
                Ok(match c {
                    "EItemAdjustmentType::None" => EItemAdjustmentType::None,
                    "EItemAdjustmentType::Ceiling" => EItemAdjustmentType::Cieling,
                    "EItemAdjustmentType::Wall" => EItemAdjustmentType::Wall,
                    "EItemAdjustmentType::Floor" => EItemAdjustmentType::Floor,
                    _ => bail!("unknown variant {}", c),
                })
            }),
            _ => bail!("{property:?}"),
        }
    }
}

#[derive(
    Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, FromExport, FromProperties,
)]
pub struct SpawnActorFeature {
    #[serde(flatten)]
    pub base: RoomFeatureBase,
    pub location: FVector,
    pub actor_to_spawn: (), // TODO TSubclassOf<AActor>
    pub adjustment_direction: FVector,
    pub adjustment: EItemAdjustmentType,
    pub scale_min: FVector,
    pub scale_max: FVector,
    pub rotation_delta: FRotator,
}

#[derive(
    Debug,
    Default,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    FromExport,
    FromProperties,
    ToExport,
    ToProperties,
)]
pub struct DropPodCalldownLocationFeature {
    #[serde(flatten)]
    pub base: RoomFeatureBase,
    pub location: FVector,
    pub call_down_class: (), // TSubclassOf<AActor>
}
impl<C: Read + Seek> BaseExportGetter<C> for DropPodCalldownLocationFeature {
    fn base_export(&self, ctx: &mut CtxSer<C>) -> Result<BaseExport> {
        const NAME: &str = "DropPodCalldownLocationFeature";
        const CLASS: ImportChain = ImportChain {
            outer: Some(&FSD_PACKAGE),
            class_package: "/Script/CoreUObject",
            class_name: "Class",
            object_name: NAME,
        };
        const CDO: ImportChain = ImportChain {
            outer: Some(&FSD_PACKAGE),
            class_package: "/Script/FSD",
            class_name: NAME,
            object_name: "Default__DropPodCalldownLocation",
        };
        let class_index = get_import(ctx.asset, &CLASS);
        let template_index = get_import(ctx.asset, &CDO);
        let object_name = ctx
            .asset
            .add_fname_with_number(NAME, ctx.name_counter.next(NAME));
        Ok(BaseExport {
            class_index: ctx.ser_dep(class_index),
            template_index: ctx.ser_dep(template_index),
            object_name,
            not_always_loaded_for_editor_game: true,
            ..Default::default()
        })
    }
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone, Serialize)]
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
impl<C: Read + Seek> ToProperty<C> for ERoomMirroringSupport {
    fn get_type() -> Option<&'static str> {
        todo!()
    }
    fn to_property(
        &self,
        ctx: &mut CtxSer<C>,
        name: unreal_asset::types::FName,
        ancestry: unreal_asset::unversioned::Ancestry,
    ) -> Result<Option<Property>> {
        if *self == Self::default() {
            return Ok(None);
        }
        todo!()
    }
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone, Serialize)]
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
impl<C: Read + Seek> ToProperty<C> for FGameplayTagContainer {
    fn get_type() -> Option<&'static str> {
        todo!()
    }
    fn to_property(
        &self,
        ctx: &mut CtxSer<C>,
        name: unreal_asset::types::FName,
        ancestry: unreal_asset::unversioned::Ancestry,
    ) -> Result<Option<Property>> {
        if self.tags.is_empty() {
            return Ok(None);
        }
        Ok(Some(Property::StructProperty(StructProperty {
            name: name.clone(),
            ancestry: ancestry.clone(),
            struct_type: Some(ctx.asset.add_fname("GameplayTagContainer")),
            struct_guid: Some(Default::default()),
            property_guid: None,
            duplication_index: 0,
            serialize_none: true,
            value: vec![GameplayTagContainerProperty {
                name,
                ancestry,
                property_guid: None,
                duplication_index: 0,
                value: self.tags.iter().map(|t| ctx.asset.add_fname(t)).collect(),
            }
            .into()],
        })))
    }
}

#[derive(
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Clone,
    Serialize,
    FromExport,
    FromProperties,
    ToProperties,
)]
pub struct RoomGeneratorBase {
    pub can_only_be_used_once: bool,
    pub mirror_support: ERoomMirroringSupport,
    pub room_tags: FGameplayTagContainer,
    pub bounds: OrderedFloat<f32>,
}

#[derive(
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Clone,
    Serialize,
    FromExport,
    FromProperties,
    ToExport,
    ToProperties,
)]
pub struct RoomGenerator {
    #[serde(flatten)]
    pub base: RoomGeneratorBase,
    pub room_features: Vec<RoomFeature>,
}
impl<C: Read + Seek> BaseExportGetter<C> for RoomGenerator {
    fn base_export(&self, ctx: &mut CtxSer<C>) -> Result<BaseExport> {
        const NAME: &str = "RoomGenerator";
        const CLASS: ImportChain = ImportChain {
            outer: Some(&FSD_PACKAGE),
            class_package: "/Script/CoreUObject",
            class_name: "Class",
            object_name: NAME,
        };
        const CDO: ImportChain = ImportChain {
            outer: Some(&FSD_PACKAGE),
            class_package: "/Script/FSD",
            class_name: NAME,
            object_name: "Default__RoomGenerator",
        };
        let class_index = get_import(ctx.asset, &CLASS);
        let template_index = get_import(ctx.asset, &CDO);
        Ok(BaseExport {
            class_index: ctx.ser_dep(class_index),
            template_index: ctx.ser_dep(template_index),
            object_name: ctx.asset.add_fname("TODO"),
            object_flags: EObjectFlags::RF_PUBLIC
                | EObjectFlags::RF_STANDALONE
                | EObjectFlags::RF_TRANSACTIONAL,
            not_always_loaded_for_editor_game: true,
            is_asset: true,
            ..Default::default()
        })
    }
}
