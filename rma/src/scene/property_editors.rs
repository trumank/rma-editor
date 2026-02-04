//! Reusable property editor widgets for egui
//!
//! Each editor function returns `true` if the value was changed.

use strum::IntoEnumIterator;
use three_d::egui::{self, DragValue, Ui};

use crate::objects::*;

/// Edit an f32 value with a drag slider
pub fn edit_f32(ui: &mut Ui, label: &str, value: &mut f32) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        changed = ui.add(DragValue::new(value).speed(1.0)).changed();
    });
    changed
}

/// Edit an f32 value with custom speed
pub fn edit_f32_speed(ui: &mut Ui, label: &str, value: &mut f32, speed: f64) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        changed = ui.add(DragValue::new(value).speed(speed)).changed();
    });
    changed
}

/// Edit an i32 value with a drag slider
pub fn edit_i32(ui: &mut Ui, label: &str, value: &mut i32) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        changed = ui.add(DragValue::new(value).speed(0.1)).changed();
    });
    changed
}

/// Edit a bool value with a checkbox
pub fn edit_bool(ui: &mut Ui, label: &str, value: &mut bool) -> bool {
    ui.checkbox(value, label).changed()
}

/// Edit a String value with a text edit
pub fn edit_string(ui: &mut Ui, label: &str, value: &mut String) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        changed = ui.text_edit_singleline(value).changed();
    });
    changed
}

/// Edit an FVector (3D position/size) with x/y/z drag values
pub fn edit_fvector(ui: &mut Ui, label: &str, value: &mut FVector) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        ui.label("X:");
        changed |= ui.add(DragValue::new(&mut value.x).speed(1.0)).changed();
        ui.label("Y:");
        changed |= ui.add(DragValue::new(&mut value.y).speed(1.0)).changed();
        ui.label("Z:");
        changed |= ui.add(DragValue::new(&mut value.z).speed(1.0)).changed();
    });
    changed
}

/// Edit an FRotator (pitch/yaw/roll) with drag values
pub fn edit_frotator(ui: &mut Ui, label: &str, value: &mut FRotator) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        ui.label("P:");
        changed |= ui
            .add(DragValue::new(&mut value.pitch).speed(1.0))
            .changed();
        ui.label("Y:");
        changed |= ui.add(DragValue::new(&mut value.yaw).speed(1.0)).changed();
        ui.label("R:");
        changed |= ui.add(DragValue::new(&mut value.roll).speed(1.0)).changed();
    });
    changed
}

/// Edit an FRandRange (min/max pair) with drag values
pub fn edit_frand_range(ui: &mut Ui, label: &str, value: &mut FRandRange) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        ui.label("Min:");
        changed |= ui.add(DragValue::new(&mut value.min).speed(1.0)).changed();
        ui.label("Max:");
        changed |= ui.add(DragValue::new(&mut value.max).speed(1.0)).changed();
    });
    changed
}

/// Edit an enum value using a ComboBox
pub fn edit_enum<E>(ui: &mut Ui, label: &str, value: &mut E) -> bool
where
    E: IntoEnumIterator + std::fmt::Debug + PartialEq + Clone,
{
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        egui::ComboBox::from_id_salt(label)
            .selected_text(format!("{:?}", value))
            .show_ui(ui, |ui| {
                for variant in E::iter() {
                    let text = format!("{:?}", variant);
                    if ui.selectable_label(*value == variant, &text).clicked() {
                        *value = variant;
                        changed = true;
                    }
                }
            });
    });
    changed
}

/// Edit a Vec of items with add/remove buttons and collapsible editors
pub fn edit_vec<T, F>(
    ui: &mut Ui,
    label: &str,
    items: &mut Vec<T>,
    default_item: impl Fn() -> T,
    mut edit_item: F,
) -> bool
where
    F: FnMut(&mut Ui, usize, &mut T) -> bool,
{
    let mut changed = false;
    let mut to_remove: Option<usize> = None;

    egui::CollapsingHeader::new(format!("{} ({})", label, items.len()))
        .default_open(false)
        .show(ui, |ui| {
            for (idx, item) in items.iter_mut().enumerate() {
                ui.push_id(idx, |ui| {
                    ui.horizontal(|ui| {
                        egui::CollapsingHeader::new(format!("[{}]", idx))
                            .default_open(false)
                            .show(ui, |ui| {
                                changed |= edit_item(ui, idx, item);
                            });
                        if ui.small_button("-").clicked() {
                            to_remove = Some(idx);
                        }
                    });
                });
            }

            if ui.button("+").clicked() {
                items.push(default_item());
                changed = true;
            }
        });

    if let Some(idx) = to_remove {
        items.remove(idx);
        changed = true;
    }

    changed
}

/// Edit an FRoomLinePoint (location + ranges for flood fill lines)
pub fn edit_room_line_point(ui: &mut Ui, _idx: usize, point: &mut FRoomLinePoint) -> bool {
    let mut changed = false;
    changed |= edit_fvector(ui, "Location", &mut point.location);
    changed |= edit_f32(ui, "H Range", &mut point.h_range);
    changed |= edit_f32(ui, "V Range", &mut point.v_range);
    changed |= edit_f32(ui, "Ceiling Noise Range", &mut point.cieling_noise_range);
    changed |= edit_f32(ui, "Wall Noise Range", &mut point.wall_noise_range);
    changed |= edit_f32(ui, "Floor Noise Range", &mut point.floor_noise_range);
    changed |= edit_f32(ui, "Ceiling Height", &mut point.cieling_height);
    changed |= edit_f32(ui, "Height Scale", &mut point.height_scale);
    changed |= edit_f32(ui, "Floor Depth", &mut point.floor_depth);
    changed |= edit_f32(ui, "Floor Angle", &mut point.floor_angle);
    changed
}

/// Edit an FRandLinePoint (location + rand ranges for pillars)
pub fn edit_rand_line_point(ui: &mut Ui, _idx: usize, point: &mut FRandLinePoint) -> bool {
    let mut changed = false;
    changed |= edit_fvector(ui, "Location", &mut point.location);
    changed |= edit_frand_range(ui, "Range", &mut point.range);
    changed |= edit_frand_range(ui, "Noise Range", &mut point.noise_range);
    changed |= edit_frand_range(ui, "Skew Factor", &mut point.skew_factor);
    changed |= edit_frand_range(ui, "Fill Amount", &mut point.fill_amount);
    changed
}

/// Edit an FVector point (for procedural pillar points)
pub fn edit_fvector_point(ui: &mut Ui, _idx: usize, point: &mut FVector) -> bool {
    edit_fvector(ui, "Point", point)
}

/// Edit an Option<UClass> (optional class reference as string)
/// Empty string is treated as None
pub fn edit_optional_uclass(ui: &mut Ui, label: &str, value: &mut Option<UClass>) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        let mut text = value.clone().unwrap_or_default();
        if ui.text_edit_singleline(&mut text).changed() {
            *value = if text.is_empty() { None } else { Some(text) };
            changed = true;
        }
    });
    changed
}
