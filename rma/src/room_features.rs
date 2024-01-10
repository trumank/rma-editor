use three_d::{
    egui, BoundingBox, CpuMaterial, CpuMesh, Gm, InstancedMesh, Instances, Mesh, Object,
    PhysicalMaterial,
};
use three_d_asset::{vec2, vec3, Angle, InnerSpace, Mat4, Quat, Radians, Srgba, Vector3};

use crate::{
    rma::{
        DropPodCalldownLocationFeature, ECaveEntranceType, EntranceFeature, FVector, FloodFillBox,
        FloodFillLine, FloodFillPillar, SpawnActorFeature,
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

pub trait RoomFeatureTrait {
    fn build(&self, ctx: &RMAContext) -> Vec<Box<dyn Object>>;
    fn editor(&mut self, ui: &mut egui::Ui) -> bool;
}

impl From<FVector> for Vector3<f32> {
    fn from(val: FVector) -> Self {
        vec3(val.x, val.y, val.z)
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

impl RoomFeatureTrait for FloodFillBox {
    fn build(&self, ctx: &RMAContext) -> Vec<Box<dyn Object>> {
        // only used in RMA_Escort10
        let mut mesh = BoundingBox::new(ctx.context, CpuMesh::cube().compute_aabb());
        mesh.set_transformation(
            Mat4::from_translation(self.position.into())
                * Mat4::from_nonuniform_scale(self.extends.x, self.extends.y, self.extends.z),
        );

        vec![Box::new(Gm::new(mesh, ctx.wireframe_material.clone()))]
    }
    fn editor(&mut self, ui: &mut egui::Ui) -> bool {
        false
    }
}

impl RoomFeatureTrait for FloodFillPillar {
    fn build(&self, ctx: &RMAContext) -> Vec<Box<dyn Object>> {
        let mut transformations = Vec::new();

        let mut add_line = |p1, p2| transformations.push(line_transform(p1, p2));

        for pair in self.points.windows(2) {
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
    fn editor(&mut self, ui: &mut egui::Ui) -> bool {
        ui.label("FloodFillPillar");
        false
    }
}

impl RoomFeatureTrait for SpawnActorFeature {
    fn build(&self, ctx: &RMAContext) -> Vec<Box<dyn Object>> {
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
            Mat4::from_translation(self.location.into())
                * Mat4::from_nonuniform_scale(100.0, 100.0, 300.0)
                * Mat4::from_angle_y(-Radians::turn_div_4()),
        );
        vec![Box::new(obj)]
    }
    fn editor(&mut self, ui: &mut egui::Ui) -> bool {
        todo!()
    }
}

impl RoomFeatureTrait for FloodFillLine {
    fn build(&self, ctx: &RMAContext) -> Vec<Box<dyn Object>> {
        let mut transformations = Vec::new();

        let mut add_line = |p1, p2| transformations.push(line_transform(p1, p2));

        for pair in self.points.windows(2) {
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
        for point in &self.points {
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
        for point in &self.points {
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
    fn editor(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        egui::Grid::new("grid")
            .num_columns(2)
            .spacing([40.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                for (i, point) in self.points.iter_mut().enumerate() {
                    ui.label(format!("point {i}"));
                    vector3(ui, &mut point.location).c(&mut changed);
                    ui.end_row();
                }
            });

        changed
    }
}

impl RoomFeatureTrait for EntranceFeature {
    fn build(&self, ctx: &RMAContext) -> Vec<Box<dyn Object>> {
        let albedo = match self.entrance_type {
            ECaveEntranceType::EntranceAndExit => Srgba {
                r: 0,
                g: 255,
                b: 255,
                a: 200,
            },
            ECaveEntranceType::Entrance => Srgba {
                r: 255,
                g: 100,
                b: 0,
                a: 200,
            },
            ECaveEntranceType::Exit => Srgba {
                r: 255,
                g: 0,
                b: 100,
                a: 200,
            },
            ECaveEntranceType::TreassureRoom => Srgba {
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
            Mat4::from_translation(self.location.into()) * Mat4::from_scale(100.0),
        );
        vec![Box::new(sphere)]
    }
    fn editor(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        egui::Grid::new("grid")
            .num_columns(2)
            .spacing([40.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label("location");
                vector3(ui, &mut self.location).c(&mut changed);
                ui.end_row();
            });

        changed
    }
}

fn vector3(ui: &mut egui::Ui, vec: &mut FVector) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.add(egui::DragValue::new(&mut vec.x).speed(1.))
            .c(&mut changed);
        ui.add(egui::DragValue::new(&mut vec.y).speed(1.))
            .c(&mut changed);
        ui.add(egui::DragValue::new(&mut vec.z).speed(1.))
            .c(&mut changed);
    });
    changed
}

impl RoomFeatureTrait for DropPodCalldownLocationFeature {
    fn build(&self, ctx: &RMAContext) -> Vec<Box<dyn Object>> {
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
            Mat4::from_translation(self.location.into())
                * Mat4::from_nonuniform_scale(100.0, 100.0, 300.0)
                * Mat4::from_angle_y(Radians::turn_div_4()),
        );
        vec![Box::new(sphere)]
    }
    fn editor(&mut self, ui: &mut egui::Ui) -> bool {
        todo!()
    }
}
