//! Save clean objects.rs types back to asset_ser ObjectPool.
//!
//! This module recreates pool objects from scratch rather than updating existing ones.

use anyhow::Result;
use asset_ser::core::name::Name;
use asset_ser::core::object_pool::{
    AssetArchiveType, LoadedObject, ObjectHandle, ObjectPool, ObjectRef,
};
use asset_ser::object::{ObjectType, UObject};
use uesave::{Properties, StructValue};

use crate::objects::*;

use super::enums::{
    adjustment_type_to_string, entrance_priority_to_string, entrance_type_to_string,
    mirroring_support_to_string,
};

/// Save a URoomGenerator to the object pool, returning the handle
pub fn save_room_generator(
    pool: &mut ObjectPool,
    room: &URoomGenerator,
    outer: Option<ObjectRef>,
    name: &str,
) -> Result<ObjectHandle> {
    let mut props = Properties::default();
    save_room_generator_base(&mut props, &room.base);

    // Create this object first to get its handle for children's outer
    // Root objects (no outer) get special flags
    let is_root = outer.is_none();
    let self_handle = allocate_object(
        pool,
        name,
        outer.clone(),
        "/Script/FSD.RoomGenerator",
        props,
        is_root,
    );
    let self_ref = ObjectRef::Loaded(self_handle);

    // Create child features
    let child_refs: Vec<ObjectRef> = room
        .room_features
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let child_name = format!("{}_Feature_{}", name, i);
            let child_handle =
                save_room_feature(pool, f, Some(self_ref.clone()), &child_name).unwrap();
            ObjectRef::Loaded(child_handle)
        })
        .collect();

    // Update the object with child refs
    let obj = pool.get_mut(self_handle).unwrap();
    set_prop!(obj.properties_mut(), "RoomFeatures" => ObjectArray(child_refs));

    Ok(self_handle)
}

/// Save URoomGeneratorBase properties
fn save_room_generator_base(props: &mut Properties<AssetArchiveType>, base: &URoomGeneratorBase) {
    set_prop!(props, "Bounds" => f32(base.bounds));
    set_prop!(props, "CanOnlyBeUsedOnce" => bool(base.can_only_be_used_once));
    set_prop!(props, "MirrorSupport" => Enum("ERoomMirroringSupport", mirroring_support_to_string(base.mirror_support)));
    // TODO: save room tags if needed
}

/// Save a URoomFeature to the pool
pub fn save_room_feature(
    pool: &mut ObjectPool,
    feature: &URoomFeature,
    outer: Option<ObjectRef>,
    name: &str,
) -> Result<ObjectHandle> {
    let mut props = Properties::default();
    let class_path =
        save_feature_type_props(pool, &mut props, &feature.feature_type, outer.clone(), name)?;

    // Create this object first to get its handle for children's outer
    // Child features are never root objects
    let self_handle = allocate_object(pool, name, outer.clone(), &class_path, props, false);
    let self_ref = ObjectRef::Loaded(self_handle);

    // Create child features recursively
    let child_refs: Vec<ObjectRef> = feature
        .children
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let child_name = format!("{}_Child_{}", name, i);
            let child_handle =
                save_room_feature(pool, f, Some(self_ref.clone()), &child_name).unwrap();
            ObjectRef::Loaded(child_handle)
        })
        .collect();

    // Update the object with child refs if there are any
    if !child_refs.is_empty() {
        let obj = pool.get_mut(self_handle).unwrap();
        set_prop!(obj.properties_mut(), "RoomFeatures" => ObjectArray(child_refs));
    }

    Ok(self_handle)
}

/// Save feature-specific properties and return the class path
fn save_feature_type_props(
    pool: &mut ObjectPool,
    props: &mut Properties<AssetArchiveType>,
    feature_type: &URoomFeatureType,
    outer: Option<ObjectRef>,
    name: &str,
) -> Result<String> {
    Ok(match feature_type {
        URoomFeatureType::FloodFillBox(f) => {
            save_flood_fill_box(pool, props, f, outer, name)?;
            "/Script/FSD.FloodFillBox".to_string()
        }
        URoomFeatureType::FloodFillLine(f) => {
            save_flood_fill_line(pool, props, f, outer, name)?;
            "/Script/FSD.FloodFillLine".to_string()
        }
        URoomFeatureType::FloodFillPillar(f) => {
            save_flood_fill_pillar(pool, props, f, outer, name)?;
            "/Script/FSD.FloodFillPillar".to_string()
        }
        URoomFeatureType::FloodFillProceduralPillar(f) => {
            save_procedural_pillar(pool, props, f, outer, name)?;
            "/Script/FSD.FloodFillProceduralPillar".to_string()
        }
        URoomFeatureType::Entrance(f) => {
            save_entrance(props, f);
            "/Script/FSD.EntranceFeature".to_string()
        }
        URoomFeatureType::RandomSelector(f) => {
            save_random_selector(props, f);
            "/Script/FSD.RandomSelector".to_string()
        }
        URoomFeatureType::RandomSubRoom(f) => {
            save_random_sub_room(pool, props, f, outer, name)?;
            "/Script/FSD.RandomSubRoomFeature".to_string()
        }
        URoomFeatureType::SubRoom(f) => {
            save_sub_room(pool, props, f, outer, name)?;
            "/Script/FSD.SubRoomFeature".to_string()
        }
        URoomFeatureType::SpawnActor(f) => {
            save_spawn_actor(props, f);
            "/Script/FSD.SpawnActorFeature".to_string()
        }
        URoomFeatureType::SpawnTrigger(f) => {
            save_spawn_trigger(props, f);
            "/Script/FSD.SpawnTriggerFeature".to_string()
        }
        URoomFeatureType::Resource(f) => {
            save_resource(props, f);
            "/Script/FSD.ResourceFeature".to_string()
        }
        URoomFeatureType::DropPodCalldownLocation(f) => {
            save_drop_pod_calldown(props, f);
            "/Script/FSD.DropPodCalldownLocationFeature".to_string()
        }
    })
}

/// Save UFloodFillBox properties
fn save_flood_fill_box(
    pool: &mut ObjectPool,
    props: &mut Properties<AssetArchiveType>,
    f: &UFloodFillBox,
    outer: Option<ObjectRef>,
    name: &str,
) -> Result<()> {
    set_prop!(props, "Position" => Vector(f.position));
    set_prop!(props, "Extends" => Vector(f.extends));
    set_prop!(props, "Rotation" => Rotator(f.rotation));
    set_prop!(props, "IsCarver" => bool(f.is_carver));
    set_prop!(props, "NoiseRange" => f32(f.noise_range));

    if let Some(noise) = &f.noise {
        let noise_name = format!("{}_Noise", name);
        let noise_handle = save_flood_fill_settings(pool, noise, outer, &noise_name)?;
        set_prop!(props, "Noise" => Object(ObjectRef::Loaded(noise_handle)));
    }

    Ok(())
}

/// Save UFloodFillLine properties
fn save_flood_fill_line(
    pool: &mut ObjectPool,
    props: &mut Properties<AssetArchiveType>,
    f: &UFloodFillLine,
    outer: Option<ObjectRef>,
    name: &str,
) -> Result<()> {
    set_prop!(props, "UseDetailNoise" => bool(f.use_detail_noise));

    // Save points as struct array
    let points_structs: Vec<StructValue<AssetArchiveType>> = f
        .points
        .iter()
        .map(|p| {
            let mut point_props = Properties::default();
            set_prop!(point_props, "Location" => Vector(p.location));
            set_prop!(point_props, "HRange" => f32(p.h_range));
            set_prop!(point_props, "VRange" => f32(p.v_range));
            set_prop!(point_props, "CielingNoiseRange" => f32(p.cieling_noise_range));
            set_prop!(point_props, "WallNoiseRange" => f32(p.wall_noise_range));
            set_prop!(point_props, "FloorNoiseRange" => f32(p.floor_noise_range));
            set_prop!(point_props, "Cielingheight" => f32(p.cieling_height));
            set_prop!(point_props, "HeightScale" => f32(p.height_scale));
            set_prop!(point_props, "FloorDepth" => f32(p.floor_depth));
            set_prop!(point_props, "FloorAngle" => f32(p.floor_angle));
            StructValue::Struct(point_props)
        })
        .collect();
    set_prop!(props, "Points" => StructArray("RoomLinePoint", points_structs));

    // Save noise overrides
    if let Some(noise) = &f.wall_noise_override {
        let noise_name = format!("{}_WallNoise", name);
        let handle = save_flood_fill_settings(pool, noise, outer.clone(), &noise_name)?;
        set_prop!(props, "WallNoiseOverride" => Object(ObjectRef::Loaded(handle)));
    }
    if let Some(noise) = &f.ceiling_noise_override {
        let noise_name = format!("{}_CeilingNoise", name);
        let handle = save_flood_fill_settings(pool, noise, outer.clone(), &noise_name)?;
        set_prop!(props, "CeilingNoiseOverride" => Object(ObjectRef::Loaded(handle)));
    }
    if let Some(noise) = &f.floor_noise_override {
        let noise_name = format!("{}_FloorNoise", name);
        let handle = save_flood_fill_settings(pool, noise, outer, &noise_name)?;
        set_prop!(props, "FloorNoiseOverride" => Object(ObjectRef::Loaded(handle)));
    }

    Ok(())
}

/// Save UFloodFillPillar properties
fn save_flood_fill_pillar(
    pool: &mut ObjectPool,
    props: &mut Properties<AssetArchiveType>,
    f: &UFloodFillPillar,
    outer: Option<ObjectRef>,
    name: &str,
) -> Result<()> {
    // Save points as struct array
    let points_structs: Vec<StructValue<AssetArchiveType>> = f
        .points
        .iter()
        .map(|p| {
            let mut point_props = Properties::default();
            set_prop!(point_props, "Location" => Vector(p.location));
            save_rand_range(&mut point_props, "Range", &p.range);
            save_rand_range(&mut point_props, "NoiseRange", &p.noise_range);
            save_rand_range(&mut point_props, "SkewFactor", &p.skew_factor);
            save_rand_range(&mut point_props, "FillAmount", &p.fill_amount);
            StructValue::Struct(point_props)
        })
        .collect();
    set_prop!(props, "Points" => StructArray("RandLinePoint", points_structs));

    save_rand_range(props, "RangeScale", &f.range_scale);
    save_rand_range(props, "NoiseRangeScale", &f.noise_range_scale);
    save_rand_range(props, "EndcapScale", &f.endcap_scale);

    if let Some(noise) = &f.noise_override {
        let noise_name = format!("{}_Noise", name);
        let handle = save_flood_fill_settings(pool, noise, outer, &noise_name)?;
        set_prop!(props, "NoiseOverride" => Object(ObjectRef::Loaded(handle)));
    }

    Ok(())
}

/// Save a FRandRange as a struct property
fn save_rand_range(props: &mut Properties<AssetArchiveType>, name: &str, range: &FRandRange) {
    let mut range_props = Properties::default();
    set_prop!(range_props, "Min" => f32(range.min));
    set_prop!(range_props, "Max" => f32(range.max));

    props.0.insert(
        uesave::PropertyKey::from(name),
        uesave::Property::Struct(StructValue::Struct(range_props)),
    );
}

/// Save UFloodFillProceduralPillar properties
fn save_procedural_pillar(
    pool: &mut ObjectPool,
    props: &mut Properties<AssetArchiveType>,
    f: &UFloodFillProceduralPillar,
    outer: Option<ObjectRef>,
    name: &str,
) -> Result<()> {
    // Save points as vector array
    let points_structs: Vec<StructValue<AssetArchiveType>> = f
        .points
        .iter()
        .map(|p| {
            StructValue::Vector(uesave::Vector {
                x: (p.x as f64).into(),
                y: (p.y as f64).into(),
                z: (p.z as f64).into(),
            })
        })
        .collect();
    set_prop!(props, "Points" => StructArray("Vector", points_structs));

    if let Some(settings) = &f.pillar_settings {
        let settings_name = format!("{}_PillarSettings", name);
        let handle = save_pillar_settings(pool, settings, outer, &settings_name)?;
        set_prop!(props, "PillarSettings" => Object(ObjectRef::Loaded(handle)));
    }

    Ok(())
}

/// Save UEntranceFeature properties
fn save_entrance(props: &mut Properties<AssetArchiveType>, f: &UEntranceFeature) {
    set_prop!(props, "Location" => Vector(f.location));
    set_prop!(props, "Direction" => Rotator(f.direction));
    set_prop!(props, "EntranceType" => Enum("ECaveEntranceType", entrance_type_to_string(f.entrance_type)));
    set_prop!(props, "Priority" => Enum("ECaveEntrancePriority", entrance_priority_to_string(f.priority)));
}

/// Save URandomSelector properties
fn save_random_selector(props: &mut Properties<AssetArchiveType>, f: &URandomSelector) {
    set_prop!(props, "Min" => i32(f.min));
    set_prop!(props, "Max" => i32(f.max));
}

/// Save URandomSubRoomFeature properties
fn save_random_sub_room(
    pool: &mut ObjectPool,
    props: &mut Properties<AssetArchiveType>,
    f: &URandomSubRoomFeature,
    outer: Option<ObjectRef>,
    name: &str,
) -> Result<()> {
    set_prop!(props, "Layer" => i32(f.layer));
    set_prop!(props, "Location" => Vector(f.location));
    set_prop!(props, "Rotation" => Rotator(f.rotation));
    set_prop!(props, "Scale" => f32(f.scale));

    if let Some(group) = &f.room_group {
        let group_name = format!("{}_RoomGroup", name);
        let handle = save_room_group(pool, group, outer, &group_name)?;
        set_prop!(props, "RoomGroup" => Object(ObjectRef::Loaded(handle)));
    }

    Ok(())
}

/// Save USubRoomFeature properties
fn save_sub_room(
    pool: &mut ObjectPool,
    props: &mut Properties<AssetArchiveType>,
    f: &USubRoomFeature,
    outer: Option<ObjectRef>,
    name: &str,
) -> Result<()> {
    set_prop!(props, "Location" => Vector(f.location));
    set_prop!(props, "Rotation" => Rotator(f.rotation));
    set_prop!(props, "Scale" => f32(f.scale));

    if let Some(room) = &f.room_generator {
        let room_name = format!("{}_RoomGenerator", name);
        let handle = save_room_generator(pool, room, outer, &room_name)?;
        set_prop!(props, "RoomGenerator" => Object(ObjectRef::Loaded(handle)));
    }

    Ok(())
}

/// Save USpawnActorFeature properties
fn save_spawn_actor(props: &mut Properties<AssetArchiveType>, f: &USpawnActorFeature) {
    set_prop!(props, "Location" => Vector(f.location));
    if let Some(actor) = &f.actor_to_spawn {
        set_prop!(props, "ActorToSpawn" => Object(ObjectRef::Unloaded(actor.clone().into())));
    }
    set_prop!(props, "AdjustmentDirection" => Vector(f.adjustment_direction));
    set_prop!(props, "Adjustment" => Enum("EItemAdjustmentType", adjustment_type_to_string(f.adjustment)));
    set_prop!(props, "ScaleMin" => Vector(f.scale_min));
    set_prop!(props, "ScaleMax" => Vector(f.scale_max));
    set_prop!(props, "RotationDelta" => Rotator(f.rotation_delta));
}

/// Save USpawnTriggerFeature properties
fn save_spawn_trigger(props: &mut Properties<AssetArchiveType>, f: &USpawnTriggerFeature) {
    if let Some(trigger) = &f.trigger_class {
        set_prop!(props, "TriggerClass" => Object(ObjectRef::Unloaded(trigger.clone().into())));
    }
    set_prop!(props, "Location" => Vector(f.transform.translation));
    set_prop!(props, "Rotation" => Rotator(FRotator::from(f.transform.rotation)));
    set_prop!(props, "Scale" => f32(f.transform.Scale3D.x)); // Use x for uniform scale
    set_prop!(props, "Message" => String(f.message));
}

/// Save UResourceFeature properties
fn save_resource(props: &mut Properties<AssetArchiveType>, f: &UResourceFeature) {
    set_prop!(props, "Location" => Vector(f.location));
    set_prop!(props, "BaseAmount" => f32(f.base_amount));
    // TODO: save resource reference if needed
}

/// Save UDropPodCalldownLocationFeature properties
fn save_drop_pod_calldown(
    props: &mut Properties<AssetArchiveType>,
    f: &UDropPodCalldownLocationFeature,
) {
    set_prop!(props, "Location" => Vector(f.location));
    if let Some(class) = &f.calldown_class {
        set_prop!(props, "CalldownClass" => Object(ObjectRef::Unloaded(class.clone().into())));
    }
}

/// Save UFloodFillSettings to the pool
fn save_flood_fill_settings(
    pool: &mut ObjectPool,
    settings: &UFloodFillSettings,
    outer: Option<ObjectRef>,
    name: &str,
) -> Result<ObjectHandle> {
    let mut props = Properties::default();

    set_prop!(props, "NoiseSize" => Vector(settings.noise_size));
    set_prop!(props, "FreqMultiplier" => f32(settings.freq_multiplier));
    set_prop!(props, "AmplitudeMultiplier" => f32(settings.amplitude_multiplier));
    set_prop!(props, "MinValue" => f32(settings.min_value));
    set_prop!(props, "MaxValue" => f32(settings.max_value));
    set_prop!(props, "Turbulence" => bool(settings.turbulence));
    set_prop!(props, "Invert" => bool(settings.invert));
    set_prop!(props, "Octaves" => i32(settings.octaves));

    let handle = allocate_object(
        pool,
        name,
        outer.clone(),
        "/Script/FSD.FloodFillSettings",
        props,
        false,
    );

    // Save noise layers
    if !settings.noise_layers.is_empty() {
        let layer_structs: Vec<StructValue<AssetArchiveType>> = settings
            .noise_layers
            .iter()
            .enumerate()
            .map(|(i, layer)| {
                let mut layer_props = Properties::default();
                set_prop!(layer_props, "Scale" => f32(layer.scale));

                if let Some(noise) = &layer.noise {
                    let layer_name = format!("{}_Layer_{}", name, i);
                    if let Ok(layer_handle) =
                        save_flood_fill_settings(pool, noise, outer.clone(), &layer_name)
                    {
                        set_prop!(layer_props, "Noise" => Object(ObjectRef::Loaded(layer_handle)));
                    }
                }

                StructValue::Struct(layer_props)
            })
            .collect();

        let obj = pool.get_mut(handle).unwrap();
        set_prop!(obj.properties_mut(), "NoiseLayers" => StructArray("LayeredNoise", layer_structs));
    }

    Ok(handle)
}

/// Save UPillarSettings to the pool
fn save_pillar_settings(
    pool: &mut ObjectPool,
    settings: &UPillarSettings,
    outer: Option<ObjectRef>,
    name: &str,
) -> Result<ObjectHandle> {
    let mut props = Properties::default();

    // Save pillar segments
    let segment_structs: Vec<StructValue<AssetArchiveType>> = settings
        .pillar_segments
        .iter()
        .map(|seg| {
            let mut seg_props = Properties::default();
            set_prop!(seg_props, "Scale" => f32(seg.scale));
            set_prop!(seg_props, "HeightOffset" => f32(seg.height_offset));
            StructValue::Struct(seg_props)
        })
        .collect();
    set_prop!(props, "PillarSegments" => StructArray("PillarSegment", segment_structs));

    save_rand_range(&mut props, "PointRange", &settings.point_range);
    save_rand_range(&mut props, "PointNoiseRange", &settings.point_noise_range);
    save_rand_range(&mut props, "PillarRangeScale", &settings.pillar_range_scale);
    save_rand_range(
        &mut props,
        "PillarNoiseRangeScale",
        &settings.pillar_noise_range_scale,
    );

    let handle = allocate_object(
        pool,
        name,
        outer.clone(),
        "/Script/FSD.PillarSettings",
        props,
        false,
    );

    if let Some(noise) = &settings.noise {
        let noise_name = format!("{}_Noise", name);
        let noise_handle = save_flood_fill_settings(pool, noise, outer, &noise_name)?;
        let obj = pool.get_mut(handle).unwrap();
        set_prop!(obj.properties_mut(), "Noise" => Object(ObjectRef::Loaded(noise_handle)));
    }

    Ok(handle)
}

/// Save URoomGeneratorGroup to the pool
fn save_room_group(
    pool: &mut ObjectPool,
    group: &URoomGeneratorGroup,
    outer: Option<ObjectRef>,
    name: &str,
) -> Result<ObjectHandle> {
    let props = Properties::default();

    // RoomGeneratorGroup is a root object
    let is_root = outer.is_none();
    let handle = allocate_object(
        pool,
        name,
        outer.clone(),
        "/Script/FSD.RoomGeneratorGroup",
        props,
        is_root,
    );
    let self_ref = ObjectRef::Loaded(handle);

    // Save rooms
    let room_refs: Vec<ObjectRef> = group
        .rooms
        .iter()
        .enumerate()
        .map(|(i, room)| {
            let room_name = format!("{}_Room_{}", name, i);
            let room_handle =
                save_room_generator(pool, room, Some(self_ref.clone()), &room_name).unwrap();
            ObjectRef::Loaded(room_handle)
        })
        .collect();

    let obj = pool.get_mut(handle).unwrap();
    set_prop!(obj.properties_mut(), "Rooms" => ObjectArray(room_refs));

    Ok(handle)
}

/// Allocate a new object in the pool
fn allocate_object(
    pool: &mut ObjectPool,
    name: &str,
    outer: Option<ObjectRef>,
    class_path: &str,
    props: Properties<AssetArchiveType>,
    is_root: bool,
) -> ObjectHandle {
    let mut uobj = UObject::default();
    *uobj.properties_mut() = props;

    // Set object flags: root objects get RF_Public | RF_Standalone | RF_Transactional
    if is_root {
        uobj.object_flags = jmap::EObjectFlags::RF_Public
            | jmap::EObjectFlags::RF_Standalone
            | jmap::EObjectFlags::RF_Transactional;
    }

    // Extract class name from path (e.g., "/Script/FSD.RoomGenerator" -> "RoomGenerator")
    let class_name = class_path.rsplit('.').next().unwrap_or("Object");
    let template_path = format!(
        "{}.Default__{}",
        class_path
            .rsplit_once('.')
            .map(|(pkg, _)| pkg)
            .unwrap_or("/Script/FSD"),
        class_name
    );

    let loaded_obj = LoadedObject {
        name: Name::new(name),
        outer,
        class: ObjectRef::Unloaded(class_path.into()),
        template: Some(ObjectRef::Unloaded(template_path.into())),
        object: Box::new(uobj),
    };

    pool.allocate(loaded_obj)
}
