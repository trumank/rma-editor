mod rma;

use rma::{
    DropPodCalldownLocationFeature, EntranceFeature, FloodFillBox, FloodFillLine, FloodFillPillar,
    RoomGenerator, SpawnActorFeature,
};
use rma_lib::FromExport;

use anyhow::Result;
use three_d::*;
use unreal_asset::engine_version::EngineVersion;
use unreal_asset::exports::ExportBaseTrait;
use unreal_asset::types::PackageIndex;
use unreal_asset::Asset;

use std::collections::HashMap;
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

struct RMAContext<'c> {
    context: &'c Context,
    wireframe_material: PhysicalMaterial,
    wireframe_mesh: CpuMesh,
}

pub fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("expected path to an RMA .uasset");
    let rma = read_rma(path)?;

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

    let mut primitives: HashMap<Vec<usize>, Vec<Box<dyn Object>>> = Default::default();

    let mut wireframe_material = PhysicalMaterial::new_opaque(
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
    );
    wireframe_material.render_states.cull = Cull::Back;
    let mut wireframe_mesh = CpuMesh::cylinder(10);
    wireframe_mesh
        .transform(&Mat4::from_nonuniform_scale(1.0, 10.0, 10.0))
        .unwrap();

    let rma_ctx = RMAContext {
        context: &context,
        wireframe_material,
        wireframe_mesh,
    };

    fn iter_features<F, T>(features: &[RoomFeature], path: &mut Vec<usize>, f: &mut F)
    where
        F: FnMut(&RoomFeature, &[usize]) -> T,
    {
        path.push(0);
        for (i, feat) in features.iter().enumerate() {
            *path.last_mut().unwrap() = i;
            f(feat, path);
            iter_features(&feat.base().room_features, path, f);
        }
    }

    let mut path = vec![];
    iter_features(&rma.room_features, &mut path, &mut |f, path| match f {
        RoomFeature::FloodFillBox(f) => {
            primitives.insert(path.to_vec(), flood_fill_box(&rma_ctx, f));
        }
        RoomFeature::FloodFillPillar(f) => {
            primitives.insert(path.to_vec(), flood_fill_pillar(&rma_ctx, f));
        }
        RoomFeature::SpawnActorFeature(f) => {
            primitives.insert(path.to_vec(), spawn_actor_feature(&rma_ctx, f));
        }
        RoomFeature::FloodFillLine(f) => {
            primitives.insert(path.to_vec(), flood_fill_line(&rma_ctx, f));
        }
        RoomFeature::EntranceFeature(f) => {
            primitives.insert(path.to_vec(), entrance_feature(&rma_ctx, f));
        }
        RoomFeature::DropPodCalldownLocationFeature(f) => {
            primitives.insert(
                path.to_vec(),
                drop_pod_calldown_location_feature(&rma_ctx, f),
            );
        }
        _ => {}
    });

    let axes = Axes::new(&context, 10., 200.0);

    let light0 = DirectionalLight::new(&context, 1.0, Srgba::WHITE, &vec3(0.0, -0.5, -0.5));
    let light1 = DirectionalLight::new(&context, 1.0, Srgba::WHITE, &vec3(0.0, 0.5, 0.5));

    #[derive(Default)]
    struct State {
        visible: bool,
    }

    let mut gui = three_d::GUI::new(&context);
    let mut states = HashMap::<Vec<usize>, State>::new();

    window.render_loop(move |mut frame_input| {
        let mut panel_width = 0.0;
        gui.update(
            &mut frame_input.events,
            frame_input.accumulated_time,
            frame_input.viewport,
            frame_input.device_pixel_ratio,
            |gui_context| {
                use three_d::egui::*;
                SidePanel::left("side_panel").show(gui_context, |ui| {
                    use three_d::egui::*;
                    ui.heading("Debug Panel");
                    fn features(
                        ui: &mut Ui,
                        path: &mut Vec<usize>,
                        f: &[RoomFeature],
                        states: &mut HashMap<Vec<usize>, State>,
                    ) {
                        path.push(0);
                        for (i, f) in f.iter().enumerate() {
                            *path.last_mut().unwrap() = i;

                            let id = ui.make_persistent_id(i);
                            egui::collapsing_header::CollapsingState::load_with_default_open(
                                ui.ctx(),
                                id,
                                true,
                            )
                            .show_header(ui, |ui| {
                                ui.checkbox(
                                    &mut states.entry(path.clone()).or_default().visible,
                                    f.name(),
                                )
                            })
                            .body(|ui| features(ui, path, &f.base().room_features, states));
                        }
                    }
                    let mut path = vec![];
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        features(ui, &mut path, &rma.room_features, &mut states);
                    });
                });
                panel_width = gui_context.used_rect().width();
            },
        );

        let viewport = Viewport {
            x: (panel_width * frame_input.device_pixel_ratio) as i32,
            y: 0,
            width: frame_input.viewport.width
                - (panel_width * frame_input.device_pixel_ratio) as u32,
            height: frame_input.viewport.height,
        };

        camera.set_viewport(viewport);
        control.handle_events(&mut camera, &mut frame_input.events);

        frame_input
            .screen()
            .clear(ClearState::color_and_depth(0.0, 0.0, 0.0, 1.0, 1.0))
            .render(
                &camera,
                axes.into_iter()
                    .chain(primitives.iter().flat_map(|(path, p)| {
                        states
                            .get(path)
                            .and_then(|state: &State| {
                                state
                                    .visible
                                    .then(|| -> Box<dyn Iterator<Item = &dyn Object>> {
                                        Box::new(p.iter().map(|o| o.deref()))
                                    })
                            })
                            .unwrap_or_else(|| Box::new(std::iter::empty()))
                    })),
                &[&light0, &light1],
            )
            .write(|| gui.render());

        FrameOutput::default()
    });

    Ok(())
}

impl From<FVector> for Vector3<f32> {
    fn from(val: FVector) -> Self {
        vec3(val.x, val.y, val.z)
    }
}

fn line_transform(p1: Vector3<f32>, p2: Vector3<f32>) -> Mat4 {
    Mat4::from_translation(p1)
        * Into::<Mat4>::into(Quat::from_arc(
            vec3(1.0, 0.0, 0.0),
            (p2 - p1).normalize(),
            None,
        ))
        * Mat4::from_nonuniform_scale((p1 - p2).magnitude(), 1.0, 1.0)
}

fn flood_fill_box(ctx: &RMAContext, box_: &FloodFillBox) -> Vec<Box<dyn Object>> {
    // only used in RMA_Escort10
    let mut mesh = BoundingBox::new(ctx.context, CpuMesh::cube().compute_aabb());
    mesh.set_transformation(
        Mat4::from_translation(box_.position.into())
            * Mat4::from_nonuniform_scale(box_.extends.x, box_.extends.y, box_.extends.z),
    );

    vec![Box::new(Gm::new(mesh, ctx.wireframe_material.clone()))]
}

fn flood_fill_pillar(ctx: &RMAContext, line: &FloodFillPillar) -> Vec<Box<dyn Object>> {
    let mut transformations = Vec::new();

    let mut add_line = |p1, p2| transformations.push(line_transform(p1, p2));

    for pair in line.points.windows(2) {
        add_line(pair[0].location.into(), pair[1].location.into());
    }

    vec![Box::new(Gm::new(
        InstancedMesh::new(
            ctx.context,
            &Instances {
                transformations,
                ..Default::default()
            },
            &ctx.wireframe_mesh,
        ),
        ctx.wireframe_material.clone(),
    ))]
}

fn spawn_actor_feature(ctx: &RMAContext, spawn: &SpawnActorFeature) -> Vec<Box<dyn Object>> {
    let mut obj = Gm::new(
        Mesh::new(ctx.context, &CpuMesh::cone(16)),
        PhysicalMaterial::new_opaque(
            ctx.context,
            &CpuMaterial {
                albedo: Srgba {
                    r: 255,
                    g: 200,
                    b: 0,
                    a: 200,
                },
                ..Default::default()
            },
        ),
    );
    obj.set_transformation(
        Mat4::from_translation(spawn.location.into())
            * Mat4::from_nonuniform_scale(100.0, 100.0, 300.0)
            * Mat4::from_angle_y(-Radians::turn_div_4()),
    );
    vec![Box::new(obj)]
}

fn flood_fill_line(ctx: &RMAContext, line: &FloodFillLine) -> Vec<Box<dyn Object>> {
    let mut transformations = Vec::new();

    let mut add_line = |p1, p2| transformations.push(line_transform(p1, p2));

    for pair in line.points.windows(2) {
        let (p1, p2) = (&pair[0], &pair[1]);
        let v1: Vector3<f32> = p1.location.into();
        let v2: Vector3<f32> = p2.location.into();
        //add_line(v1, v2);

        let d = v1.truncate() - v2.truncate();
        let d = d / d.magnitude();
        let d = vec2(-d.y, d.x);

        let o1 = (p1.h_range * d).extend(0.);
        let o2 = (p2.h_range * d).extend(0.);
        add_line(v1 + o1, v2 + o2);
        add_line(v1 - o1, v2 - o2);
        add_line(v1 + vec3(0., 0., p1.v_range), v2 + vec3(0., 0., p2.v_range));
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

    vec![Box::new(Gm::new(
        InstancedMesh::new(
            ctx.context,
            &Instances {
                transformations,
                ..Default::default()
            },
            &ctx.wireframe_mesh,
        ),
        ctx.wireframe_material.clone(),
    ))]
}

fn entrance_feature(ctx: &RMAContext, entrance: &EntranceFeature) -> Vec<Box<dyn Object>> {
    let albedo = match entrance.entrance_type {
        rma::ECaveEntranceType::EntranceAndExit => Srgba {
            r: 0,
            g: 255,
            b: 255,
            a: 200,
        },
        rma::ECaveEntranceType::Entrance => Srgba {
            r: 255,
            g: 100,
            b: 0,
            a: 200,
        },
        rma::ECaveEntranceType::Exit => Srgba {
            r: 255,
            g: 0,
            b: 100,
            a: 200,
        },
        rma::ECaveEntranceType::TreassureRoom => Srgba {
            r: 255,
            g: 200,
            b: 0,
            a: 200,
        },
    };
    let mut sphere = Gm::new(
        Mesh::new(ctx.context, &CpuMesh::sphere(16)),
        PhysicalMaterial::new_opaque(
            ctx.context,
            &CpuMaterial {
                albedo,
                ..Default::default()
            },
        ),
    );
    // TODO there's also a direction component but I can't be bothered to figure out how it's
    // mapped at this moment
    sphere.set_transformation(
        Mat4::from_translation(entrance.location.into()) * Mat4::from_scale(100.0),
    );
    vec![Box::new(sphere)]
}

fn drop_pod_calldown_location_feature(
    ctx: &RMAContext,
    entrance: &DropPodCalldownLocationFeature,
) -> Vec<Box<dyn Object>> {
    let mut sphere = Gm::new(
        Mesh::new(ctx.context, &CpuMesh::cylinder(16)),
        PhysicalMaterial::new_opaque(
            ctx.context,
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
    sphere.set_transformation(
        Mat4::from_translation(entrance.location.into())
            * Mat4::from_nonuniform_scale(100.0, 100.0, 300.0)
            * Mat4::from_angle_y(Radians::turn_div_4()),
    );
    vec![Box::new(sphere)]
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
