//! Load asset_ser ObjectPool data into clean objects.rs types.

use anyhow::{Context, Result};
use asset_ser::core::object_pool::{AssetArchiveType, ObjectHandle, ObjectPool, ObjectRef};
use std::collections::BTreeSet;
use uesave::{Properties, Property, PropertyKey, StructValue};

use crate::objects::*;

use super::enums::{
    extract_class_name, parse_adjustment_type, parse_entrance_priority, parse_entrance_type,
    parse_mirroring_support,
};

/// Helper: get an ObjectRef from a property by name (runtime string)
fn get_object_ref(props: &Properties<AssetArchiveType>, name: &str) -> Option<ObjectRef> {
    let key = PropertyKey::from(name);
    match props.0.get(&key) {
        Some(Property::Object(r)) => Some(r.clone()),
        _ => None,
    }
}

/// Load a URoomGenerator from the object pool
pub fn load_room_generator(pool: &ObjectPool, handle: ObjectHandle) -> Result<URoomGenerator> {
    let obj = pool
        .get(handle)
        .context("Failed to get root object from pool")?;
    let props = obj.properties();

    Ok(URoomGenerator {
        base: load_room_generator_base(props),
        room_features: load_children_as_features(pool, props),
    })
}

/// Load the base properties of a room generator
fn load_room_generator_base(props: &Properties<AssetArchiveType>) -> URoomGeneratorBase {
    URoomGeneratorBase {
        bounds: get_prop!(props, "Bounds" => f32, 1.0),
        can_only_be_used_once: get_prop!(props, "CanOnlyBeUsedOnce" => bool),
        mirror_support: parse_mirroring_support(get_prop!(props, "MirrorSupport" => Enum)),
        room_tags: load_gameplay_tag_container(props),
    }
}

/// Load a FGameplayTagContainer from properties
fn load_gameplay_tag_container(props: &Properties<AssetArchiveType>) -> FGameplayTagContainer {
    // GameplayTags are stored in a nested struct with a GameplayTags array
    if let Some(tag_struct) = get_prop!(props, "RoomTags" => Struct)
        && let Some(tags_array) = get_prop!(tag_struct, "GameplayTags" => Struct) {
            // Try to extract tag names from the array
            let mut tags = BTreeSet::new();
            for (_key, prop) in tags_array.0.iter() {
                if let uesave::Property::Name(name) = prop {
                    tags.insert(name.clone());
                }
            }
            return FGameplayTagContainer(tags);
        }
    FGameplayTagContainer(BTreeSet::new())
}

/// Load child features from the RoomFeatures array
fn load_children_as_features(
    pool: &ObjectPool,
    props: &Properties<AssetArchiveType>,
) -> Vec<URoomFeature> {
    get_prop!(props, "RoomFeatures" => ObjectArray)
        .iter()
        .filter_map(|r| r.as_handle())
        .filter_map(|h| load_room_feature(pool, h).ok())
        .collect()
}

/// Load a single URoomFeature from the pool
fn load_room_feature(pool: &ObjectPool, handle: ObjectHandle) -> Result<URoomFeature> {
    let obj = pool.get(handle).context("Failed to get feature object")?;
    let class_path = pool.resolve_path(&obj.class);
    let class_name = extract_class_name(class_path.as_str());
    let props = obj.properties();

    Ok(URoomFeature {
        children: load_children_as_features(pool, props),
        feature_type: load_feature_type(pool, props, class_name)?,
    })
}

/// Load the specific feature type based on class name
fn load_feature_type(
    pool: &ObjectPool,
    props: &Properties<AssetArchiveType>,
    class: &str,
) -> Result<URoomFeatureType> {
    Ok(match class {
        "FloodFillBox" => URoomFeatureType::FloodFillBox(load_flood_fill_box(pool, props)),
        "FloodFillLine" => URoomFeatureType::FloodFillLine(load_flood_fill_line(pool, props)),
        "FloodFillPillar" => URoomFeatureType::FloodFillPillar(load_flood_fill_pillar(pool, props)),
        "FloodFillProceduralPillar" => {
            URoomFeatureType::FloodFillProceduralPillar(load_procedural_pillar(pool, props))
        }
        "EntranceFeature" => URoomFeatureType::Entrance(load_entrance(props)),
        "RandomSelector" => URoomFeatureType::RandomSelector(load_random_selector(props)),
        "RandomSubRoomFeature" => {
            URoomFeatureType::RandomSubRoom(load_random_sub_room(pool, props))
        }
        "SubRoomFeature" => URoomFeatureType::SubRoom(load_sub_room(pool, props)),
        "SpawnActorFeature" => URoomFeatureType::SpawnActor(load_spawn_actor(props)),
        "SpawnTriggerFeature" => URoomFeatureType::SpawnTrigger(load_spawn_trigger(props)),
        "ResourceFeature" => URoomFeatureType::Resource(load_resource(pool, props)),
        "DropPodCalldownLocationFeature" => {
            URoomFeatureType::DropPodCalldownLocation(load_drop_pod_calldown(props))
        }
        _ => {
            // Unknown feature type - default to a flood fill box
            log::warn!("Unknown feature type: {}", class);
            URoomFeatureType::FloodFillBox(UFloodFillBox {
                noise: None,
                position: get_prop!(props, "Position" => Vector),
                extends: FVector {
                    x: 100.0,
                    y: 100.0,
                    z: 100.0,
                },
                rotation: FRotator::default(),
                is_carver: false,
                noise_range: 0.0,
            })
        }
    })
}

/// Load a UFloodFillBox feature
fn load_flood_fill_box(pool: &ObjectPool, props: &Properties<AssetArchiveType>) -> UFloodFillBox {
    UFloodFillBox {
        noise: load_flood_fill_settings_ref(pool, props, "Noise"),
        position: get_prop!(props, "Position" => Vector),
        extends: get_prop!(props, "Extends" => Vector),
        rotation: get_prop!(props, "Rotation" => Rotator),
        is_carver: get_prop!(props, "IsCarver" => bool),
        noise_range: get_prop!(props, "NoiseRange" => f32),
    }
}

/// Load a UFloodFillLine feature
fn load_flood_fill_line(pool: &ObjectPool, props: &Properties<AssetArchiveType>) -> UFloodFillLine {
    UFloodFillLine {
        wall_noise_override: load_flood_fill_settings_ref(pool, props, "WallNoiseOverride"),
        ceiling_noise_override: load_flood_fill_settings_ref(pool, props, "CeilingNoiseOverride"),
        floor_noise_override: load_flood_fill_settings_ref(pool, props, "FloorNoiseOverride"),
        use_detail_noise: get_prop!(props, "UseDetailNoise" => bool),
        points: load_room_line_points(props),
    }
}

/// Load FRoomLinePoint array from Points property
fn load_room_line_points(props: &Properties<AssetArchiveType>) -> Vec<FRoomLinePoint> {
    get_prop!(props, "Points" => StructArray)
        .iter()
        .filter_map(|sv| {
            if let StructValue::Struct(point_props) = sv {
                Some(FRoomLinePoint {
                    location: get_prop!(point_props, "Location" => Vector),
                    h_range: get_prop!(point_props, "HRange" => f32),
                    v_range: get_prop!(point_props, "VRange" => f32),
                    cieling_noise_range: get_prop!(point_props, "CielingNoiseRange" => f32),
                    wall_noise_range: get_prop!(point_props, "WallNoiseRange" => f32),
                    floor_noise_range: get_prop!(point_props, "FloorNoiseRange" => f32),
                    cieling_height: get_prop!(point_props, "Cielingheight" => f32),
                    height_scale: get_prop!(point_props, "HeightScale" => f32, 1.0),
                    floor_depth: get_prop!(point_props, "FloorDepth" => f32),
                    floor_angle: get_prop!(point_props, "FloorAngle" => f32),
                })
            } else {
                None
            }
        })
        .collect()
}

/// Load a UFloodFillPillar feature
fn load_flood_fill_pillar(
    pool: &ObjectPool,
    props: &Properties<AssetArchiveType>,
) -> UFloodFillPillar {
    UFloodFillPillar {
        noise_override: load_flood_fill_settings_ref(pool, props, "NoiseOverride"),
        points: load_rand_line_points(props),
        range_scale: load_rand_range(props, "RangeScale"),
        noise_range_scale: load_rand_range(props, "NoiseRangeScale"),
        endcap_scale: load_rand_range(props, "EndcapScale"),
    }
}

/// Load FRandLinePoint array from Points property
fn load_rand_line_points(props: &Properties<AssetArchiveType>) -> Vec<FRandLinePoint> {
    get_prop!(props, "Points" => StructArray)
        .iter()
        .filter_map(|sv| {
            if let StructValue::Struct(point_props) = sv {
                Some(FRandLinePoint {
                    location: get_prop!(point_props, "Location" => Vector),
                    range: load_rand_range(point_props, "Range"),
                    noise_range: load_rand_range(point_props, "NoiseRange"),
                    skew_factor: load_rand_range(point_props, "SkewFactor"),
                    fill_amount: load_rand_range(point_props, "FillAmount"),
                })
            } else {
                None
            }
        })
        .collect()
}

/// Load a FRandRange from a named property
fn load_rand_range(props: &Properties<AssetArchiveType>, name: &str) -> FRandRange {
    let key = uesave::PropertyKey::from(name);
    if let Some(uesave::Property::Struct(StructValue::Struct(range_props))) = props.0.get(&key) {
        FRandRange {
            min: get_prop!(range_props, "Min" => f32),
            max: get_prop!(range_props, "Max" => f32),
        }
    } else {
        FRandRange { min: 0.0, max: 0.0 }
    }
}

/// Load a UFloodFillProceduralPillar feature
fn load_procedural_pillar(
    pool: &ObjectPool,
    props: &Properties<AssetArchiveType>,
) -> UFloodFillProceduralPillar {
    UFloodFillProceduralPillar {
        points: load_vector_points(props),
        pillar_settings: load_pillar_settings_ref(pool, props),
    }
}

/// Load a simple Vec<FVector> from Points property
fn load_vector_points(props: &Properties<AssetArchiveType>) -> Vec<FVector> {
    get_prop!(props, "Points" => StructArray)
        .iter()
        .filter_map(|sv| {
            if let StructValue::Vector(v) = sv {
                Some(FVector {
                    x: v.x.0 as f32,
                    y: v.y.0 as f32,
                    z: v.z.0 as f32,
                })
            } else {
                None
            }
        })
        .collect()
}

/// Load a UEntranceFeature
fn load_entrance(props: &Properties<AssetArchiveType>) -> UEntranceFeature {
    UEntranceFeature {
        location: get_prop!(props, "Location" => Vector),
        direction: get_prop!(props, "Direction" => Rotator),
        entrance_type: parse_entrance_type(get_prop!(props, "EntranceType" => Enum)),
        priority: parse_entrance_priority(get_prop!(props, "Priority" => Enum)),
    }
}

/// Load a URandomSelector
fn load_random_selector(props: &Properties<AssetArchiveType>) -> URandomSelector {
    URandomSelector {
        min: get_prop!(props, "Min" => i32),
        max: get_prop!(props, "Max" => i32),
    }
}

/// Load a URandomSubRoomFeature
fn load_random_sub_room(
    pool: &ObjectPool,
    props: &Properties<AssetArchiveType>,
) -> URandomSubRoomFeature {
    URandomSubRoomFeature {
        room_group: load_room_group_ref(pool, props),
        tag_query: FGameplayTagQuery {},
        layer: get_prop!(props, "Layer" => i32),
        location: get_prop!(props, "Location" => Vector),
        rotation: get_prop!(props, "Rotation" => Rotator),
        scale: get_prop!(props, "Scale" => f32, 1.0),
    }
}

/// Load a USubRoomFeature
fn load_sub_room(pool: &ObjectPool, props: &Properties<AssetArchiveType>) -> USubRoomFeature {
    USubRoomFeature {
        room_generator: load_room_generator_ref(pool, props),
        location: get_prop!(props, "Location" => Vector),
        rotation: get_prop!(props, "Rotation" => Rotator),
        scale: get_prop!(props, "Scale" => f32, 1.0),
    }
}

/// Load a USpawnActorFeature
fn load_spawn_actor(props: &Properties<AssetArchiveType>) -> USpawnActorFeature {
    USpawnActorFeature {
        location: get_prop!(props, "Location" => Vector),
        actor_to_spawn: get_prop!(props, "ActorToSpawn" => ObjectRef)
            .and_then(|r| r.as_path().map(|s| s.to_string())),
        adjustment_direction: get_prop!(props, "AdjustmentDirection" => Vector),
        adjustment: parse_adjustment_type(get_prop!(props, "Adjustment" => Enum)),
        scale_min: get_prop!(props, "ScaleMin" => Vector),
        scale_max: get_prop!(props, "ScaleMax" => Vector),
        rotation_delta: get_prop!(props, "RotationDelta" => Rotator),
    }
}

/// Load a USpawnTriggerFeature
fn load_spawn_trigger(props: &Properties<AssetArchiveType>) -> USpawnTriggerFeature {
    USpawnTriggerFeature {
        trigger_class: get_prop!(props, "TriggerClass" => ObjectRef)
            .and_then(|r| r.as_path().map(|s| s.to_string())),
        transform: FTransform {
            translation: get_prop!(props, "Location" => Vector),
            rotation: get_prop!(props, "Rotation" => Rotator).into(),
            Scale3D: FVector {
                x: get_prop!(props, "Scale" => f32, 1.0),
                y: get_prop!(props, "Scale" => f32, 1.0),
                z: get_prop!(props, "Scale" => f32, 1.0),
            },
        },
        message: get_prop!(props, "Message" => String),
    }
}

/// Load a UResourceFeature
fn load_resource(_pool: &ObjectPool, props: &Properties<AssetArchiveType>) -> UResourceFeature {
    UResourceFeature {
        location: get_prop!(props, "Location" => Vector),
        resource: None, // TODO: implement resource data loading
        base_amount: get_prop!(props, "BaseAmount" => f32),
    }
}

/// Load a UDropPodCalldownLocationFeature
fn load_drop_pod_calldown(props: &Properties<AssetArchiveType>) -> UDropPodCalldownLocationFeature {
    UDropPodCalldownLocationFeature {
        location: get_prop!(props, "Location" => Vector),
        calldown_class: get_prop!(props, "CalldownClass" => ObjectRef)
            .and_then(|r| r.as_path().map(|s| s.to_string())),
    }
}

/// Load UFloodFillSettings from an object reference in the properties
fn load_flood_fill_settings_ref(
    pool: &ObjectPool,
    props: &Properties<AssetArchiveType>,
    prop_name: &str,
) -> Option<Box<UFloodFillSettings>> {
    let obj_ref = get_object_ref(props, prop_name)?;
    let handle = obj_ref.as_handle()?;
    let obj = pool.get(handle)?;
    let settings_props = obj.properties();

    Some(Box::new(UFloodFillSettings {
        noise_size: get_prop!(settings_props, "NoiseSize" => Vector),
        freq_multiplier: get_prop!(settings_props, "FreqMultiplier" => f32, 1.0),
        amplitude_multiplier: get_prop!(settings_props, "AmplitudeMultiplier" => f32, 1.0),
        min_value: get_prop!(settings_props, "MinValue" => f32),
        max_value: get_prop!(settings_props, "MaxValue" => f32, 1.0),
        turbulence: get_prop!(settings_props, "Turbulence" => bool),
        invert: get_prop!(settings_props, "Invert" => bool),
        octaves: get_prop!(settings_props, "Octaves" => i32),
        noise_layers: load_noise_layers(pool, settings_props),
    }))
}

/// Load noise layers array
fn load_noise_layers(
    pool: &ObjectPool,
    props: &Properties<AssetArchiveType>,
) -> Vec<FLayeredNoise> {
    get_prop!(props, "NoiseLayers" => StructArray)
        .iter()
        .filter_map(|sv| {
            if let StructValue::Struct(layer_props) = sv {
                Some(FLayeredNoise {
                    noise: load_flood_fill_settings_ref(pool, layer_props, "Noise"),
                    scale: get_prop!(layer_props, "Scale" => f32, 1.0),
                })
            } else {
                None
            }
        })
        .collect()
}

/// Load UPillarSettings from an object reference
fn load_pillar_settings_ref(
    pool: &ObjectPool,
    props: &Properties<AssetArchiveType>,
) -> Option<Box<UPillarSettings>> {
    let obj_ref = get_prop!(props, "PillarSettings" => ObjectRef)?;
    let handle = obj_ref.as_handle()?;
    let obj = pool.get(handle)?;
    let settings_props = obj.properties();

    Some(Box::new(UPillarSettings {
        pillar_segments: load_pillar_segments(settings_props),
        point_range: load_rand_range(settings_props, "PointRange"),
        point_noise_range: load_rand_range(settings_props, "PointNoiseRange"),
        pillar_range_scale: load_rand_range(settings_props, "PillarRangeScale"),
        pillar_noise_range_scale: load_rand_range(settings_props, "PillarNoiseRangeScale"),
        noise: load_flood_fill_settings_ref(pool, settings_props, "Noise"),
    }))
}

/// Load pillar segments array
fn load_pillar_segments(props: &Properties<AssetArchiveType>) -> Vec<FPillarSegment> {
    get_prop!(props, "PillarSegments" => StructArray)
        .iter()
        .filter_map(|sv| {
            if let StructValue::Struct(seg_props) = sv {
                Some(FPillarSegment {
                    scale: get_prop!(seg_props, "Scale" => f32, 1.0),
                    height_offset: get_prop!(seg_props, "HeightOffset" => f32),
                })
            } else {
                None
            }
        })
        .collect()
}

/// Load URoomGeneratorGroup from an object reference
fn load_room_group_ref(
    pool: &ObjectPool,
    props: &Properties<AssetArchiveType>,
) -> Option<Box<URoomGeneratorGroup>> {
    let obj_ref = get_prop!(props, "RoomGroup" => ObjectRef)?;
    let handle = obj_ref.as_handle()?;
    let obj = pool.get(handle)?;
    let group_props = obj.properties();

    Some(Box::new(URoomGeneratorGroup {
        rooms: get_prop!(group_props, "Rooms" => ObjectArray)
            .iter()
            .filter_map(|r| r.as_handle())
            .filter_map(|h| load_room_generator(pool, h).ok())
            .collect(),
    }))
}

/// Load nested URoomGenerator from an object reference
fn load_room_generator_ref(
    pool: &ObjectPool,
    props: &Properties<AssetArchiveType>,
) -> Option<Box<URoomGenerator>> {
    let obj_ref = get_prop!(props, "RoomGenerator" => ObjectRef)?;
    let handle = obj_ref.as_handle()?;
    load_room_generator(pool, handle).ok().map(Box::new)
}
