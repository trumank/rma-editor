//! Macros for property access to reduce boilerplate in conversion code.

/// Read a property with a default value if missing or wrong type.
///
/// Supported types:
/// - `Vector` -> FVector
/// - `Rotator` -> FRotator
/// - `f32` -> f32 (default 0.0)
/// - `f32, $default` -> f32 with custom default
/// - `bool` -> bool (default false)
/// - `i32` -> i32 (default 0)
/// - `Enum` -> &str (empty string if missing)
/// - `ObjectRef` -> Option<ObjectRef>
/// - `Struct` -> Option<&Properties>
/// - `ObjectArray` -> &[ObjectRef] (empty slice if missing)
/// - `StructArray` -> &[StructValue] (empty slice if missing)
#[macro_export]
macro_rules! get_prop {
    // Vector
    ($props:expr, $name:literal => Vector) => {{
        let key = uesave::PropertyKey::from($name);
        match $props.0.get(&key) {
            Some(uesave::Property::Struct(uesave::StructValue::Vector(v))) => {
                $crate::objects::FVector {
                    x: v.x.0 as f32,
                    y: v.y.0 as f32,
                    z: v.z.0 as f32,
                }
            }
            _ => $crate::objects::FVector::default(),
        }
    }};

    // Rotator
    ($props:expr, $name:literal => Rotator) => {{
        let key = uesave::PropertyKey::from($name);
        match $props.0.get(&key) {
            Some(uesave::Property::Struct(uesave::StructValue::Rotator(r))) => {
                $crate::objects::FRotator {
                    pitch: r.x.0 as f32,
                    yaw: r.y.0 as f32,
                    roll: r.z.0 as f32,
                }
            }
            _ => $crate::objects::FRotator::default(),
        }
    }};

    // Float with default 0.0
    ($props:expr, $name:literal => f32) => {{
        let key = uesave::PropertyKey::from($name);
        match $props.0.get(&key) {
            Some(uesave::Property::Float(f)) => f.0 as f32,
            _ => 0.0,
        }
    }};

    // Float with custom default
    ($props:expr, $name:literal => f32, $default:expr) => {{
        let key = uesave::PropertyKey::from($name);
        match $props.0.get(&key) {
            Some(uesave::Property::Float(f)) => f.0 as f32,
            _ => $default,
        }
    }};

    // Bool
    ($props:expr, $name:literal => bool) => {{
        let key = uesave::PropertyKey::from($name);
        match $props.0.get(&key) {
            Some(uesave::Property::Bool(b)) => *b,
            _ => false,
        }
    }};

    // i32
    ($props:expr, $name:literal => i32) => {{
        let key = uesave::PropertyKey::from($name);
        match $props.0.get(&key) {
            Some(uesave::Property::Int(i)) => *i,
            _ => 0,
        }
    }};

    // Enum (returns Option<&str>)
    ($props:expr, $name:literal => Enum) => {{
        let key = uesave::PropertyKey::from($name);
        match $props.0.get(&key) {
            Some(uesave::Property::Enum(e)) => Some(e.as_str()),
            _ => None,
        }
    }};

    // Object reference
    ($props:expr, $name:literal => ObjectRef) => {{
        let key = uesave::PropertyKey::from($name);
        $props.0.get(&key).and_then(|p| match p {
            uesave::Property::Object(r) => Some(r.clone()),
            _ => None,
        })
    }};

    // Struct (nested properties)
    ($props:expr, $name:literal => Struct) => {{
        let key = uesave::PropertyKey::from($name);
        match $props.0.get(&key) {
            Some(uesave::Property::Struct(uesave::StructValue::Struct(p))) => Some(p),
            _ => None,
        }
    }};

    // Array of ObjectRefs
    ($props:expr, $name:literal => ObjectArray) => {{
        let key = uesave::PropertyKey::from($name);
        match $props.0.get(&key) {
            Some(uesave::Property::Array(uesave::ValueVec::Object(refs))) => refs.as_slice(),
            _ => &[],
        }
    }};

    // Array of Structs
    ($props:expr, $name:literal => StructArray) => {{
        let key = uesave::PropertyKey::from($name);
        match $props.0.get(&key) {
            Some(uesave::Property::Array(uesave::ValueVec::Struct(structs))) => structs.as_slice(),
            _ => &[],
        }
    }};

    // String
    ($props:expr, $name:literal => String) => {{
        let key = uesave::PropertyKey::from($name);
        match $props.0.get(&key) {
            Some(uesave::Property::Str(s)) => s.clone(),
            _ => String::new(),
        }
    }};

    // Name (FName)
    ($props:expr, $name:literal => Name) => {{
        let key = uesave::PropertyKey::from($name);
        match $props.0.get(&key) {
            Some(uesave::Property::Name(n)) => Some(n.clone()),
            _ => None,
        }
    }};
}

/// Write a property to the Properties map.
///
/// Supported types:
/// - `Vector($val)` - FVector
/// - `Rotator($val)` - FRotator
/// - `f32($val)` - Float
/// - `bool($val)` - Bool
/// - `i32($val)` - Int
/// - `Enum($type, $val)` - Enum with type and value
/// - `Object($val)` - ObjectRef
/// - `ObjectArray($val)` - Vec<ObjectRef>
/// - `StructArray($val)` - Vec<StructValue>
/// - `String($val)` - Str
#[macro_export]
macro_rules! set_prop {
    ($props:expr, $name:literal => Vector($val:expr)) => {{
        $props.0.insert(
            uesave::PropertyKey::from($name),
            uesave::Property::Struct(uesave::StructValue::Vector(uesave::Vector {
                x: ($val.x as f64).into(),
                y: ($val.y as f64).into(),
                z: ($val.z as f64).into(),
            })),
        );
    }};

    ($props:expr, $name:literal => Rotator($val:expr)) => {{
        $props.0.insert(
            uesave::PropertyKey::from($name),
            uesave::Property::Struct(uesave::StructValue::Rotator(uesave::Rotator {
                x: ($val.pitch as f64).into(),
                y: ($val.yaw as f64).into(),
                z: ($val.roll as f64).into(),
            })),
        );
    }};

    ($props:expr, $name:literal => f32($val:expr)) => {{
        $props.0.insert(
            uesave::PropertyKey::from($name),
            uesave::Property::Float(($val as f64).into()),
        );
    }};

    ($props:expr, $name:literal => bool($val:expr)) => {{
        $props.0.insert(
            uesave::PropertyKey::from($name),
            uesave::Property::Bool($val),
        );
    }};

    ($props:expr, $name:literal => i32($val:expr)) => {{
        $props.0.insert(
            uesave::PropertyKey::from($name),
            uesave::Property::Int($val),
        );
    }};

    ($props:expr, $name:literal => Enum($enum_type:expr, $val:expr)) => {{
        $props.0.insert(
            uesave::PropertyKey::from($name),
            uesave::Property::Enum($val.to_string()),
        );
    }};

    ($props:expr, $name:literal => Object($val:expr)) => {{
        $props.0.insert(
            uesave::PropertyKey::from($name),
            uesave::Property::Object($val),
        );
    }};

    ($props:expr, $name:literal => ObjectArray($val:expr)) => {{
        $props.0.insert(
            uesave::PropertyKey::from($name),
            uesave::Property::Array(uesave::ValueVec::Object($val)),
        );
    }};

    ($props:expr, $name:literal => StructArray($struct_type:expr, $val:expr)) => {{
        $props.0.insert(
            uesave::PropertyKey::from($name),
            uesave::Property::Array(uesave::ValueVec::Struct($val)),
        );
    }};

    ($props:expr, $name:literal => String($val:expr)) => {{
        $props.0.insert(
            uesave::PropertyKey::from($name),
            uesave::Property::Str($val.to_string()),
        );
    }};
}

pub use get_prop;
pub use set_prop;
