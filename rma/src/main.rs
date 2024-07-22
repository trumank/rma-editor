#[cfg(target_arch = "wasm32")]
use crate as rma;

use anyhow::Result;
use futures::task::LocalSpawnExt;
use log::info;
use rma::read_rma;
use rma::rma::FQuat;
use rma::rma::FTransform;
use rma::rma::FVector;
use rma::room_features::Gizmos;
use rma::AppMode;
use three_d::*;
use transform_gizmo_egui::Gizmo;
use transform_gizmo_egui::GizmoConfig;
use transform_gizmo_egui::GizmoOrientation;
use transform_gizmo_egui::GizmoResult;
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
    path.pop();
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

trait RoomFeatureExt {
    fn ui<'s>(&'s mut self, ui: &mut egui::Ui, gizmos: &mut Gizmos<'s>) -> bool;
    fn room_features_mut(&mut self) -> &mut Vec<RoomFeature>;
}

impl RoomFeatureExt for RoomFeature {
    fn ui<'s>(&'s mut self, ui: &mut egui::Ui, gizmos: &mut Gizmos<'s>) -> bool {
        match self {
            RoomFeature::FloodFillBox(f) => f.editor(ui, gizmos),
            RoomFeature::FloodFillPillar(f) => f.editor(ui, gizmos),
            RoomFeature::SpawnActorFeature(f) => f.editor(ui, gizmos),
            RoomFeature::FloodFillLine(f) => f.editor(ui, gizmos),
            RoomFeature::EntranceFeature(f) => f.editor(ui, gizmos),
            RoomFeature::DropPodCalldownLocationFeature(f) => f.editor(ui, gizmos),
            _ => todo!(),
        }
    }
    fn room_features_mut(&mut self) -> &mut Vec<RoomFeature> {
        match self {
            RoomFeature::FloodFillBox(f) => &mut f.base.room_features,
            RoomFeature::FloodFillPillar(f) => &mut f.base.room_features,
            RoomFeature::SpawnActorFeature(f) => &mut f.base.room_features,
            RoomFeature::FloodFillLine(f) => &mut f.base.room_features,
            RoomFeature::EntranceFeature(f) => &mut f.base.room_features,
            RoomFeature::DropPodCalldownLocationFeature(f) => &mut f.base.room_features,
            RoomFeature::FloodFillProceduralPillar => todo!(),
            RoomFeature::SpawnTriggerFeature(f) => &mut f.base.room_features,
            RoomFeature::RandomSelector(f) => &mut f.base.room_features,
            RoomFeature::RandomSubRoomFeature => todo!(),
            RoomFeature::ResourceFeature(f) => &mut f.base.room_features,
            RoomFeature::SubRoomFeature => todo!(),
        }
    }
}

struct State {
    visible: bool,
}
impl Default for State {
    fn default() -> Self {
        Self { visible: true }
    }
}

type RMA = Option<(RoomGenerator, Asset<Cursor<Vec<u8>>>)>;
struct App {
    panel_width: f32,
    //rma: Option<(RoomGenerator, Asset<Cursor<Vec<u8>>>)>,
    mode: AppMode,
    selected_room: Option<String>,
    selected_feature: Vec<usize>,
    tx: std::sync::mpsc::Sender<(RoomGenerator, Asset<Cursor<Vec<u8>>>)>,
    spawner: futures::executor::LocalSpawner,
    task_handles: Vec<Result<(), futures::task::SpawnError>>,
    states: HashMap<Vec<usize>, State>,
    context: three_d::core::Context,
    wireframe_material: PhysicalMaterial,
    wireframe_mesh: CpuMesh,
    primitives: Option<HashMap<Vec<usize>, Vec<Box<dyn Object>>>>,
    camera: Camera,
    gizmos: Vec<Gizmo>,
}

pub fn run(mode: AppMode) -> Result<()> {
    let mut rma = match &mode {
        AppMode::Editor { path } => {
            use rma::read_asset;

            let asset = read_asset(path, EngineVersion::VER_UE4_27)?;
            Some((read_rma(&asset)?, asset))
        }
        AppMode::Gallery { paths: _ } => None,
    };

    let mut ex = futures::executor::LocalPool::new();

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
    camera.mirror_in_xz_plane();

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

    let axes = Axes::new(&context, 10., 200.0);

    let light0 = DirectionalLight::new(&context, 1.0, Srgba::WHITE, &vec3(0.0, -0.5, -0.5));
    let light1 = DirectionalLight::new(&context, 1.0, Srgba::WHITE, &vec3(0.0, 0.5, 0.5));

    let mut gui = three_d::GUI::new(&context);
    let (tx, rx) = mpsc::channel();

    let mut app = App {
        panel_width: 400.0,
        primitives: rma.as_ref().map(|rma| build_primitives(&rma_ctx, &rma.0)),
        //rma,
        mode,
        selected_room: None,
        selected_feature: vec![],
        tx,
        spawner: ex.spawner(),
        task_handles: vec![],
        states: HashMap::new(),
        context,
        wireframe_material,
        wireframe_mesh,
        camera,
        gizmos: vec![],
    };

    window.render_loop(move |mut frame_input| {
        ex.run_until_stalled();

        if let Ok(new_rma) = rx.try_recv() {
            rma = Some(new_rma);
            app.states.clear();
            app.primitives = rma.as_ref().map(|rma| {
                build_primitives(
                    &RMAContext {
                        context: &app.context,
                        wireframe_material: app.wireframe_material.clone(),
                        wireframe_mesh: app.wireframe_mesh.clone(),
                    },
                    &rma.0,
                )
            });
        }

        let scaled_panel_width = app.panel_width * frame_input.device_pixel_ratio;

        let mut clear_events = false;

        let viewport = Viewport {
            x: scaled_panel_width as i32,
            y: 0,
            width: frame_input.viewport.width - scaled_panel_width as u32,
            height: frame_input.viewport.height,
        };

        app.camera.set_viewport(viewport);

        gui.update(
            &mut frame_input.events,
            frame_input.accumulated_time,
            frame_input.viewport,
            frame_input.device_pixel_ratio,
            |gui_context| {
                let mut changed = false;
                {
                    let mut gizmos = vec![];

                    draw_panel(gui_context, &mut app, &mut rma, &mut changed, &mut gizmos);

                    let viewport = egui::Rect::from_min_max(
                        (app.panel_width, 0.).into(),
                        egui::pos2(
                            frame_input.viewport.width as f32,
                            frame_input.viewport.height as f32,
                        ) / frame_input.device_pixel_ratio,
                    );
                    draw_gizmo(
                        gui_context,
                        viewport,
                        gizmos,
                        &mut clear_events,
                        &mut app,
                        &mut changed,
                    );
                }

                if changed {
                    //states.clear();
                    app.primitives = Some(build_primitives(
                        &RMAContext {
                            context: &app.context,
                            wireframe_material: app.wireframe_material.clone(),
                            wireframe_mesh: app.wireframe_mesh.clone(),
                        },
                        &rma.as_ref().unwrap().0,
                    ));
                }
            },
        );

        if clear_events {
            frame_input.events.clear();
        }

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
        control.handle_events(&mut app.camera, &mut frame_input.events);

        frame_input
            .screen()
            .clear(ClearState::color_and_depth(0.0, 0.0, 0.0, 1.0, 1.0))
            .render(
                &app.camera,
                axes.into_iter()
                    .chain(app.primitives.iter().flatten().flat_map(|(path, p)| {
                        app.states
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
            .write(|| gui.render())
            .unwrap();

        FrameOutput::default()
    });

    Ok(())
}

fn draw_panel<'g>(
    ctx: &egui::Context,
    app: &mut App,
    rma: &'g mut RMA,
    changed: &mut bool,
    gizmos: &mut Gizmos<'g>,
) {
    use three_d::egui::*;
    SidePanel::left("side_panel")
        .resizable(false)
        .min_width(app.panel_width)
        .max_width(app.panel_width)
        .show(ctx, |ui| {
            ui.heading("Debug Panel");
            if let Some(rma) = rma.as_mut() {
                if ui.button("save").clicked() {
                    save(&mut rma.1, &rma.0).unwrap();
                }
            }
            fn features(
                ui: &mut Ui,
                path: &mut Vec<usize>,
                f: &[RoomFeature],
                states: &mut HashMap<Vec<usize>, State>,
                selected_feature: &mut Vec<usize>,
                deferred_select: &mut Vec<usize>,
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
                        ui.checkbox(&mut states.entry(path.clone()).or_default().visible, "");
                        let mut checked = path == selected_feature;
                        //println!("{path:?} {selected_feature:?}");
                        if ui.toggle_value(&mut checked, f.name()).changed() && checked {
                            println!("{path:?} asdf");
                            deferred_select.clone_from(path);
                        }
                    })
                    .body(|ui| {
                        features(
                            ui,
                            path,
                            &f.base().room_features,
                            states,
                            selected_feature,
                            deferred_select,
                        )
                    });
                }
                path.pop();
            }

            let rooms = match &app.mode {
                AppMode::Gallery { paths } => Some(paths),
                AppMode::Editor { .. } => None,
            };

            let mut strip = egui_extras::StripBuilder::new(ui);
            let mut num_cells = 1;
            if rooms.is_some() {
                num_cells += 1;
            }
            if !app.selected_feature.is_empty() {
                num_cells += 1;
            }
            for _ in 0..num_cells {
                strip = strip.size(egui_extras::Size::relative(1. / num_cells as f32));
            }
            strip.vertical(|mut strip| {
                if let Some(rooms) = rooms {
                    strip.cell(|ui| {
                        ui.push_id("rooms", |ui| {
                            ui.group(|ui| {
                                ui.heading("Rooms");
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    ui.with_layout(Layout::top_down_justified(Align::LEFT), |ui| {
                                        for room in rooms {
                                            let selected = app.selected_room.as_ref() == Some(room);
                                            if ui.selectable_label(selected, room).clicked() {
                                                app.selected_room = Some(room.to_string());
                                                info!("{:?}", app.selected_room);

                                                let name = room.to_string();
                                                let tx = app.tx.clone();
                                                let task = app.spawner.spawn_local(async move {
                                                    let uasset =
                                                        three_d_asset::io::load_async(&[format!(
                                                            "rma/{name}.uasset"
                                                        )])
                                                        .await
                                                        .unwrap();
                                                    let uexp =
                                                        three_d_asset::io::load_async(&[format!(
                                                            "rma/{name}.uexp"
                                                        )])
                                                        .await
                                                        .unwrap();

                                                    let version = EngineVersion::VER_UE4_27;
                                                    let uasset = Cursor::new(
                                                        uasset.get("").unwrap().to_vec(),
                                                    );
                                                    let uexp =
                                                        Cursor::new(uexp.get("").unwrap().to_vec());
                                                    let asset = Asset::new(
                                                        uasset,
                                                        Some(uexp),
                                                        version,
                                                        None,
                                                        false,
                                                    )
                                                    .unwrap();

                                                    let rma = read_rma(&asset).unwrap();

                                                    info!("{rma:?}");
                                                    tx.send((rma, asset)).unwrap();
                                                });
                                                app.task_handles.push(task);
                                            }
                                        }
                                        ui.allocate_space(ui.available_size());
                                    });
                                });
                            });
                        });
                    });
                }
                let mut deferred_select = vec![];
                strip.cell(|ui| {
                    ui.push_id("features", |ui| {
                        ui.group(|ui| {
                            ui.heading("Room Features");
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                if let Some(rma) = rma.as_ref() {
                                    let mut path = vec![];
                                    features(
                                        ui,
                                        &mut path,
                                        &rma.0.room_features,
                                        &mut app.states,
                                        &mut app.selected_feature,
                                        &mut deferred_select,
                                    );
                                }
                                ui.allocate_space(ui.available_size());
                            });
                        });
                    });
                });
                let mut path_iter = app.selected_feature.iter();
                if let Some(first) = path_iter.next() {
                    strip.cell(|ui| {
                        ui.push_id("edit feature", |ui| {
                            ui.group(|ui| {
                                ui.heading("Edit Feature");
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    let Some(rma) = rma.as_mut() else { return };
                                    let mut feature = &mut rma.0.room_features[*first];
                                    for feature_index in path_iter {
                                        feature = &mut feature.room_features_mut()[*feature_index];
                                    }
                                    *changed |= feature.ui(ui, gizmos);

                                    ui.allocate_space(ui.available_size());
                                });
                            });
                        });
                    });
                }
                if !deferred_select.is_empty() {
                    app.selected_feature = deferred_select;
                }
            });
        });
}

fn draw_gizmo(
    ctx: &egui::Context,
    viewport: egui::Rect,
    gizmos: Gizmos,
    clear_events: &mut bool,
    app: &mut App,
    changed: &mut bool,
) {
    egui::Area::new("Viewport".into())
        .fixed_pos(viewport.min)
        .constrain_to(viewport)
        .show(ctx, |ui| {
            ui.with_layer_id(egui::LayerId::background(), |ui| {
                while app.gizmos.len() < gizmos.len() {
                    app.gizmos.push(Default::default());
                }
                while app.gizmos.len() > gizmos.len() {
                    app.gizmos.pop();
                }

                let mut already_interacted = false;

                for ((modes, start, cb), gizmo) in gizmos.into_iter().zip(app.gizmos.iter_mut()) {
                    pub fn convert_mat4_to_mint(
                        mat: &Mat4,
                    ) -> transform_gizmo_egui::mint::RowMatrix4<f64> {
                        #[rustfmt::skip]
                        let tab: [[f64; 4]; 4] = [
                            [mat.x.x as f64, mat.y.x as f64, mat.z.x as f64, mat.w.x as f64],
                            [mat.x.y as f64, mat.y.y as f64, mat.z.y as f64, mat.w.y as f64],
                            [mat.x.z as f64, mat.y.z as f64, mat.z.z as f64, mat.w.z as f64],
                            [mat.x.w as f64, mat.y.w as f64, mat.z.w as f64, mat.w.w as f64],
                        ];
                        transform_gizmo_egui::mint::RowMatrix4::from(tab)
                    }

                    // Fixed camera position
                    let snapping = ui.input(|input| input.modifiers.ctrl);

                    gizmo.update_config(GizmoConfig {
                        view_matrix: convert_mat4_to_mint(app.camera.view()),
                        projection_matrix: convert_mat4_to_mint(app.camera.projection()),
                        viewport,
                        modes,
                        orientation: GizmoOrientation::Local,
                        snapping,
                        ..Default::default()
                    });

                    let mut transform =
                        transform_gizmo_egui::math::Transform::from_scale_rotation_translation(
                            [
                                start.Scale3D.x.0 as f64,
                                start.Scale3D.y.0 as f64,
                                start.Scale3D.z.0 as f64,
                            ],
                            [
                                start.rotation.x.0 as f64,
                                start.rotation.y.0 as f64,
                                start.rotation.z.0 as f64,
                                start.rotation.w.0 as f64,
                            ],
                            [
                                start.translation.x.0 as f64,
                                start.translation.y.0 as f64,
                                start.translation.z.0 as f64,
                            ],
                        );

                    if let Some((_result, new_transforms)) =
                        gizmo.interact2(ui, &[transform], !already_interacted)
                    {
                        already_interacted = true;
                        *clear_events = true;

                        for (new_transform, transform) in
                            new_transforms.iter().zip(std::iter::once(&mut transform))
                        {
                            *transform = *new_transform;
                        }

                        *changed = true;

                        cb(FTransform {
                            translation: FVector {
                                x: (transform.translation.x as f32).into(),
                                y: (transform.translation.y as f32).into(),
                                z: (transform.translation.z as f32).into(),
                            },
                            rotation: FQuat {
                                x: (transform.rotation.v.x as f32).into(),
                                y: (transform.rotation.v.y as f32).into(),
                                z: (transform.rotation.v.z as f32).into(),
                                w: (transform.rotation.s as f32).into(),
                            },
                            Scale3D: FVector {
                                x: (transform.scale.x as f32).into(),
                                y: (transform.scale.y as f32).into(),
                                z: (transform.scale.z as f32).into(),
                            },
                        });
                    }
                }
            })
        });
}

pub trait GizmoExt2 {
    /// Version of Gizmo interact that can have input disabled
    /// needed to prevent overlapping gizmos from handling the same input
    fn interact2(
        &mut self,
        ui: &egui::Ui,
        targets: &[transform_gizmo_egui::math::Transform],
        enable: bool,
    ) -> Option<(GizmoResult, Vec<transform_gizmo_egui::math::Transform>)>;
}

impl GizmoExt2 for Gizmo {
    fn interact2(
        &mut self,
        ui: &egui::Ui,
        targets: &[transform_gizmo_egui::math::Transform],
        enable: bool,
    ) -> Option<(GizmoResult, Vec<transform_gizmo_egui::math::Transform>)> {
        let config = self.config();

        let egui_viewport = egui::Rect {
            min: egui::Pos2::new(config.viewport.min.x, config.viewport.min.y),
            max: egui::Pos2::new(config.viewport.max.x, config.viewport.max.y),
        };

        let cursor_pos = ui
            .input(|input| input.pointer.hover_pos())
            .unwrap_or_default();

        let mut viewport = self.config().viewport;
        if !viewport.is_finite() {
            viewport = ui.clip_rect();
        }

        self.update_config(GizmoConfig {
            viewport,
            pixels_per_point: ui.ctx().pixels_per_point(),
            ..*self.config()
        });

        let gizmo_result = self.update(
            transform_gizmo_egui::GizmoInteraction {
                cursor_pos: (cursor_pos.x, cursor_pos.y),
                drag_started: ui.input(|input| {
                    enable && input.pointer.button_pressed(egui::PointerButton::Primary)
                }),
                dragging: ui.input(|input| {
                    enable && input.pointer.button_down(egui::PointerButton::Primary)
                }),
            },
            targets,
        );

        let draw_data = self.draw();

        ui.painter()
            .with_clip_rect(egui_viewport)
            .add(egui::epaint::Mesh {
                indices: draw_data.indices,
                vertices: draw_data
                    .vertices
                    .into_iter()
                    .zip(draw_data.colors)
                    .map(|(pos, [r, g, b, a])| egui::epaint::Vertex {
                        pos: pos.into(),
                        uv: egui::Pos2::default(),
                        color: egui::Rgba::from_rgba_premultiplied(r, g, b, a).into(),
                    })
                    .collect(),
                ..Default::default()
            });

        gizmo_result
    }
}

fn save<C: std::io::Read + std::io::Seek>(asset: &mut Asset<C>, rma: &RoomGenerator) -> Result<()> {
    use rma_lib::{CtxSer, NameCounter, ToExport as _};
    use unreal_asset::{exports::ExportBaseTrait, types::PackageIndex};

    asset.asset_data.exports.clear();
    asset.imports.clear();

    let mut name_counter = NameCounter::default();
    let pi = dbg!(rma.to_export(&mut CtxSer::new(asset, &mut name_counter))?);

    let name = asset.add_fname("RMA_CarverA");
    asset
        .asset_data
        .exports
        .last_mut()
        .unwrap()
        .get_base_export_mut()
        .object_name = name;

    for (i, export) in asset.asset_data.exports.iter_mut().enumerate() {
        let i = PackageIndex::from_export(i as i32).unwrap();
        if i != pi {
            let base = export.get_base_export_mut();
            base.outer_index = pi;
            base.create_before_create_dependencies.push(pi);
        }
    }

    let new_path =
        std::path::Path::new("test-pak/FSD/Content/Maps/Rooms/RoomGenerators/RMA_CarverA.uasset");
    asset.write_data(
        &mut std::fs::File::create(new_path)?,
        Some(&mut std::fs::File::create(new_path.with_extension("uexp"))?),
    )?;
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
                let _rma = read_rma(&asset)
                    .with_context(|| format!("parsing asset {:?}", path.display()))?;
                println!("{_rma:?}");
            }
        }

        Ok(())
    }

    #[test]
    fn test_read_small() -> Result<()> {
        use std::fmt::Write;

        let path = std::path::Path::new("../assets/rma/RMA_2PValley.uasset");
        let mut asset_orig = read_asset(path, EngineVersion::VER_UE4_27)?;
        let asset = read_asset(path, EngineVersion::VER_UE4_27)?;

        let mut buf = String::new();
        writeln!(&mut buf, "{asset:#?}").unwrap();
        std::fs::write("../dbg_orig.txt", buf)?;

        dbg!(&asset.asset_data.exports);

        let _rma = read_rma(&asset)?;
        println!("{_rma:#?}");

        asset_stuff::asdf(&mut asset_orig, &_rma)?;

        let new_path = std::path::Path::new("../RMA_CarverA.uasset");
        dbg!(&asset_orig);

        let mut buf = String::new();
        writeln!(&mut buf, "{asset_orig:#?}").unwrap();
        std::fs::write("../dbg_new.txt", buf)?;

        asset_orig.write_data(
            &mut std::fs::File::create(new_path)?,
            Some(&mut std::fs::File::create(new_path.with_extension("uexp"))?),
        )?;

        //let rma_round_trip = read_rma(asset_orig)?;
        //assert_eq!(_rma, rma_round_trip);

        Ok(())
    }

    mod asset_stuff {
        use anyhow::Result;
        use rma::rma::RoomGenerator;
        use rma_lib::{NameCounter, ToExport as _};
        use std::io::{Read, Seek};

        use unreal_asset::{exports::ExportBaseTrait, types::PackageIndex, Asset};

        pub fn asdf<C: Read + Seek>(other: &mut Asset<C>, data: &RoomGenerator) -> Result<()> {
            other.asset_data.exports.clear();
            //other.imports.clear();

            let mut name_counter = NameCounter::default();
            let pi = dbg!(data.to_export(&mut rma_lib::CtxSer::new(other, &mut name_counter))?);

            let name = other.add_fname("RMA_CarverA");
            other
                .asset_data
                .exports
                .last_mut()
                .unwrap()
                .get_base_export_mut()
                .object_name = name;

            for (i, export) in other.asset_data.exports.iter_mut().enumerate() {
                let i = PackageIndex::from_export(i as i32).unwrap();
                if i != pi {
                    let base = export.get_base_export_mut();
                    base.outer_index = pi;
                    base.create_before_create_dependencies.push(pi);
                }
            }

            //other.asset_data.exports.pop();

            //*other.asset_data.exports.last_mut().unwrap() = export;
            //let last = other.asset_data.exports.last().unwrap();
            //pretty_assertions::assert_eq!(*last, export);

            //dbg!(&other.asset_data.exports.last().unwrap());
            Ok(())
        }
    }
}
