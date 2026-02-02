//! Room feature visualization and editing UI
//!
//! This module provides 3D visualization and egui editors for RMA room features.

use three_d::{
    egui, vec2, vec3, Angle, BoundingBox, CpuMaterial, CpuMesh, Gm, InnerSpace, InstancedMesh,
    Instances, Mat4, Mesh, Object, PhysicalMaterial, Quat, Radians, Srgba, Vector3,
};
use transform_gizmo_egui::GizmoMode;

use asset_ser::core::object_pool::{ObjectHandle, ObjectPool};

use crate::{
    rma::{
        FTransform, FVector, RoomFeature, TypedProperties, UDropPodCalldownLocationFeature,
        UEntranceFeature, UFloodFillBox, UFloodFillLine, UFloodFillPillar, USpawnActorFeature,
    },
    RMAContext,
};

trait ChangedTrait {
    fn c(&self, changed: &mut bool);
}
impl ChangedTrait for egui::Response {
    fn c(&self, changed: &mut bool) {
        if self.changed() {
            *changed = true;
        }
    }
}
impl ChangedTrait for bool {
    fn c(&self, changed: &mut bool) {
        if *self {
            *changed = true;
        }
    }
}

pub type Gizmos<'s> = Vec<(
    enumset::EnumSet<GizmoMode>,
    FTransform,
    Box<dyn FnOnce(FTransform) + 's>,
)>;

impl From<FVector> for Vector3<f32> {
    fn from(val: FVector) -> Self {
        vec3(*val.x, *val.y, *val.z)
    }
}

pub fn line_transform(p1: Vector3<f32>, p2: Vector3<f32>) -> Mat4 {
    Mat4::from_translation(p1)
        * Into::<Mat4>::into(Quat::from_arc(
            vec3(1.0, 0.0, 0.0),
            (p2 - p1).normalize(),
            None,
        ))
        * Mat4::from_nonuniform_scale((p1 - p2).magnitude(), 1.0, 1.0)
}

/// Build 3D visualization for a room feature
pub fn build_feature(
    pool: &ObjectPool,
    handle: ObjectHandle,
    feature: &RoomFeature,
    ctx: &RMAContext,
) -> Vec<Box<dyn Object>> {
    let obj = pool.get(handle).expect("Invalid handle");
    let props = obj.properties();

    match feature {
        RoomFeature::FloodFillBox(_) => {
            let typed = UFloodFillBox::from_properties(props).unwrap();
            let position = typed.position();
            let extends = typed.extends();

            let mut mesh = BoundingBox::new(ctx.context, CpuMesh::cube().compute_aabb());
            mesh.set_transformation(
                Mat4::from_translation(position.into())
                    * Mat4::from_nonuniform_scale(*extends.x, *extends.y, *extends.z),
            );
            vec![Box::new(Gm::new(mesh, ctx.wireframe_material.clone()))]
        }

        RoomFeature::FloodFillPillar(_) => {
            let typed = UFloodFillPillar::from_properties(props).unwrap();
            let mut transformations = Vec::new();

            let points: Vec<FVector> = typed.points().iter().map(|p| p.location()).collect();

            for pair in points.windows(2) {
                transformations.push(line_transform(pair[0].into(), pair[1].into()));
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

        RoomFeature::FloodFillLine(_) => {
            let typed = UFloodFillLine::from_properties(props).unwrap();
            let mut transformations = Vec::new();

            let mut add_line = |p1, p2| transformations.push(line_transform(p1, p2));

            // Collect points data
            let points_data: Vec<_> = typed
                .points()
                .iter()
                .map(|p| (p.location(), p.h_range(), p.v_range()))
                .collect();

            for pair in points_data.windows(2) {
                let (loc1, h1, v1) = pair[0];
                let (loc2, h2, v2) = pair[1];
                let v1: Vector3<f32> = loc1.into();
                let v2: Vector3<f32> = loc2.into();

                let d = v1.truncate() - v2.truncate();
                let d = d / d.magnitude();
                let d = vec2(-d.y, d.x);

                let o1 = (h1 * d).extend(0.);
                let o2 = (h2 * d).extend(0.);
                add_line(v1 + o1, v2 + o2);
                add_line(v1 - o1, v2 - o2);
                add_line(v1 + vec3(0., 0., v1.z), v2 + vec3(0., 0., v2.z));
            }

            // horizontal perimeter circle
            for (location, h_range, _v_range) in &points_data {
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
                            *location.x + h_range * a.0,
                            *location.y + h_range * a.1,
                            *location.z,
                        ),
                        vec3(
                            *location.x + h_range * b.0,
                            *location.y + h_range * b.1,
                            *location.z,
                        ),
                    );
                }
            }

            // vertical half circles
            for (location, h_range, v_range) in &points_data {
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
                            *location.x + h_range * a.0,
                            *location.y,
                            *location.z + v_range * a.1,
                        ),
                        vec3(
                            *location.x + h_range * b.0,
                            *location.y,
                            *location.z + v_range * b.1,
                        ),
                    );
                    add_line(
                        vec3(
                            *location.x,
                            *location.y + h_range * a.0,
                            *location.z + v_range * a.1,
                        ),
                        vec3(
                            *location.x,
                            *location.y + h_range * b.0,
                            *location.z + v_range * b.1,
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

        RoomFeature::EntranceFeature(_) => {
            let typed = UEntranceFeature::from_properties(props).unwrap();
            let location = typed.location();
            let entrance_type = typed.entrance_type();

            let albedo = match entrance_type {
                "ECaveEntranceType::EntranceAndExit" => Srgba {
                    r: 0,
                    g: 255,
                    b: 255,
                    a: 200,
                },
                "ECaveEntranceType::Entrance" => Srgba {
                    r: 255,
                    g: 100,
                    b: 0,
                    a: 200,
                },
                "ECaveEntranceType::Exit" => Srgba {
                    r: 255,
                    g: 0,
                    b: 100,
                    a: 200,
                },
                "ECaveEntranceType::TreassureRoom" => Srgba {
                    r: 255,
                    g: 200,
                    b: 0,
                    a: 200,
                },
                _ => Srgba {
                    r: 128,
                    g: 128,
                    b: 128,
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
            sphere.set_transformation(
                Mat4::from_translation(location.into()) * Mat4::from_scale(100.0),
            );
            vec![Box::new(sphere)]
        }

        RoomFeature::SpawnActorFeature(_) => {
            let typed = USpawnActorFeature::from_properties(props).unwrap();
            let location = typed.location();

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
                Mat4::from_translation(location.into())
                    * Mat4::from_nonuniform_scale(100.0, 100.0, 300.0)
                    * Mat4::from_angle_y(-Radians::turn_div_4()),
            );
            vec![Box::new(obj)]
        }

        RoomFeature::DropPodCalldownLocationFeature(_) => {
            let typed = UDropPodCalldownLocationFeature::from_properties(props).unwrap();
            let location = typed.location();

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
                Mat4::from_translation(location.into())
                    * Mat4::from_nonuniform_scale(100.0, 100.0, 300.0)
                    * Mat4::from_angle_y(Radians::turn_div_4()),
            );
            vec![Box::new(sphere)]
        }

        _ => Vec::new(),
    }
}

/// Build editor UI for a room feature
pub fn edit_feature<'s>(
    pool: &'s mut ObjectPool,
    handle: ObjectHandle,
    feature: &RoomFeature,
    ui: &mut egui::Ui,
    gizmos: &mut Gizmos<'s>,
) -> bool {
    // For now, just show the feature name
    // Full editing support requires more work to handle mutable property access
    ui.label(format!("Feature: {}", feature.name()));

    let obj = pool.get(handle).expect("Invalid handle");
    let props = obj.properties();

    match feature {
        RoomFeature::FloodFillBox(_) => {
            let typed = UFloodFillBox::from_properties(props).unwrap();
            ui.label(format!("Position: {:?}", typed.position()));
            ui.label(format!("Extends: {:?}", typed.extends()));
            false
        }

        RoomFeature::FloodFillPillar(_) => {
            let typed = UFloodFillPillar::from_properties(props).unwrap();
            ui.label(format!("Points: {}", typed.points().len()));
            false
        }

        RoomFeature::FloodFillLine(_) => {
            let typed = UFloodFillLine::from_properties(props).unwrap();
            ui.label(format!("Points: {}", typed.points().len()));
            false
        }

        RoomFeature::EntranceFeature(_) => {
            let typed = UEntranceFeature::from_properties(props).unwrap();
            ui.label(format!("Location: {:?}", typed.location()));
            ui.label(format!("Type: {}", typed.entrance_type()));
            false
        }

        RoomFeature::SpawnActorFeature(_) => {
            let typed = USpawnActorFeature::from_properties(props).unwrap();
            ui.label(format!("Location: {:?}", typed.location()));
            false
        }

        RoomFeature::DropPodCalldownLocationFeature(_) => {
            let typed = UDropPodCalldownLocationFeature::from_properties(props).unwrap();
            ui.label(format!("Location: {:?}", typed.location()));
            false
        }

        _ => {
            ui.label("(no editor available)");
            false
        }
    }
}

fn vector3_editor(ui: &mut egui::Ui, vec: &mut FVector) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.add(egui::DragValue::new(&mut *vec.x).speed(1.))
            .c(&mut changed);
        ui.add(egui::DragValue::new(&mut *vec.y).speed(1.))
            .c(&mut changed);
        ui.add(egui::DragValue::new(&mut *vec.z).speed(1.))
            .c(&mut changed);
    });
    changed
}
