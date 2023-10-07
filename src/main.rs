use three_d::*;

use anyhow::Result;
use unreal_asset::exports::{ExportBaseTrait, ExportNormalTrait, NormalExport};
use unreal_asset::properties::{array_property, Property, PropertyDataTrait};
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

fn property_or_default<C: Read + Seek, T: Default + FromProperty<C>>(
    asset: &Asset<C>,
    properties: &[Property],
    name: &str,
) -> T {
    for property in properties {
        if property.get_name().get_content(|c| c == name) {
            return T::from_proeprty(asset, property);
        }
    }
    T::default()
}

trait FromExport<C: Seek + Read> {
    fn from_export(asset: &Asset<C>, package_index: PackageIndex) -> Self;
}
trait FromProperty<C: Seek + Read> {
    fn from_proeprty(asset: &Asset<C>, property: &Property) -> Self;
}

#[derive(Debug)]
struct RoomFeatureBase {
    room_features: Vec<RoomFeature>,
}
impl<C: Seek + Read> FromExport<C> for RoomFeatureBase {
    fn from_export(asset: &Asset<C>, package_index: PackageIndex) -> Self {
        let export = asset.get_export(package_index).unwrap();
        let normal_export = export.get_normal_export().unwrap();
        let properties = &normal_export.properties;
        Self {
            room_features: property_or_default(asset, properties, "RoomFeatures"),
        }
    }
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
    FloodFillLine,
    ResourceFeature,
    SubRoomFeature,
    DropPodCalldownLocationFeature,
}

#[derive(Debug, Default)]
struct FRandRange {
    min: f32,
    max: f32,
}

impl<C: Read + Seek> FromProperty<C> for FRandRange {
    fn from_proeprty(asset: &Asset<C>, property: &Property) -> Self {
        match property {
            Property::StructProperty(property) => match &property.value[0] {
                Property::VectorProperty(property) => Self {
                    min: property.value.y.0 as f32,
                    max: property.value.z.0 as f32,
                },
                _ => panic!("{property:?}"),
            },
            _ => panic!("{property:?}"),
        }
    }
}

#[derive(Debug)]
struct FloodFillPillar {
    base: RoomFeatureBase,
    //noise_override: UFloodFillSettings,
    //points: Vec<FRandLinePoint>,
    range_scale: FRandRange,
    noise_range_scale: FRandRange,
    endcap_scale: FRandRange,
}

impl<C: Seek + Read> FromExport<C> for FloodFillPillar {
    fn from_export(asset: &Asset<C>, package_index: PackageIndex) -> Self {
        let export = asset.get_export(package_index).unwrap();
        let normal_export = export.get_normal_export().unwrap();
        let properties = &normal_export.properties;

        Self {
            base: FromExport::from_export(asset, package_index),
            //location: property_or_default(asset, properties, "Location"),
            //direction: property_or_default(asset, properties, "Direction"),
            //entrance_type: Default::default(),
            range_scale: property_or_default(asset, properties, "RangeScale"),
            noise_range_scale: property_or_default(asset, properties, "NoiseRangeScale"),
            endcap_scale: property_or_default(asset, properties, "EndcapScale"),
        }
    }
}


#[derive(Debug)]
struct RandomSelector {
    base: RoomFeatureBase,
}

impl<C: Seek + Read> FromExport<C> for RandomSelector {
    fn from_export(asset: &Asset<C>, package_index: PackageIndex) -> Self {
        let export = asset.get_export(package_index).unwrap();
        let normal_export = export.get_normal_export().unwrap();
        let properties = &normal_export.properties;
        todo!();
    }
}

#[derive(Debug, Default)]
struct FVector {
    x: f32,
    y: f32,
    z: f32,
}

impl<C: Read + Seek> FromProperty<C> for FVector {
    fn from_proeprty(asset: &Asset<C>, property: &Property) -> Self {
        match property {
            Property::StructProperty(property) => match &property.value[0] {
                Property::VectorProperty(property) => Self {
                    x: property.value.x.0 as f32,
                    y: property.value.y.0 as f32,
                    z: property.value.z.0 as f32,
                },
                _ => panic!("{property:?}"),
            },
            _ => panic!("{property:?}"),
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
    fn from_proeprty(asset: &Asset<C>, property: &Property) -> Self {
        match property {
            Property::StructProperty(property) => match &property.value[0] {
                Property::RotatorProperty(property) => Self {
                    pitch: property.value.x.0 as f32,
                    yaw: property.value.y.0 as f32,
                    roll: property.value.z.0 as f32,
                },
                _ => panic!("{property:?}"),
            },
            _ => panic!("{property:?}"),
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

#[derive(Debug, Default)]
enum ECaveEntrancePriority {
    #[default]
    Primary,
    Secondary,
}

#[derive(Debug)]
struct EntranceFeature {
    base: RoomFeatureBase,
    location: FVector,
    direction: FRotator,
    entrance_type: ECaveEntranceType,
    priority: ECaveEntrancePriority,
}

impl<C: Seek + Read> FromExport<C> for EntranceFeature {
    fn from_export(asset: &Asset<C>, package_index: PackageIndex) -> Self {
        let export = asset.get_export(package_index).unwrap();
        let normal_export = export.get_normal_export().unwrap();
        let properties = &normal_export.properties;

        Self {
            base: FromExport::from_export(asset, package_index),
            location: property_or_default(asset, properties, "Location"),
            direction: property_or_default(asset, properties, "Direction"),
            entrance_type: Default::default(),
            priority: Default::default(),
        }
    }
}

impl<C: Seek + Read> FromExport<C> for RoomFeature {
    fn from_export(asset: &Asset<C>, package_index: PackageIndex) -> Self {
        let export = asset.get_export(package_index).unwrap();
        let normal_export = export.get_normal_export().unwrap();
        let name = asset
            .get_import(export.get_base_export().class_index)
            .unwrap()
            .object_name
            .get_owned_content();

        let res = match name.as_str() {
            "FloodFillPillar" => {
                RoomFeature::FloodFillPillar(FromExport::from_export(asset, package_index))
            }
            "RandomSelector" => {
                RoomFeature::RandomSelector(FromExport::from_export(asset, package_index))
            }
            "EntranceFeature" => {
                RoomFeature::EntranceFeature(FromExport::from_export(asset, package_index))
            }
            _ => unimplemented!("{}", name),
        };
        dbg!(res)
    }
}

impl<C: Read + Seek, T: FromExport<C>> FromProperty<C> for Vec<T> {
    fn from_proeprty(asset: &Asset<C>, property: &Property) -> Self {
        let mut values = vec![];
        match property {
            Property::ArrayProperty(property) => {
                for value in &property.value {
                    match value {
                        Property::ObjectProperty(property) => {
                            values.push(T::from_export(asset, property.value));
                        }
                        _ => panic!("wrong property type"),
                    }
                }
            }
            _ => panic!("wrong property type"),
        }
        values
    }
}

#[derive(Debug)]
struct RoomGenerator {
    room_features: Vec<RoomFeature>,
}

impl RoomGenerator {
    fn from<C: std::io::Seek + std::io::Read>(asset: &Asset<C>, export: PackageIndex) {
        let properties = &asset
            .get_export(export)
            .unwrap()
            .get_normal_export()
            .unwrap()
            .properties;
        for prop in properties {
            if prop.get_name().get_content(|c| c == "RoomFeatures") {
                let a: Vec<RoomFeature> = FromProperty::from_proeprty(asset, prop);
                dbg!(a);
            }
        }
        dbg!();
    }
}

#[cfg(test)]
mod test {
    use unreal_asset::exports::ExportNormalTrait;

    use super::*;

    #[test]
    fn test_load_asset() -> Result<()> {
        let asset = read_asset("RMA_BigBridge02.uasset", EngineVersion::VER_UE4_27)?;
        //dbg!(asset.asset_data.get_class_export());
        for (i, export) in asset.asset_data.exports.iter().enumerate() {
            if let Some(normal) = export.get_normal_export() {
                if normal.base_export.outer_index.index == 0 {
                    RoomGenerator::from(&asset, PackageIndex::from_export(i as i32).unwrap());
                }
                //for prop in &normal.properties {
                //dbg!(prop);
                //}
            }
        }
        Ok(())
    }
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
