use std::ops::{Deref, DerefMut};

use super::types::{AAR, AAW, ObjectType, Result, impl_object_type_base};
use super::uobject::UObject;
use super::ustruct::UStruct;
use crate::core::object_pool::{AssetArchiveType, ObjectRef};
use byteorder::{LE, ReadBytesExt as _};
use serde::Serialize;
use uesave::ArchiveReader as _;

#[derive(Debug, Serialize)]
pub struct UClass {
    pub base: UStruct,
    pub func_map: Vec<(String, ObjectRef)>,
    pub class_flags: jmap::EClassFlags,
    pub class_within: ObjectRef,
    pub class_config_name: String,
    pub class_generated_by: ObjectRef,
    pub interfaces: Vec<FImplementedInterface>,
    pub deprecated_force_script_order: bool,
    pub cooked: bool,
    pub class_default_object: ObjectRef,
}

impl Default for UClass {
    fn default() -> Self {
        Self {
            base: Default::default(),
            func_map: Vec::new(),
            class_flags: jmap::EClassFlags::empty(),
            class_within: ObjectRef::Unloaded("None".into()),
            class_config_name: String::new(),
            class_generated_by: ObjectRef::Unloaded("None".into()),
            interfaces: Vec::new(),
            deprecated_force_script_order: false,
            cooked: false,
            class_default_object: ObjectRef::Unloaded("None".into()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FImplementedInterface {
    pub class: ObjectRef,
    pub pointer_offset: i32,
    pub implemented_in_blueprint: bool,
}

impl Deref for UClass {
    type Target = UStruct;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
impl DerefMut for UClass {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl ObjectType for UClass {
    fn de(&mut self, ar: &mut AAR) -> Result<()> {
        self.base.de(ar)?;

        self.func_map = vec![];
        for _ in 0..ar.read_u32::<LE>()? {
            self.func_map
                .push((ar.read_fname()?, ar.read_object_ref()?));
        }

        self.class_flags = jmap::EClassFlags::from_bits(ar.read_u32::<LE>()?).unwrap();
        self.class_within = ar.read_object_ref()?;
        self.class_config_name = ar.read_fname()?;

        self.interfaces = vec![];
        for _ in 0..ar.read_u32::<LE>()? {
            self.interfaces.push(FImplementedInterface {
                class: ar.read_object_ref()?,
                pointer_offset: ar.read_i32::<LE>()?,
                implemented_in_blueprint: ar.read_u32::<LE>()? != 0,
            });
        }

        self.class_generated_by = ar.read_object_ref()?;
        self.deprecated_force_script_order = ar.read_u32::<LE>()? != 0;

        let _ = ar.read_u64::<LE>()?; // Unknown/padding

        self.cooked = ar.read_u32::<LE>()? != 0;
        self.class_default_object = ar.read_object_ref()?;

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
        self.base.get_preload_dependencies(deps);

        deps.push(self.class_within.clone());
        deps.push(self.class_generated_by.clone());
        deps.push(self.class_default_object.clone());

        for interface in &self.interfaces {
            deps.push(interface.class.clone());
        }
    }

    impl_object_type_base!(delegate base);
}
