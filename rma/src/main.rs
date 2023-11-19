#[cfg(target_arch = "wasm32")]
use crate as rma;

use anyhow::Result;
use log::info;
use rma::read_rma;
use rma::AppMode;
use three_d::*;
use unreal_asset::engine_version::EngineVersion;
use unreal_asset::Asset;

use std::collections::HashMap;
use std::io::Cursor;
use std::ops::Deref;
use std::sync::mpsc;

use rma::rma::RoomFeature;
use rma::rma::RoomGenerator;
use rma::room_features::RoomFeatureTrait;
use rma::RMAContext;

// Entry point for non-wasm
#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("expected path to an RMA .uasset");

    run(AppMode::Editor { path })
}

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

fn build_primitives(
    ctx: &RMAContext,
    rma: &RoomGenerator,
) -> HashMap<Vec<usize>, Vec<Box<dyn Object>>> {
    let mut primitives = HashMap::new();
    let mut path = vec![];
    iter_features(&rma.room_features, &mut path, &mut |f, path| match f {
        RoomFeature::FloodFillBox(f) => {
            primitives.insert(path.to_vec(), RoomFeatureTrait::build(f, ctx));
        }
        RoomFeature::FloodFillPillar(f) => {
            primitives.insert(path.to_vec(), RoomFeatureTrait::build(f, ctx));
        }
        RoomFeature::SpawnActorFeature(f) => {
            primitives.insert(path.to_vec(), RoomFeatureTrait::build(f, ctx));
        }
        RoomFeature::FloodFillLine(f) => {
            primitives.insert(path.to_vec(), RoomFeatureTrait::build(f, ctx));
        }
        RoomFeature::EntranceFeature(f) => {
            primitives.insert(path.to_vec(), RoomFeatureTrait::build(f, ctx));
        }
        RoomFeature::DropPodCalldownLocationFeature(f) => {
            primitives.insert(path.to_vec(), RoomFeatureTrait::build(f, ctx));
        }
        _ => {}
    });
    primitives
}

pub fn run(mode: AppMode) -> Result<()> {
    let mut rma = match &mode {
        AppMode::Editor { path } => {
            use rma::read_asset;

            let asset = read_asset(path, EngineVersion::VER_UE4_27)?;
            Some(read_rma(asset)?)
        }
        AppMode::Gallery { paths: _ } => None,
    };

    use futures::task::LocalSpawnExt;

    let mut ex = futures::executor::LocalPool::new();
    let spawner = ex.spawner();

    let window = Window::new(WindowSettings {
        title: "RMA Editor".to_string(),
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
        wireframe_material: wireframe_material.clone(),
        wireframe_mesh: wireframe_mesh.clone(),
    };

    let mut primitives = rma.as_ref().map(|rma| build_primitives(&rma_ctx, rma));

    let axes = Axes::new(&context, 10., 200.0);

    let light0 = DirectionalLight::new(&context, 1.0, Srgba::WHITE, &vec3(0.0, -0.5, -0.5));
    let light1 = DirectionalLight::new(&context, 1.0, Srgba::WHITE, &vec3(0.0, 0.5, 0.5));

    struct State {
        visible: bool,
    }
    impl Default for State {
        fn default() -> Self {
            Self { visible: true }
        }
    }

    let mut gui = three_d::GUI::new(&context);
    let mut states = HashMap::<Vec<usize>, State>::new();
    let mut selected_room = None;
    let (tx, rx) = mpsc::channel();

    let mut task_handles = vec![];

    window.render_loop(move |mut frame_input| {
        ex.run_until_stalled();

        if let Ok(new_rma) = rx.try_recv() {
            rma = Some(new_rma);
            states.clear();
            primitives = rma.as_ref().map(|rma| build_primitives(&RMAContext {
                context: &context,
                wireframe_material: wireframe_material.clone(),
                wireframe_mesh: wireframe_mesh.clone(),
            }, rma));
        }

        let panel_width = 300.0;

        gui.update(
            &mut frame_input.events,
            frame_input.accumulated_time,
            frame_input.viewport,
            frame_input.device_pixel_ratio,
            |gui_context| {
                use three_d::egui::*;
                SidePanel::left("side_panel")
                    .resizable(false)
                    .min_width(panel_width)
                    .max_width(panel_width)
                    .show(gui_context, |ui| {
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

                        let rooms = match &mode {
                            AppMode::Gallery { paths } => Some(paths),
                            AppMode::Editor { .. } => None,
                        };

                        let strip = egui_extras::StripBuilder::new(ui);
                        let strip = if rooms.is_some() {
                            strip.size(egui_extras::Size::relative(0.5))
                            .size(egui_extras::Size::relative(0.5))
                        } else {
                            strip.size(egui_extras::Size::relative(1.))
                        };
                        strip.vertical(|mut strip| {
                            if let Some(rooms) = rooms {
                                strip.cell(|ui| {
                                    ui.push_id("rooms", |ui| {
                                        ui.group(|ui| {
                                            ui.heading("Rooms");
                                            egui::ScrollArea::vertical().show(ui, |ui| {
                                                ui.with_layout(Layout::top_down_justified(Align::LEFT), |ui| {
                                                    for room in rooms {
                                                        let selected = selected_room.as_ref() == Some(room);
                                                        if ui.selectable_label(selected, room).clicked() {
                                                            selected_room = Some(room.to_string());
                                                            info!("{:?}", selected_room);

                                                            let name = room.to_string();
                                                            let tx = tx.clone();
                                                            let task = spawner.spawn_local(async move {
                                                                let uasset = three_d_asset::io::load_async(&[format!("rma/{name}.uasset")])
                                                                    .await
                                                                    .unwrap();
                                                                let uexp = three_d_asset::io::load_async(&[format!("rma/{name}.uexp")])
                                                                    .await
                                                                    .unwrap();

                                                                let version = EngineVersion::VER_UE4_27;
                                                                let uasset = Cursor::new(uasset.get("").unwrap());
                                                                let uexp = Cursor::new(uexp.get("").unwrap());
                                                                let asset = Asset::new(uasset, Some(uexp), version, None).unwrap();

                                                                let rma = read_rma(asset).unwrap();

                                                                info!("{rma:?}");
                                                                tx.send(rma).unwrap();
                                                            });
                                                            task_handles.push(task);
                                                        }
                                                    }
                                                ui.allocate_space(ui.available_size());
                                                });
                                            });
                                        });
                                    });
                                });
                            }
                                strip.cell(|ui| {
                                    ui.push_id("features", |ui| {
                                        ui.group(|ui| {
                                            ui.heading("Room Features");
                                            egui::ScrollArea::vertical().show(ui, |ui| {
                                                if let Some(rma) = &rma {
                                                    let mut path = vec![];
                                                    features(
                                                        ui,
                                                        &mut path,
                                                        &rma.room_features,
                                                        &mut states,
                                                    );
                                                }
                                                ui.allocate_space(ui.available_size());
                                            });
                                        });
                                    });
                                });
                            });
                    });
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

        #[cfg(target_arch = "wasm32")]
        for event in &mut frame_input.events {
            if let Event::MouseWheel {
                ref mut delta,
                handled,
                ..
            } = event
            {
                if !*handled {
                    // artificially decrease zoom delta
                    // https://github.com/asny/three-d/issues/403
                    delta.1 /= 5.;
                }
            }
        }
        control.handle_events(&mut camera, &mut frame_input.events);

        frame_input
            .screen()
            .clear(ClearState::color_and_depth(0.0, 0.0, 0.0, 1.0, 1.0))
            .render(
                &camera,
                axes.into_iter()
                    .chain(primitives.iter().flatten().flat_map(|(path, p)| {
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

#[cfg(test)]
mod test {
    use std::ffi::OsStr;

    use anyhow::Context;
    use rma::read_asset;

    use super::*;

    #[test]
    fn test_read_all() -> Result<()> {
        for path in std::fs::read_dir("../assets/rma")? {
            let path = path?.path();
            if path.extension() == Some(OsStr::new("uasset")) {
                println!("{:?}", path.display());
                let asset = read_asset(&path, EngineVersion::VER_UE4_27)?;
                let _rma = read_rma(asset)
                    .with_context(|| format!("parsing asset {:?}", path.display()))?;
            }
        }

        Ok(())
    }
}
