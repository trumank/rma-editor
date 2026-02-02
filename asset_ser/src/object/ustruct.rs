use std::io::Read as _;
use std::ops::{Deref, DerefMut};

use super::types::{AAR, AAW, FProperty, ObjectType, Result, impl_object_type_base};
use super::uobject::UObject;
use crate::core::object_pool::{AssetArchiveType, ObjectRef};
use byteorder::{LE, ReadBytesExt as _};
use jmap::EObjectFlags;
use serde::Serialize;
use uesave::ArchiveReader as _;

/// UStruct type definition - base for UClass, UScriptStruct, UFunction
#[derive(Debug, Default, Serialize)]
pub struct UStruct {
    pub base: UObject,
    pub super_struct: Option<ObjectRef>,
    pub children: Vec<ObjectRef>,
    pub child_properties: Vec<FProperty>,
    pub script: Vec<u8>,
}

impl Deref for UStruct {
    type Target = UObject;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
impl DerefMut for UStruct {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl ObjectType for UStruct {
    fn de(&mut self, ar: &mut AAR) -> Result<()> {
        self.base.de(ar)?;

        if !self
            .object_flags
            .contains(EObjectFlags::RF_ClassDefaultObject)
            && ar.read_u32::<LE>()? > 0
        {
            // TODO handle guid
            let mut guid = [0; 16];
            ar.read_exact(&mut guid)?;
        }

        self.super_struct = Some(ar.read_object_ref()?);

        self.children = vec![];
        for _ in 0..ar.read_u32::<LE>()? {
            self.children.push(ar.read_object_ref()?);
        }

        self.child_properties = vec![];
        for _ in 0..ar.read_u32::<LE>()? {
            self.child_properties.push(FProperty::read(ar)?);
        }

        let _script_bytecode_size = ar.read_u32::<LE>()?;
        let script_storage_size = ar.read_u32::<LE>()?;
        self.script = vec![0; script_storage_size as usize];
        ar.read_exact(&mut self.script)?;

        Ok(())
    }

    fn ser(&self, ar: &mut AAW) -> Result<()> {
        self.base.ser(ar)?;
        Ok(())
    }

    fn collect_property_refs(&self, refs: &mut Vec<ObjectRef>) {
        self.base.collect_property_refs(refs);
    }

    fn get_preload_dependencies(&self, deps: &mut Vec<ObjectRef>) {
        if let Some(super_ref) = &self.super_struct {
            deps.push(super_ref.clone());
        }
    }

    impl_object_type_base!(direct base);
}
