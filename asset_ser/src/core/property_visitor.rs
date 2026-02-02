//! Generic property visitor for walking ObjectRef instances in property trees.

use crate::core::object_pool::{AssetArchiveType, ObjectRef};
use uesave::{Properties, Property, StructValue, ValueVec};

/// Visit all ObjectRef instances in a properties collection.
pub fn visit_object_refs<F>(properties: &Properties<AssetArchiveType>, visitor: &mut F)
where
    F: FnMut(&ObjectRef),
{
    for (_key, property) in properties.0.iter() {
        visit_property_object_refs(property, visitor);
    }
}

fn visit_property_object_refs<F>(property: &Property<AssetArchiveType>, visitor: &mut F)
where
    F: FnMut(&ObjectRef),
{
    match property {
        Property::Object(obj_ref) => visitor(obj_ref),
        Property::Delegate(d) => visitor(&d.object),
        Property::MulticastDelegate(d) => d.0.iter().for_each(|d| visitor(&d.object)),
        Property::MulticastInlineDelegate(d) => d.0.iter().for_each(|d| visitor(&d.object)),
        Property::MulticastSparseDelegate(d) => d.0.iter().for_each(|d| visitor(&d.object)),
        Property::Struct(StructValue::Struct(nested)) => visit_object_refs(nested, visitor),
        Property::Array(v) | Property::Set(v) => visit_value_vec_object_refs(v, visitor),
        Property::Map(entries) => {
            for e in entries {
                visit_property_object_refs(&e.key, visitor);
                visit_property_object_refs(&e.value, visitor);
            }
        }
        _ => {}
    }
}

fn visit_value_vec_object_refs<F>(value_vec: &ValueVec<AssetArchiveType>, visitor: &mut F)
where
    F: FnMut(&ObjectRef),
{
    match value_vec {
        ValueVec::Object(objects) => objects.iter().for_each(visitor),
        ValueVec::Struct(structs) => {
            for sv in structs {
                if let StructValue::Struct(nested) = sv {
                    visit_object_refs(nested, visitor);
                }
            }
        }
        _ => {}
    }
}
