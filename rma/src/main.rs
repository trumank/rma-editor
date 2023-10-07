use rma_lib::{from_object_property, FromExport, FromProperties, FromProperty};
use three_d::*;

use anyhow::{bail, Result};
use unreal_asset::exports::{ExportBaseTrait, ExportNormalTrait};
use unreal_asset::properties::Property;
use unreal_asset::reader::ArchiveTrait;
use unreal_asset::types::PackageIndex;

use std::fs;
use std::io::{Cursor, Read, Seek};
use std::path::Path;
use unreal_asset::engine_version::EngineVersion;
use unreal_asset::Asset;

pub fn read_asset<P: AsRef<Path>>(
    path: P,
    version: EngineVersion,
) -> Result<Asset<Cursor<Vec<u8>>>> {
    let uasset = Cursor::new(fs::read(&path)?);
    let uexp = Cursor::new(fs::read(path.as_ref().with_extension("uexp"))?);
    let asset = Asset::new(uasset, Some(uexp), version, None)?;

    Ok(asset)
}

#[derive(Debug, Default, FromExport, FromProperties)]
struct RoomFeatureBase {
    room_features: Vec<RoomFeature>,
}

#[derive(Debug)]
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

#[derive(Debug, Default, FromProperty, FromProperties)]
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

#[derive(Debug, Default, FromProperty, FromProperties)]
struct FRandLinePoint {
    location: FVector,
    range: FRandRange,
    noise_range: FRandRange,
    skew_factor: FRandRange,
    fill_amount: FRandRange,
}

#[derive(Debug, Default, FromExport, FromProperties)]
struct FloodFillPillar {
    base: RoomFeatureBase,
    noise_override: UFloodFillSettings,
    points: Vec<FRandLinePoint>,
    range_scale: FRandRange,
    noise_range_scale: FRandRange,
    endcap_scale: FRandRange,
}

#[derive(Debug, Default, FromExport, FromProperties)]
struct RandomSelector {
    base: RoomFeatureBase,
    min: i32,
    max: i32,
}

#[derive(Debug, Default)]
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

#[derive(Debug, Default)]
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

#[derive(Debug, Default)]
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

#[derive(Debug, Default)]
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

#[derive(Debug, Default, FromExport, FromProperties)]
struct EntranceFeature {
    base: RoomFeatureBase,
    location: FVector,
    direction: FRotator,
    entrance_type: ECaveEntranceType,
    priority: ECaveEntrancePriority,
}

#[derive(Debug, Default, FromProperty, FromProperties)]
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

#[derive(Debug, Default, FromProperty, FromProperties)]
struct FLayeredNoise {
    noise: UFloodFillSettings,
    scale: f32,
}

#[derive(Debug, Default, FromExport, FromProperties)]
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

#[derive(Debug, Default, FromExport, FromProperties)]
struct FloodFillLine {
    base: RoomFeatureBase,
    wall_noise_override: UFloodFillSettings,
    ceiling_noise_override: UFloodFillSettings,
    flood_noise_override: UFloodFillSettings,
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
        Ok(dbg!(res))
    }
}

impl<C: Read + Seek> FromProperty<C> for RoomFeature {
    fn from_property(asset: &Asset<C>, property: &Property) -> Result<Self> {
        from_object_property(asset, property)
    }
}

#[derive(Debug, Default)]
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

#[derive(Debug, Default)]
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

#[derive(Debug, Default, FromExport, FromProperties)]
struct RoomGeneratorBase {
    bounds: f32,
    can_only_be_used_once: bool,
    mirror_support: ERoomMirroringSupport,
    room_tags: FGameplayTagContainer,
}

#[derive(Debug, Default, FromExport, FromProperties)]
struct RoomGenerator {
    base: RoomGeneratorBase,
    room_features: Vec<RoomFeature>,
}

pub fn main() {
    let asset = read_asset("RMA_BigBridge02.uasset", EngineVersion::VER_UE4_27);

    let window = Window::new(WindowSettings {
        title: "Shapes!".to_string(),
        max_size: Some((1280, 720)),
        ..Default::default()
    })
    .unwrap();
    let context = window.gl();

    let mut camera = Camera::new_perspective(
        window.viewport(),
        vec3(5.0, 2.0, 2.5),
        vec3(0.0, 0.0, -0.5),
        vec3(0.0, 1.0, 0.0),
        degrees(45.0),
        0.1,
        1000.0,
    );
    let mut control = OrbitControl::new(*camera.target(), 1.0, 100.0);

    let mut sphere = Gm::new(
        Mesh::new(&context, &CpuMesh::sphere(16)),
        PhysicalMaterial::new_transparent(
            &context,
            &CpuMaterial {
                albedo: Srgba {
                    r: 255,
                    g: 0,
                    b: 0,
                    a: 200,
                },
                ..Default::default()
            },
        ),
    );
    sphere.set_transformation(Mat4::from_translation(vec3(0.0, 1.3, 0.0)) * Mat4::from_scale(0.2));
    let mut cylinder = Gm::new(
        Mesh::new(&context, &CpuMesh::cylinder(16)),
        PhysicalMaterial::new_transparent(
            &context,
            &CpuMaterial {
                albedo: Srgba {
                    r: 0,
                    g: 255,
                    b: 0,
                    a: 200,
                },
                ..Default::default()
            },
        ),
    );
    cylinder
        .set_transformation(Mat4::from_translation(vec3(1.3, 0.0, 0.0)) * Mat4::from_scale(0.2));
    let mut cube = Gm::new(
        Mesh::new(&context, &CpuMesh::cube()),
        PhysicalMaterial::new_transparent(
            &context,
            &CpuMaterial {
                albedo: Srgba {
                    r: 0,
                    g: 0,
                    b: 255,
                    a: 100,
                },
                ..Default::default()
            },
        ),
    );
    cube.set_transformation(Mat4::from_translation(vec3(0.0, 0.0, 1.3)) * Mat4::from_scale(0.2));
    let axes = Axes::new(&context, 0.1, 2.0);
    let bounding_box_sphere = Gm::new(
        BoundingBox::new(&context, sphere.aabb()),
        ColorMaterial {
            color: Srgba::BLACK,
            ..Default::default()
        },
    );
    let bounding_box_cube = Gm::new(
        BoundingBox::new(&context, cube.aabb()),
        ColorMaterial {
            color: Srgba::BLACK,
            ..Default::default()
        },
    );
    let bounding_box_cylinder = Gm::new(
        BoundingBox::new(&context, cylinder.aabb()),
        ColorMaterial {
            color: Srgba::BLACK,
            ..Default::default()
        },
    );

    let light0 = DirectionalLight::new(&context, 1.0, Srgba::WHITE, &vec3(0.0, -0.5, -0.5));
    let light1 = DirectionalLight::new(&context, 1.0, Srgba::WHITE, &vec3(0.0, 0.5, 0.5));

    window.render_loop(move |mut frame_input| {
        camera.set_viewport(frame_input.viewport);
        control.handle_events(&mut camera, &mut frame_input.events);

        frame_input
            .screen()
            .clear(ClearState::color_and_depth(0.0, 0.0, 0.0, 1.0, 1.0))
            .render(
                &camera,
                sphere
                    .into_iter()
                    .chain(&cylinder)
                    .chain(&cube)
                    .chain(&axes)
                    .chain(&bounding_box_sphere)
                    .chain(&bounding_box_cube)
                    .chain(&bounding_box_cylinder),
                &[&light0, &light1],
            );

        FrameOutput::default()
    });
}

#[cfg(test)]
mod test {
    use unreal_asset::exports::ExportNormalTrait;

    use super::*;

    #[test]
    fn test_load_asset() -> Result<()> {
        let asset = read_asset("../RMA_BigBridge02.uasset", EngineVersion::VER_UE4_27)?;
        //dbg!(asset.asset_data.get_class_export());
        for (i, export) in asset.asset_data.exports.iter().enumerate() {
            if let Some(normal) = export.get_normal_export() {
                if normal.base_export.outer_index.index == 0 {
                    dbg!(RoomGenerator::from_export(
                        &asset,
                        PackageIndex::from_export(i as i32).unwrap()
                    )?);
                }
            }
        }
        Ok(())
    }
}
