mod rma;

use rma::{FloodFillLine, RoomGenerator};
use rma_lib::FromExport;

use anyhow::Result;
use three_d::*;
use unreal_asset::engine_version::EngineVersion;
use unreal_asset::exports::ExportBaseTrait;
use unreal_asset::types::PackageIndex;
use unreal_asset::Asset;

use std::io::Cursor;
use std::path::Path;
use std::{fs, ops::Deref};

use crate::rma::{FVector, RoomFeature};

pub fn read_asset<P: AsRef<Path>>(
    path: P,
    version: EngineVersion,
) -> Result<Asset<Cursor<Vec<u8>>>> {
    let uasset = Cursor::new(fs::read(&path)?);
    let uexp = Cursor::new(fs::read(path.as_ref().with_extension("uexp"))?);
    let asset = Asset::new(uasset, Some(uexp), version, None)?;

    Ok(asset)
}

fn read_rma<P: AsRef<Path>>(path: P) -> Result<RoomGenerator> {
    let asset = read_asset(path, EngineVersion::VER_UE4_27)?;

    let root = asset
        .asset_data
        .exports
        .iter()
        .enumerate()
        .find_map(|(i, export)| {
            (export.get_base_export().outer_index.index == 0)
                .then(|| PackageIndex::from_export(i as i32).unwrap())
        })
        .unwrap();

    RoomGenerator::from_export(&asset, root)
}

pub fn main() -> Result<()> {
    let rma = read_rma("RMA_BigBridge02.uasset")?;
    dbg!(&rma);

    let window = Window::new(WindowSettings {
        title: "Shapes!".to_string(),
        max_size: Some((1280, 720)),
        ..Default::default()
    })
    .unwrap();
    let context = window.gl();

    let mut camera = Camera::new_perspective(
        window.viewport(),
        vec3(5000.0, 0.0, 2.5),
        vec3(0.0, 0.0, 0.0),
        vec3(0.0, 0.0, 1.0),
        degrees(45.0),
        0.1,
        100000.0,
    );
    let mut control = OrbitControl::new(*camera.target(), 1.0, 100000.0);

    let mut primitives = vec![];

    let mut wireframe_material = PhysicalMaterial::new_opaque(
        &context,
        &CpuMaterial {
            albedo: Srgba {
                r: 255,
                g: 0,
                b: 0,
                a: 200,
            },
            //albedo: Srgba::new_opaque(220, 50, 50),
            //roughness: 0.7,
            //metallic: 0.8,
            ..Default::default()
        },
    );
    wireframe_material.render_states.cull = Cull::Back;
    let mut cylinder = CpuMesh::cylinder(10);
    cylinder
        .transform(&Mat4::from_nonuniform_scale(1.0, 10.0, 10.0))
        .unwrap();

    fn iter_features<F>(features: Vec<RoomFeature>, f: &mut F)
    where
        F: FnMut(&RoomFeature),
    {
        for feat in features {
            f(&feat);
            match feat {
                RoomFeature::FloodFillBox => todo!(),
                RoomFeature::FloodFillProceduralPillar => todo!(),
                RoomFeature::SpawnTriggerFeature => todo!(),
                RoomFeature::FloodFillPillar(feat) => iter_features(feat.base.room_features, f),
                RoomFeature::RandomSelector(feat) => iter_features(feat.base.room_features, f),
                RoomFeature::EntranceFeature(feat) => iter_features(feat.base.room_features, f),
                RoomFeature::RandomSubRoomFeature => todo!(),
                RoomFeature::SpawnActorFeature => todo!(),
                RoomFeature::FloodFillLine(feat) => iter_features(feat.base.room_features, f),
                RoomFeature::ResourceFeature => todo!(),
                RoomFeature::SubRoomFeature => todo!(),
                RoomFeature::DropPodCalldownLocationFeature => todo!(),
            }
        }
    }

    iter_features(rma.room_features, &mut |f| match f {
        RoomFeature::FloodFillLine(f) => {
            primitives.push(Box::new(Gm::new(
                InstancedMesh::new(&context, &edge_transformations(f), &cylinder),
                wireframe_material.clone(),
            )));
        }
        _ => {}
    });
    primitives.truncate(1);

    let axes = Axes::new(&context, 10., 200.0);

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
                axes.into_iter()
                    .chain(primitives.iter().map(|p| -> &dyn Object { p.deref() })),
                &[&light0, &light1],
            );

        FrameOutput::default()
    });

    Ok(())
}

impl From<FVector> for Vector3<f32> {
    fn from(val: FVector) -> Self {
        vec3(val.x, val.y, val.z)
    }
}

fn edge_transformations(line: &FloodFillLine) -> Instances {
    let mut transformations = Vec::new();

    let mut add_line = |p1: Vector3<f32>, p2: Vector3<f32>| {
        transformations.push(
            Mat4::from_translation(p1)
                * Into::<Mat4>::into(Quat::from_arc(
                    vec3(1.0, 0.0, 0.0),
                    (p2 - p1).normalize(),
                    None,
                ))
                * Mat4::from_nonuniform_scale((p1 - p2).magnitude(), 1.0, 1.0),
        );
    };

    for pair in line.points.windows(2) {
        add_line(pair[0].location.into(), pair[1].location.into());
    }

    // horizontal perimeter circle
    for point in &line.points {
        let segments = 40;
        let mut iter = (0..segments + 1)
            .map(|i| {
                let angle = 2.0 * std::f32::consts::PI * i as f32 / segments as f32;
                (angle.cos(), angle.sin())
            })
            .peekable();
        while let (Some(a), Some(b)) = (iter.next(), iter.peek()) {
            add_line(
                vec3(
                    point.location.x + point.h_range * a.0,
                    point.location.y + point.h_range * a.1,
                    point.location.z,
                ),
                vec3(
                    point.location.x + point.h_range * b.0,
                    point.location.y + point.h_range * b.1,
                    point.location.z,
                ),
            );
        }
    }

    // vertical half circles
    for point in &line.points {
        let segments = 40;
        let mut iter = (0..segments / 2 + 1)
            .map(|i| {
                let angle = 2.0 * std::f32::consts::PI * i as f32 / segments as f32;
                (angle.cos(), angle.sin())
            })
            .peekable();
        while let (Some(a), Some(b)) = (iter.next(), iter.peek()) {
            add_line(
                vec3(
                    point.location.x + point.h_range * a.0,
                    point.location.y,
                    point.location.z + point.v_range * a.1,
                ),
                vec3(
                    point.location.x + point.h_range * b.0,
                    point.location.y,
                    point.location.z + point.v_range * b.1,
                ),
            );
            add_line(
                vec3(
                    point.location.x,
                    point.location.y + point.h_range * a.0,
                    point.location.z + point.v_range * a.1,
                ),
                vec3(
                    point.location.x,
                    point.location.y + point.h_range * b.0,
                    point.location.z + point.v_range * b.1,
                ),
            );
        }
    }

    Instances {
        transformations,
        ..Default::default()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_load_asset() -> Result<()> {
        let rma = read_rma("../RMA_BigBridge02.uasset")?;

        std::fs::write("../room.json", serde_json::to_string_pretty(&rma)?)?;
        Ok(())
    }
}
