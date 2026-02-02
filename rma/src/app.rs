use anyhow::Result;
use log::info;
use rma::AppMode;
use rma::CameraMode;
use rma::RenderMode;
use rma::convert::load_room_generator;
use rma::objects::{FQuat, FTransform, FVector, URoomFeature, URoomGenerator};
use rma::scene::csg_mesh::{HasVisible, build_csg_from_features, csg_to_three_d_mesh};
use rma::scene::fly_control::FlyControl;
use rma::scene::room_features::Gizmos;
use rma::scene::room_features::build_feature;
use rma::scene::room_features::build_grid_planes;
use rma::scene::room_features::compute_room_bounds;
use rma::scene::room_features::feature_type_name;
use rma::scene::room_features::is_mesh_primitive;
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

fn iter_features<F, T>(features: &[URoomFeature], path: &mut Vec<usize>, f: &mut F)
where
    F: FnMut(&URoomFeature, &[usize]) -> T,
{
    path.push(0);
    for (i, feat) in features.iter().enumerate() {
        *path.last_mut().unwrap() = i;
        f(feat, path);
        iter_features(&feat.children, path, f);
    }
    path.pop();
}

fn build_primitives(
    ctx: &RMAContext,
    room: &URoomGenerator,
    selected: &[usize],
    hovered: Option<&[usize]>,
    render_mode: RenderMode,
) -> HashMap<Vec<usize>, Vec<Box<dyn Object>>> {
    let mut primitives = HashMap::new();
    let mut path = vec![];

    iter_features(&room.room_features, &mut path, &mut |f, path| {
        // In CSG mode, skip mesh primitives (they're rendered as a single CSG mesh)
        if render_mode == RenderMode::CsgMesh && is_mesh_primitive(&f.feature_type) {
            return;
        }

        // Determine highlight color based on selection/hover state
        let color = if hovered == Some(path) {
            Some(Srgba::new_opaque(255, 255, 100)) // Yellow for hover
        } else if selected == path {
            Some(Srgba::new_opaque(100, 200, 255)) // Cyan for selection
        } else {
            None
        };
        let objs = build_feature(f, ctx, color);
        if !objs.is_empty() {
            primitives.insert(path.to_vec(), objs);
        }
    });
    primitives
}

/// Build the CSG mesh for all visible mesh primitives in the room
fn build_csg_mesh_object(
    ctx: &RMAContext,
    room: &URoomGenerator,
    states: &HashMap<Vec<usize>, State>,
) -> Option<Box<dyn Object>> {
    let csg = build_csg_from_features(room, states)?;
    let cpu_mesh = csg_to_three_d_mesh(&csg);

    let mesh = Mesh::new(ctx.context, &cpu_mesh);
    let mut material = PhysicalMaterial::new_opaque(
        ctx.context,
        &CpuMaterial {
            albedo: Srgba {
                r: 150,
                g: 100,
                b: 80,
                a: 255,
            },
            roughness: 0.8,
            metallic: 0.0,
            ..Default::default()
        },
    );
    // One-sided mesh - cull front faces so cave interior is visible from outside
    material.render_states.cull = Cull::Front;

    Some(Box::new(Gm::new(mesh, material)))
}

struct State {
    visible: bool,
}
impl Default for State {
    fn default() -> Self {
        Self { visible: true }
    }
}
impl HasVisible for State {
    fn visible(&self) -> bool {
        self.visible
    }
}

#[allow(clippy::type_complexity)]
struct App {
    panel_width: f32,
    mode: AppMode,
    room: Option<URoomGenerator>,
    selected_room: Option<String>,
    selected_feature: Vec<usize>,
    prev_selected_feature: Vec<usize>,
    hovered_feature: Option<Vec<usize>>,
    prev_hovered_feature: Option<Vec<usize>>,
    _tx: std::sync::mpsc::Sender<(ObjectPool, ObjectHandle)>,
    _spawner: futures::executor::LocalSpawner,
    _task_handles: Vec<Result<(), futures::task::SpawnError>>,
    states: HashMap<Vec<usize>, State>,
    context: three_d::core::Context,
    wireframe_material: PhysicalMaterial,
    wireframe_mesh: CpuMesh,
    primitives: Option<HashMap<Vec<usize>, Vec<Box<dyn Object>>>>,
    csg_mesh: Option<Box<dyn Object>>,
    grid_objects: Vec<Box<dyn Object>>,
    camera: Camera,
    gizmos: Vec<Gizmo>,
    camera_mode: CameraMode,
    prev_camera_mode: CameraMode,
    render_mode: RenderMode,
    prev_render_mode: RenderMode,
    fly_control: FlyControl,
}

pub fn run(mode: AppMode) -> Result<()> {
    let room = match &mode {
        AppMode::Editor { path } => rma::load_room(Path::new(path)).ok(),
        AppMode::Gallery { paths: _ } => None,
    };

    let mut ex = futures::executor::LocalPool::new();

    let window = Window::new(WindowSettings {
        title: "RMA Editor".to_string(),
        ..Default::default()
    })
    .unwrap();
    let context = window.gl();

    let initial_camera_pos = vec3(5000.0, 0.0, 2.5);
    let initial_camera_target = vec3(0.0, 0.0, 0.0);
    let initial_camera_up = vec3(0.0, 0.0, 1.0);

    let camera = Camera::new_perspective(
        window.viewport(),
        initial_camera_pos,
        initial_camera_target,
        initial_camera_up,
        degrees(45.0),
        1.0,
        1000000.0,
    );
    let mut orbit_control = OrbitControl::new(camera.target(), 1.0, 1000000.0);
    let mut last_time = 0.0f64;

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

    let render_mode = RenderMode::default();
    let primitives = room
        .as_ref()
        .map(|r| build_primitives(&rma_ctx, r, &[], None, render_mode));

    let empty_states: HashMap<Vec<usize>, State> = HashMap::new();
    let csg_mesh = if render_mode == RenderMode::CsgMesh {
        room.as_ref()
            .and_then(|r| build_csg_mesh_object(&rma_ctx, r, &empty_states))
    } else {
        None
    };

    let grid_objects = room
        .as_ref()
        .map(|r| {
            let bounds = compute_room_bounds(r);
            build_grid_planes(&rma_ctx, &bounds)
        })
        .unwrap_or_default();

    let mut app = App {
        panel_width: 400.0,
        room,
        primitives,
        csg_mesh,
        grid_objects,
        mode,
        selected_room: None,
        selected_feature: vec![],
        prev_selected_feature: vec![],
        hovered_feature: None,
        prev_hovered_feature: None,
        _tx: tx,
        _spawner: ex.spawner(),
        _task_handles: vec![],
        states: HashMap::new(),
        context,
        wireframe_material,
        wireframe_mesh,
        camera,
        gizmos: vec![],
        camera_mode: CameraMode::default(),
        prev_camera_mode: CameraMode::default(),
        render_mode,
        prev_render_mode: render_mode,
        fly_control: FlyControl::default(),
    };

    window.render_loop(move |mut frame_input| {
        ex.run_until_stalled();

        if let Ok((new_pool, new_handle)) = rx.try_recv() {
            app.room = load_room_generator(&new_pool, new_handle).ok();
            app.states.clear();
            let ctx = RMAContext {
                context: &app.context,
                wireframe_material: app.wireframe_material.clone(),
                wireframe_mesh: app.wireframe_mesh.clone(),
            };
            app.primitives = app.room.as_ref().map(|r| {
                build_primitives(
                    &ctx,
                    r,
                    &app.selected_feature,
                    app.hovered_feature.as_deref(),
                    app.render_mode,
                )
            });
            app.csg_mesh = if app.render_mode == RenderMode::CsgMesh {
                app.room
                    .as_ref()
                    .and_then(|r| build_csg_mesh_object(&ctx, r, &app.states))
            } else {
                None
            };
            app.grid_objects = app
                .room
                .as_ref()
                .map(|r| {
                    let bounds = compute_room_bounds(r);
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

                    draw_panel(gui_context, &mut app, &mut changed, &mut gizmos);

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
                    let ctx = RMAContext {
                        context: &app.context,
                        wireframe_material: app.wireframe_material.clone(),
                        wireframe_mesh: app.wireframe_mesh.clone(),
                    };
                    app.primitives = app.room.as_ref().map(|r| {
                        build_primitives(
                            &ctx,
                            r,
                            &app.selected_feature,
                            app.hovered_feature.as_deref(),
                            app.render_mode,
                        )
                    });
                }
            },
        );

        if clear_events {
            frame_input.events.clear();
        }

        // Rebuild primitives when selection or hover changes
        let selection_changed = app.selected_feature != app.prev_selected_feature;
        let hover_changed = app.hovered_feature != app.prev_hovered_feature;
        if selection_changed || hover_changed {
            app.prev_selected_feature = app.selected_feature.clone();
            app.prev_hovered_feature = app.hovered_feature.clone();
            let ctx = RMAContext {
                context: &app.context,
                wireframe_material: app.wireframe_material.clone(),
                wireframe_mesh: app.wireframe_mesh.clone(),
            };
            app.primitives = app.room.as_ref().map(|r| {
                build_primitives(
                    &ctx,
                    r,
                    &app.selected_feature,
                    app.hovered_feature.as_deref(),
                    app.render_mode,
                )
            });
        }

        // Rebuild when render mode changes
        if app.render_mode != app.prev_render_mode {
            app.prev_render_mode = app.render_mode;
            let ctx = RMAContext {
                context: &app.context,
                wireframe_material: app.wireframe_material.clone(),
                wireframe_mesh: app.wireframe_mesh.clone(),
            };
            app.primitives = app.room.as_ref().map(|r| {
                build_primitives(
                    &ctx,
                    r,
                    &app.selected_feature,
                    app.hovered_feature.as_deref(),
                    app.render_mode,
                )
            });
            app.csg_mesh = if app.render_mode == RenderMode::CsgMesh {
                app.room
                    .as_ref()
                    .and_then(|r| build_csg_mesh_object(&ctx, r, &app.states))
            } else {
                None
            };
        }

        // Compute delta time
        let dt = (frame_input.accumulated_time - last_time) as f32;
        last_time = frame_input.accumulated_time;

        // Reset camera when switching back to orbit mode
        if app.camera_mode != app.prev_camera_mode {
            if app.camera_mode == CameraMode::Orbit {
                app.camera
                    .set_view(initial_camera_pos, initial_camera_target, initial_camera_up);
                orbit_control = OrbitControl::new(initial_camera_target, 1.0, 1000000.0);
            }
            app.prev_camera_mode = app.camera_mode;
        }

        // Handle camera events based on mode
        match app.camera_mode {
            CameraMode::Orbit => {
                orbit_control.handle_events(&mut app.camera, &mut frame_input.events);
            }
            CameraMode::Fly => {
                app.fly_control
                    .handle_events(&mut app.camera, &mut frame_input.events, dt);
            }
        }

        frame_input
            .screen()
            .clear(ClearState::color_and_depth(0.0, 0.0, 0.0, 1.0, 1.0))
            .render(
                &app.camera,
                axes.into_iter()
                    .chain(app.grid_objects.iter().map(|o| o.deref()))
                    .chain(app.csg_mesh.iter().map(|o| o.deref()))
                    .chain(app.primitives.iter().flatten().flat_map(|(path, p)| {
                        // Only render visible primitives
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

fn draw_panel<'g>(ctx: &egui::Context, app: &mut App, changed: &mut bool, gizmos: &mut Gizmos<'g>) {
    use three_d::egui::*;
    SidePanel::left("side_panel")
        .resizable(false)
        .min_width(app.panel_width)
        .max_width(app.panel_width)
        .show(ctx, |ui| {
            // Camera mode selector
            ui.horizontal(|ui| {
                ui.label("Camera:");
                ui.selectable_value(&mut app.camera_mode, CameraMode::Orbit, "Orbit");
                ui.selectable_value(&mut app.camera_mode, CameraMode::Fly, "Fly");
            });
            if app.camera_mode == CameraMode::Fly {
                ui.label(format!(
                    "Speed: {:.0} (scroll to adjust)",
                    app.fly_control.speed()
                ));
                ui.label("WASD: move, Q/E: down/up, RMB: look");
            }
            ui.separator();

            // Render mode selector
            ui.horizontal(|ui| {
                ui.label("Render:");
                ui.selectable_value(&mut app.render_mode, RenderMode::Wireframe, "Wireframe");
                ui.selectable_value(&mut app.render_mode, RenderMode::CsgMesh, "CSG Mesh");
            });
            ui.separator();

            if let AppMode::Editor { path } = &app.mode {
                if app.room.is_some() && ui.button("Save").clicked() {
                    if let Some(room) = &app.room {
                        if let Err(e) = rma::save_room(Path::new(path), room) {
                            eprintln!("Failed to save: {}", e);
                        } else {
                            eprintln!("Saved to {}", path);
                        }
                    }
                }
            }

            #[allow(clippy::too_many_arguments)]
            fn features(
                ui: &mut Ui,
                path: &mut Vec<usize>,
                f: &[URoomFeature],
                states: &mut HashMap<Vec<usize>, State>,
                selected_feature: &mut Vec<usize>,
                deferred_select: &mut Vec<usize>,
                hovered_feature: &mut Option<Vec<usize>>,
                visibility_changed: &mut bool,
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
                            if checkbox.changed() {
                                *visibility_changed = true;
                            }
                            if checkbox.hovered() {
                                is_hovered = true;
                            }
                            let mut checked = path == selected_feature;
                            let toggle =
                                ui.toggle_value(&mut checked, feature_type_name(&f.feature_type));
                            if toggle.hovered() {
                                is_hovered = true;
                            }
                            if toggle.changed() && checked {
                                deferred_select.clone_from(path);
                            }
                        })
                        .body(|ui| {
                            features(
                                ui,
                                path,
                                &f.children,
                                states,
                                selected_feature,
                                deferred_select,
                                hovered_feature,
                                visibility_changed,
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
                let mut visibility_changed = false;
                strip.cell(|ui| {
                    ui.push_id("features", |ui| {
                        ui.group(|ui| {
                            ui.heading("Room Features");
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                if let Some(room) = &app.room {
                                    let mut path = vec![];
                                    app.hovered_feature = None; // Reset before checking
                                    features(
                                        ui,
                                        &mut path,
                                        &room.room_features,
                                        &mut app.states,
                                        &mut app.selected_feature,
                                        &mut deferred_select,
                                        &mut app.hovered_feature,
                                        &mut visibility_changed,
                                    );
                                }
                                ui.allocate_space(ui.available_size());
                            });
                        });
                    });
                });

                // Rebuild CSG mesh when visibility changes in CSG mode
                if visibility_changed && app.render_mode == RenderMode::CsgMesh {
                    let ctx = RMAContext {
                        context: &app.context,
                        wireframe_material: app.wireframe_material.clone(),
                        wireframe_mesh: app.wireframe_mesh.clone(),
                    };
                    app.csg_mesh = app
                        .room
                        .as_ref()
                        .and_then(|r| build_csg_mesh_object(&ctx, r, &app.states));
                }

                // Edit feature panel
                if !app.selected_feature.is_empty() {
                    strip.cell(|ui| {
                        ui.push_id("edit feature", |ui| {
                            ui.group(|ui| {
                                ui.heading("Edit Feature");
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    // Helper to navigate to feature by path (mutable)
                                    fn get_feature_by_path_mut<'a>(
                                        room: &'a mut URoomGenerator,
                                        path: &[usize],
                                    ) -> Option<&'a mut URoomFeature>
                                    {
                                        let mut path_iter = path.iter();
                                        let first = *path_iter.next()?;
                                        let mut current = room.room_features.get_mut(first)?;
                                        for &idx in path_iter {
                                            current = current.children.get_mut(idx)?;
                                        }
                                        Some(current)
                                    }

                                    if let Some(room) = &mut app.room
                                        && let Some(feature) =
                                            get_feature_by_path_mut(room, &app.selected_feature)
                                    {
                                        *changed |= rma::scene::room_features::edit_feature(
                                            feature, ui, gizmos,
                                        );
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
                                    start.Scale3D.x as f64,
                                    start.Scale3D.y as f64,
                                    start.Scale3D.z as f64,
                                ],
                                [
                                    start.rotation.x as f64,
                                    start.rotation.y as f64,
                                    start.rotation.z as f64,
                                    start.rotation.w as f64,
                                ],
                                [
                                    start.translation.x as f64,
                                    start.translation.y as f64,
                                    start.translation.z as f64,
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
                                    x: transform.translation.x as f32,
                                    y: transform.translation.y as f32,
                                    z: transform.translation.z as f32,
                                },
                                rotation: FQuat {
                                    x: transform.rotation.v.x as f32,
                                    y: transform.rotation.v.y as f32,
                                    z: transform.rotation.v.z as f32,
                                    w: transform.rotation.s as f32,
                                },
                                Scale3D: FVector {
                                    x: transform.scale.x as f32,
                                    y: transform.scale.y as f32,
                                    z: transform.scale.z as f32,
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
