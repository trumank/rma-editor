#[cfg(target_arch = "wasm32")]
use crate as rma;

use anyhow::Result;
use three_d::*;

use std::collections::HashMap;
use std::ops::Deref;

use rma::rma::RoomFeature;
use rma::rma::RoomGenerator;
use rma::room_features::RoomFeatureTrait;
use rma::RMAContext;

// Entry point for non-wasm
#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> Result<()> {
    use rma::{read_asset, read_rma};
    use unreal_asset::engine_version::EngineVersion;

    let path = std::env::args()
        .nth(1)
        .expect("expected path to an RMA .uasset");

    let asset = read_asset(path, EngineVersion::VER_UE4_27)?;
    let rma = read_rma(asset)?;

    run(rma).await
}

pub async fn run(rma: RoomGenerator) -> Result<()> {
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
            primitives.insert(path.to_vec(), RoomFeatureTrait::build(f, &rma_ctx));
        }
        RoomFeature::FloodFillPillar(f) => {
            primitives.insert(path.to_vec(), RoomFeatureTrait::build(f, &rma_ctx));
        }
        RoomFeature::SpawnActorFeature(f) => {
            primitives.insert(path.to_vec(), RoomFeatureTrait::build(f, &rma_ctx));
        }
        RoomFeature::FloodFillLine(f) => {
            primitives.insert(path.to_vec(), RoomFeatureTrait::build(f, &rma_ctx));
        }
        RoomFeature::EntranceFeature(f) => {
            primitives.insert(path.to_vec(), RoomFeatureTrait::build(f, &rma_ctx));
        }
        RoomFeature::DropPodCalldownLocationFeature(f) => {
            primitives.insert(path.to_vec(), RoomFeatureTrait::build(f, &rma_ctx));
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
