use std::ops::{Deref, DerefMut};

use super::types::{AAR, AAW, ObjectType, Result, impl_object_type_base};
use super::uobject::UObject;
use super::ustruct::UStruct;
use crate::core::object_pool::{AssetArchiveType, ObjectRef};
use byteorder::{LE, ReadBytesExt as _, WriteBytesExt as _};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct UFunction {
    pub base: UStruct,
    pub function_flags: jmap::EFunctionFlags,
}
impl Default for UFunction {
    fn default() -> Self {
        Self {
            base: Default::default(),
            function_flags: jmap::EFunctionFlags::empty(),
        }
    }
}
impl Deref for UFunction {
    type Target = UStruct;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
impl DerefMut for UFunction {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl ObjectType for UFunction {
    fn de(&mut self, ar: &mut AAR) -> Result<()> {
        self.base.de(ar)?;
        self.function_flags = jmap::EFunctionFlags::from_bits(ar.read_u32::<LE>()?).unwrap();
        Ok(())
    }

    fn ser(&self, ar: &mut AAW) -> Result<()> {
        self.base.ser(ar)?;
        ar.write_u32::<LE>(self.function_flags.bits())?;
        Ok(())
    }

    fn collect_property_refs(&self, refs: &mut Vec<ObjectRef>) {
        self.base.collect_property_refs(refs);
    }

    fn get_preload_dependencies(&self, deps: &mut Vec<ObjectRef>) {
        self.base.get_preload_dependencies(deps);
    }

    impl_object_type_base!(delegate base);
}
