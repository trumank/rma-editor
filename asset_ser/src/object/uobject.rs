use super::types::{AAR, AAW, ObjectType, Result};
use crate::core::object_pool::{AssetArchiveType, ObjectRef};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct UObject {
    pub object_flags: jmap::EObjectFlags,
    pub properties: uesave::Properties<AssetArchiveType>,
}

impl Default for UObject {
    fn default() -> Self {
        Self {
            object_flags: jmap::EObjectFlags::empty(),
            properties: Default::default(),
        }
    }
}

impl ObjectType for UObject {
    fn de(&mut self, ar: &mut AAR) -> Result<()> {
        self.properties = uesave::read_properties_until_none(ar)?;
        Ok(())
    }

    fn ser(&self, ar: &mut AAW) -> Result<()> {
        uesave::write_properties_none_terminated(ar, &self.properties)?;
        Ok(())
    }

    fn collect_property_refs(&self, refs: &mut Vec<ObjectRef>) {
        crate::core::property_visitor::visit_object_refs(&self.properties, &mut |obj_ref| {
            refs.push(obj_ref.clone());
        });
    }

    fn get_preload_dependencies(&self, _deps: &mut Vec<ObjectRef>) {}

    fn properties(&self) -> &uesave::Properties<AssetArchiveType> {
        &self.properties
    }

    fn properties_mut(&mut self) -> &mut uesave::Properties<AssetArchiveType> {
        &mut self.properties
    }

    fn as_uobject(&self) -> &UObject {
        self
    }

    fn as_uobject_mut(&mut self) -> &mut UObject {
        self
    }
}
