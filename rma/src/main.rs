#[cfg(target_arch = "wasm32")]
use crate as rma;

use anyhow::Result;
use log::info;
use rma::AppMode;
use rma::rma::FQuat;
use rma::rma::FTransform;
use rma::rma::FVector;
use rma::rma::RoomFeature;
use rma::room_features::Gizmos;
use rma::room_features::build_feature;
use rma::room_features::build_grid_planes;
use rma::room_features::compute_room_bounds;
use three_d::*;
use transform_gizmo_egui::Gizmo;
use transform_gizmo_egui::GizmoConfig;
use transform_gizmo_egui::GizmoOrientation;
use transform_gizmo_egui::GizmoResult;

use asset_ser::core::object_pool::{ObjectHandle, ObjectPool};

use std::collections::HashMap;
use std::ops::Deref;
use std::path::Path;
use std::sync::mpsc;

use rma::RMAContext;

// Entry point for non-wasm
#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("expected path to an RMA .uasset");

    run(AppMode::Editor { path })
}

fn iter_features<F, T>(
    pool: &ObjectPool,
    features: &[RoomFeature],
    path: &mut Vec<usize>,
    f: &mut F,
) where
    F: FnMut(&RoomFeature, &[usize]) -> T,
{
    path.push(0);
    for (i, feat) in features.iter().enumerate() {
        *path.last_mut().unwrap() = i;
        f(feat, path);
        let children = feat.get_child_features(pool);
        iter_features(pool, &children, path, f);
    }
    path.pop();
}

fn build_primitives(
    ctx: &RMAContext,
    pool: &ObjectPool,
    root_handle: ObjectHandle,
) -> HashMap<Vec<usize>, Vec<Box<dyn Object>>> {
    let mut primitives = HashMap::new();
    let features = rma::rma::load_room_features(pool, root_handle);
    let mut path = vec![];

    iter_features(pool, &features, &mut path, &mut |f, path| {
        let objs = build_feature(pool, f.handle(), f, ctx, None);
        if !objs.is_empty() {
            primitives.insert(path.to_vec(), objs);
        }
    });
    primitives
}

struct State {
    visible: bool,
}
impl Default for State {
    fn default() -> Self {
        Self { visible: true }
    }
}

#[allow(clippy::type_complexity)]
struct App {
    panel_width: f32,
    mode: AppMode,
    selected_room: Option<String>,
    selected_feature: Vec<usize>,
    hovered_feature: Option<Vec<usize>>,
    prev_hovered_feature: Option<Vec<usize>>,
    highlighted_primitive: Option<Vec<Box<dyn Object>>>,
    prev_selected_feature: Vec<usize>,
    selected_primitive: Option<Vec<Box<dyn Object>>>,
    _tx: std::sync::mpsc::Sender<(ObjectPool, ObjectHandle)>,
    _spawner: futures::executor::LocalSpawner,
    _task_handles: Vec<Result<(), futures::task::SpawnError>>,
    states: HashMap<Vec<usize>, State>,
    context: three_d::core::Context,
    wireframe_material: PhysicalMaterial,
    wireframe_mesh: CpuMesh,
    primitives: Option<HashMap<Vec<usize>, Vec<Box<dyn Object>>>>,
    grid_objects: Vec<Box<dyn Object>>,
    camera: Camera,
    gizmos: Vec<Gizmo>,
}

pub fn run(mode: AppMode) -> Result<()> {
    let (mut pool, root_handle) = match &mode {
        AppMode::Editor { path } => {
            let mut pool = ObjectPool::new();
            let handle = rma::load_rma_asset(Path::new(path), &mut pool)?;
            (Some(pool), Some(handle))
        }
        AppMode::Gallery { paths: _ } => (None, None),
    };

    let mut ex = futures::executor::LocalPool::new();

    let window = Window::new(WindowSettings {
        title: "RMA Editor".to_string(),
        ..Default::default()
    })
    .unwrap();
    let context = window.gl();

    let camera = Camera::new_perspective(
        window.viewport(),
        vec3(5000.0, 0.0, 2.5),
        vec3(0.0, 0.0, 0.0),
        vec3(0.0, 0.0, 1.0),
        degrees(45.0),
        1.0,
        1000000.0,
    );
    let mut control = OrbitControl::new(camera.target(), 1.0, 1000000.0);

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
        .transform(Mat4::from_nonuniform_scale(1.0, 10.0, 10.0))
        .unwrap();

    let rma_ctx = RMAContext {
        context: &context,
        wireframe_material: wireframe_material.clone(),
        wireframe_mesh: wireframe_mesh.clone(),
    };

    let axes = Axes::new(&context, 10., 200.0);

    let light0 = DirectionalLight::new(&context, 1.0, Srgba::WHITE, vec3(0.0, -0.5, -0.5));
    let light1 = DirectionalLight::new(&context, 1.0, Srgba::WHITE, vec3(0.0, 0.5, 0.5));

    let mut gui = three_d::GUI::new(&context);
    let (tx, rx) = mpsc::channel();

    let primitives = match (&pool, root_handle) {
        (Some(p), Some(h)) => Some(build_primitives(&rma_ctx, p, h)),
        _ => None,
    };

    let grid_objects = match (&pool, root_handle) {
        (Some(p), Some(h)) => {
            let bounds = compute_room_bounds(p, h);
            build_grid_planes(&rma_ctx, &bounds)
        }
        _ => Vec::new(),
    };

    let mut app = App {
        panel_width: 400.0,
        primitives,
        grid_objects,
        mode,
        selected_room: None,
        selected_feature: vec![],
        hovered_feature: None,
        prev_hovered_feature: None,
        highlighted_primitive: None,
        prev_selected_feature: vec![],
        selected_primitive: None,
        _tx: tx,
        _spawner: ex.spawner(),
        _task_handles: vec![],
        states: HashMap::new(),
        context,
        wireframe_material,
        wireframe_mesh,
        camera,
        gizmos: vec![],
    };

    window.render_loop(move |mut frame_input| {
        ex.run_until_stalled();

        if let Ok((new_pool, new_handle)) = rx.try_recv() {
            pool = Some(new_pool);
            app.states.clear();
            let ctx = RMAContext {
                context: &app.context,
                wireframe_material: app.wireframe_material.clone(),
                wireframe_mesh: app.wireframe_mesh.clone(),
            };
            app.primitives = pool.as_ref().map(|p| build_primitives(&ctx, p, new_handle));
            app.grid_objects = pool
                .as_ref()
                .map(|p| {
                    let bounds = compute_room_bounds(p, new_handle);
                    build_grid_planes(&ctx, &bounds)
                })
                .unwrap_or_default();
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

                    draw_panel(
                        gui_context,
                        &mut app,
                        &mut pool,
                        root_handle,
                        &mut changed,
                        &mut gizmos,
                    );

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
                    app.primitives = pool.as_ref().and_then(|p| {
                        root_handle.map(|h| {
                            build_primitives(
                                &RMAContext {
                                    context: &app.context,
                                    wireframe_material: app.wireframe_material.clone(),
                                    wireframe_mesh: app.wireframe_mesh.clone(),
                                },
                                p,
                                h,
                            )
                        })
                    });
                }
            },
        );

        if clear_events {
            frame_input.events.clear();
        }

        // Update highlighted primitive if hover changed
        if app.hovered_feature != app.prev_hovered_feature {
            app.prev_hovered_feature = app.hovered_feature.clone();
            app.highlighted_primitive = None;

            if let (Some(hovered_path), Some(p), Some(h)) =
                (&app.hovered_feature, &pool, root_handle)
            {
                // Navigate to the hovered feature
                let room_features = rma::rma::load_room_features(p, h);
                let mut path_iter = hovered_path.iter();
                if let Some(&first) = path_iter.next()
                    && let Some(feature) = room_features.get(first)
                {
                    let mut current = feature.clone();
                    for &idx in path_iter {
                        let children = current.get_child_features(p);
                        if let Some(child) = children.get(idx) {
                            current = child.clone();
                        }
                    }
                    // Build highlighted version with bright yellow color
                    let highlight_color = Srgba::new_opaque(255, 255, 100);
                    let ctx = RMAContext {
                        context: &app.context,
                        wireframe_material: app.wireframe_material.clone(),
                        wireframe_mesh: app.wireframe_mesh.clone(),
                    };
                    app.highlighted_primitive = Some(build_feature(
                        p,
                        current.handle(),
                        &current,
                        &ctx,
                        Some(highlight_color),
                    ));
                }
            }
        }

        // Update selected primitive if selection changed
        if app.selected_feature != app.prev_selected_feature {
            app.prev_selected_feature = app.selected_feature.clone();
            app.selected_primitive = None;

            if let (Some(p), Some(h)) = (&pool, root_handle)
                && !app.selected_feature.is_empty()
            {
                // Navigate to the selected feature
                let room_features = rma::rma::load_room_features(p, h);
                let mut path_iter = app.selected_feature.iter();
                if let Some(&first) = path_iter.next()
                    && let Some(feature) = room_features.get(first)
                {
                    let mut current = feature.clone();
                    for &idx in path_iter {
                        let children = current.get_child_features(p);
                        if let Some(child) = children.get(idx) {
                            current = child.clone();
                        }
                    }
                    // Build selected version with cyan color
                    let select_color = Srgba::new_opaque(100, 200, 255);
                    let ctx = RMAContext {
                        context: &app.context,
                        wireframe_material: app.wireframe_material.clone(),
                        wireframe_mesh: app.wireframe_mesh.clone(),
                    };
                    app.selected_primitive = Some(build_feature(
                        p,
                        current.handle(),
                        &current,
                        &ctx,
                        Some(select_color),
                    ));
                }
            }
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
                    .chain(app.grid_objects.iter().map(|o| o.deref()))
                    .chain(app.primitives.iter().flatten().flat_map(|(path, p)| {
                        // Skip hovered/selected primitives - we'll render special versions
                        let is_hovered = app.hovered_feature.as_ref() == Some(path);
                        let is_selected = &app.selected_feature == path;
                        app.states
                            .get(path)
                            .and_then(|state: &State| {
                                (state.visible && !is_hovered && !is_selected).then(
                                    || -> Box<dyn Iterator<Item = &dyn Object>> {
                                        Box::new(p.iter().map(|o| o.deref()))
                                    },
                                )
                            })
                            .unwrap_or_else(|| Box::new(std::iter::empty()))
                    }))
                    // Render selected primitive (cyan) - but not if it's also hovered
                    .chain({
                        let skip_selected =
                            app.hovered_feature.as_ref() == Some(&app.selected_feature);
                        app.selected_primitive
                            .iter()
                            .flatten()
                            .filter(move |_| !skip_selected)
                            .map(|o| o.deref())
                    })
                    // Render highlighted/hovered primitive (yellow) - always on top
                    .chain(
                        app.highlighted_primitive
                            .iter()
                            .flatten()
                            .map(|o| o.deref()),
                    ),
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
    pool: &'g mut Option<ObjectPool>,
    root_handle: Option<ObjectHandle>,
    changed: &mut bool,
    gizmos: &mut Gizmos<'g>,
) {
    use three_d::egui::*;
    SidePanel::left("side_panel")
        .resizable(false)
        .min_width(app.panel_width)
        .max_width(app.panel_width)
        .show(ctx, |ui| {
            // TODO: Implement save using asset_ser::saver::save_asset
            // if pool.is_some() && root_handle.is_some() && ui.button("save").clicked() {
            //     ui.label("Save not yet implemented with asset_ser");
            // }

            #[allow(clippy::too_many_arguments)]
            fn features(
                ui: &mut Ui,
                pool: &ObjectPool,
                path: &mut Vec<usize>,
                f: &[RoomFeature],
                states: &mut HashMap<Vec<usize>, State>,
                selected_feature: &mut Vec<usize>,
                deferred_select: &mut Vec<usize>,
                hovered_feature: &mut Option<Vec<usize>>,
            ) {
                path.push(0);
                for (i, f) in f.iter().enumerate() {
                    *path.last_mut().unwrap() = i;

                    let id = ui.make_persistent_id(i);
                    let mut is_hovered = false;
                    let collapsing_response =
                        egui::collapsing_header::CollapsingState::load_with_default_open(
                            ui.ctx(),
                            id,
                            true,
                        )
                        .show_header(ui, |ui| {
                            let checkbox = ui
                                .checkbox(&mut states.entry(path.clone()).or_default().visible, "");
                            if checkbox.hovered() {
                                is_hovered = true;
                            }
                            let mut checked = path == selected_feature;
                            let toggle = ui.toggle_value(&mut checked, f.name());
                            if toggle.hovered() {
                                is_hovered = true;
                            }
                            if toggle.changed() && checked {
                                deferred_select.clone_from(path);
                            }
                        })
                        .body(|ui| {
                            let children = f.get_child_features(pool);
                            features(
                                ui,
                                pool,
                                path,
                                &children,
                                states,
                                selected_feature,
                                deferred_select,
                                hovered_feature,
                            )
                        });

                    // Check if header or any element is hovered
                    if is_hovered || collapsing_response.0.hovered() {
                        *hovered_feature = Some(path.clone());
                    }
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

                                                // TODO: Load room with asset_ser
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
                                if let (Some(p), Some(h)) = (pool.as_ref(), root_handle) {
                                    let room_features = rma::rma::load_room_features(p, h);
                                    let mut path = vec![];
                                    app.hovered_feature = None; // Reset before checking
                                    features(
                                        ui,
                                        p,
                                        &mut path,
                                        &room_features,
                                        &mut app.states,
                                        &mut app.selected_feature,
                                        &mut deferred_select,
                                        &mut app.hovered_feature,
                                    );
                                }
                                ui.allocate_space(ui.available_size());
                            });
                        });
                    });
                });

                // Edit feature panel
                let mut path_iter = app.selected_feature.iter();
                if let Some(first) = path_iter.next() {
                    strip.cell(|ui| {
                        ui.push_id("edit feature", |ui| {
                            ui.group(|ui| {
                                ui.heading("Edit Feature");
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    if let (Some(p), Some(h)) = (pool.as_mut(), root_handle) {
                                        let room_features = rma::rma::load_room_features(p, h);
                                        if let Some(feature) = room_features.get(*first) {
                                            // Navigate to the selected feature
                                            let mut current = feature.clone();
                                            for &idx in path_iter {
                                                let children = current.get_child_features(p);
                                                if let Some(child) = children.get(idx) {
                                                    current = child.clone();
                                                }
                                            }
                                            *changed |= rma::room_features::edit_feature(
                                                p,
                                                current.handle(),
                                                &current,
                                                ui,
                                                gizmos,
                                            );
                                        }
                                    }
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
            ui.scope_builder(
                egui::UiBuilder::new().layer_id(egui::LayerId::background()),
                |ui| {
                    while app.gizmos.len() < gizmos.len() {
                        app.gizmos.push(Default::default());
                    }
                    while app.gizmos.len() > gizmos.len() {
                        app.gizmos.pop();
                    }

                    let mut already_interacted = false;

                    for ((modes, start, cb), gizmo) in gizmos.into_iter().zip(app.gizmos.iter_mut())
                    {
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
                            view_matrix: convert_mat4_to_mint(&app.camera.view()),
                            projection_matrix: convert_mat4_to_mint(&app.camera.projection()),
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

                            let new_transform = FTransform {
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
                            };
                            println!(
                                "Gizmo transform: translation=({}, {}, {}), scale=({}, {}, {})",
                                new_transform.translation.x,
                                new_transform.translation.y,
                                new_transform.translation.z,
                                new_transform.Scale3D.x,
                                new_transform.Scale3D.y,
                                new_transform.Scale3D.z
                            );
                            cb(new_transform);
                        }
                    }
                },
            )
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
                hovered: false, // TODO
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

#[cfg(test)]
mod test {
    use std::ffi::OsStr;

    use anyhow::Context;
    use asset_ser::core::object_pool::ObjectPool;
    use asset_ser::loader::asset_loader;

    use super::*;

    #[test]
    fn test_read_all() -> Result<()> {
        for path in std::fs::read_dir("../assets/rma")? {
            let path = path?.path();
            if path.extension() == Some(OsStr::new("uasset")) {
                println!("{:?}", path.display());
                let mut pool = ObjectPool::new();
                let _handle = asset_loader::load_asset(&path, &mut pool)
                    .with_context(|| format!("loading asset {:?}", path.display()))?;
                println!("Loaded {} objects", pool.len());
            }
        }

        Ok(())
    }
}
