//! Property schema utilities for jmap-based property tag generation
//!
//! This module provides functionality to look up property schemas from jmap
//! reflection data and convert them to uesave PropertyTagPartial for serialization.

use jmap::Jmap;
use uesave::{PropertyTagPartial, PropertyType, StructType};

use crate::core::object_path::ObjectPath;

/// Schema provider for looking up property tags from jmap
pub struct PropertySchemaProvider<'a> {
    jmap: &'a Jmap,
}

impl<'a> PropertySchemaProvider<'a> {
    /// Create a new property schema provider
    pub fn new(jmap: &'a Jmap) -> Self {
        Self { jmap }
    }

    /// Get schema for a property path within a struct
    ///
    /// Parses property chains like "Points.Location" or "Rooms.Key.ID" and
    /// resolves them through the jmap type system.
    pub fn get_schema(&self, struct_path: &str, property_path: &str) -> Option<PropertyTagPartial> {
        // Parse the property chain (e.g., "Points.Location" or "Rooms.Key.ID")
        let parts: Vec<&str> = property_path.split('.').collect();
        if parts.is_empty() {
            return None;
        }

        // Start with the first property from the current struct
        let first_property_name = parts[0];
        let mut current_jmap_prop = self.find_jmap_property(struct_path, first_property_name)?;
        let mut current_type = &current_jmap_prop.r#type;

        // Walk through the property chain
        for part in &parts[1..] {
            match current_type {
                jmap::PropertyType::Map {
                    key_prop,
                    value_prop,
                } => match *part {
                    "Key" => {
                        current_type = &key_prop.r#type;
                    }
                    "Value" => {
                        current_type = &value_prop.r#type;
                    }
                    _ => {
                        return None;
                    }
                },
                jmap::PropertyType::Array { inner } => {
                    // Navigate to the inner type first
                    current_type = &inner.r#type;
                    // Then look up the property on the inner type
                    match current_type {
                        jmap::PropertyType::Struct { r#struct } => {
                            current_jmap_prop = self.find_jmap_property(r#struct, part)?;
                            current_type = &current_jmap_prop.r#type;
                        }
                        _ => {
                            return None;
                        }
                    }
                }
                jmap::PropertyType::Set { key_prop } => {
                    // Navigate to the key type first
                    current_type = &key_prop.r#type;
                    // Then look up the property on the inner type
                    match current_type {
                        jmap::PropertyType::Struct { r#struct } => {
                            current_jmap_prop = self.find_jmap_property(r#struct, part)?;
                            current_type = &current_jmap_prop.r#type;
                        }
                        _ => {
                            return None;
                        }
                    }
                }
                jmap::PropertyType::Struct { r#struct } => {
                    // Direct struct property lookup
                    current_jmap_prop = self.find_jmap_property(r#struct, part)?;
                    current_type = &current_jmap_prop.r#type;
                }
                _ => {
                    return None;
                }
            }
        }

        // Convert the final type to PropertyTagDataPartial
        let tag_data = convert_jmap_type(current_type)?;

        Some(PropertyTagPartial {
            id: None,
            data: tag_data,
        })
    }

    /// Look up a property in a specific struct path, following the super chain
    fn find_jmap_property(&self, struct_path: &str, property_name: &str) -> Option<jmap::Property> {
        let mut current_path = Some(struct_path);

        let lower_property_name = property_name.to_ascii_lowercase();

        while let Some(path) = current_path {
            if let Some(obj_type) = self.jmap.objects.get(path) {
                if let Some(struct_def) = obj_type.get_struct() {
                    // Check if this struct has the property
                    if let Some(prop) = struct_def
                        .properties
                        .iter()
                        .find(|p| p.name.to_ascii_lowercase() == lower_property_name)
                    {
                        return Some(prop.clone());
                    }

                    // Follow super chain
                    current_path = struct_def.super_struct.as_deref();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        None
    }
}

/// Convert jmap::PropertyType to PropertyTagDataPartial
fn convert_jmap_type(jmap_type: &jmap::PropertyType) -> Option<uesave::PropertyTagDataPartial> {
    use jmap::PropertyType as JType;
    use uesave::PropertyTagDataPartial;

    match jmap_type {
        JType::Struct { r#struct } => {
            // Map struct path to specialized StructType variant
            let struct_type = match StructType::from_full(r#struct, false) {
                // TODO handle this in a better way
                // for old property tags, only struct object name is serialized, not full path
                StructType::Struct(Some(full_path)) => {
                    StructType::Struct(Some(ObjectPath::new(full_path).object_name().to_string()))
                }
                t => t,
            };
            Some(PropertyTagDataPartial::Struct {
                struct_type,
                id: uesave::FGuid::default(),
            })
        }
        JType::Array { inner } => {
            let inner_data = convert_jmap_type(&inner.r#type)?;
            Some(PropertyTagDataPartial::Array(Box::new(inner_data)))
        }
        JType::Set { key_prop } => {
            let key_data = convert_jmap_type(&key_prop.r#type)?;
            Some(PropertyTagDataPartial::Set {
                key_type: Box::new(key_data),
            })
        }
        JType::Map {
            key_prop,
            value_prop,
        } => {
            let key_data = convert_jmap_type(&key_prop.r#type)?;
            let value_data = convert_jmap_type(&value_prop.r#type)?;
            Some(PropertyTagDataPartial::Map {
                key_type: Box::new(key_data),
                value_type: Box::new(value_data),
            })
        }
        JType::Byte { r#enum } => {
            Some(PropertyTagDataPartial::Byte(r#enum.as_ref().map(|e| {
                ObjectPath::new(e.to_string()).object_name().to_string()
            })))
        }
        JType::Enum { r#enum, .. } => Some(PropertyTagDataPartial::Enum(
            ObjectPath::new(r#enum.as_ref().unwrap().clone())
                .object_name()
                .to_string(),
            None,
        )),
        _ => {
            let prop_type = jmap_type_to_property_type(jmap_type)?;
            Some(PropertyTagDataPartial::Other(prop_type))
        }
    }
}

/// Convert jmap::PropertyType to uesave::PropertyType
fn jmap_type_to_property_type(jmap_type: &jmap::PropertyType) -> Option<PropertyType> {
    use jmap::PropertyType as JType;

    match jmap_type {
        JType::Struct { .. } | JType::Array { .. } | JType::Set { .. } | JType::Map { .. } => {
            unreachable!()
        }
        JType::Int8 => Some(PropertyType::Int8Property),
        JType::Int16 => Some(PropertyType::Int16Property),
        JType::Int => Some(PropertyType::IntProperty),
        JType::Int64 => Some(PropertyType::Int64Property),
        JType::UInt16 => Some(PropertyType::UInt16Property),
        JType::UInt32 => Some(PropertyType::UInt32Property),
        JType::UInt64 => Some(PropertyType::UInt64Property),
        JType::Float => Some(PropertyType::FloatProperty),
        JType::Double => Some(PropertyType::DoubleProperty),
        JType::Bool { .. } => Some(PropertyType::BoolProperty),
        JType::Str => Some(PropertyType::StrProperty),
        JType::Name => Some(PropertyType::NameProperty),
        JType::Text => Some(PropertyType::TextProperty),
        JType::Object { .. } | JType::Class { .. } => Some(PropertyType::ObjectProperty),
        JType::SoftObject { .. } => Some(PropertyType::SoftObjectProperty),
        JType::MulticastInlineDelegate { .. } => todo!(),
        JType::MulticastSparseDelegate { .. } => todo!(),
        JType::MulticastDelegate { .. } => todo!(),
        JType::Delegate { .. } => todo!(),
        JType::Enum { .. } => todo!(),
        JType::Byte { .. } => todo!(),
        JType::WeakObject { .. } => todo!(),
        JType::SoftClass { .. } => todo!(),
        JType::LazyObject { .. } => todo!(),
        JType::Interface { .. } => todo!(),
        JType::FieldPath => todo!(),
        JType::Optional { .. } => todo!(),
        JType::Utf8Str => todo!(),
        JType::AnsiStr => todo!(),
    }
}
