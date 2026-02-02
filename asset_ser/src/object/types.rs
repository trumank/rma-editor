use crate::archive::reader::AssetArchiveReader;
use crate::archive::writer::AssetArchiveWriter;
use crate::core::object_pool::{AssetArchiveType, ObjectRef};
use byteorder::{LE, ReadBytesExt as _};
use serde::{Deserialize, Serialize};
use uesave::ArchiveReader;
pub use uesave::Error;

pub type Result<T> = std::result::Result<T, Error>;

/// Macro to implement the common ObjectType trait methods for types with a base field.
///
/// Usage:
/// - `impl_object_type_base!(direct base)` - for types where base is UObject
/// - `impl_object_type_base!(delegate base)` - for types where base delegates to UObject
macro_rules! impl_object_type_base {
    (direct $base:ident) => {
        fn properties(&self) -> &uesave::Properties<AssetArchiveType> {
            &self.properties
        }
        fn properties_mut(&mut self) -> &mut uesave::Properties<AssetArchiveType> {
            &mut self.properties
        }
        fn as_uobject(&self) -> &UObject {
            &self.$base
        }
        fn as_uobject_mut(&mut self) -> &mut UObject {
            &mut self.$base
        }
    };
    (delegate $base:ident) => {
        fn properties(&self) -> &uesave::Properties<AssetArchiveType> {
            &self.properties
        }
        fn properties_mut(&mut self) -> &mut uesave::Properties<AssetArchiveType> {
            &mut self.properties
        }
        fn as_uobject(&self) -> &UObject {
            self.$base.as_uobject()
        }
        fn as_uobject_mut(&mut self) -> &mut UObject {
            self.$base.as_uobject_mut()
        }
    };
}
pub(super) use impl_object_type_base;

pub type AAR<'a, 'b> = AssetArchiveReader<'a, std::io::Cursor<&'b [u8]>>;
pub type AAW<'a> = AssetArchiveWriter<'a, std::io::Cursor<Vec<u8>>>;

/// Trait for downcasting to concrete types
pub trait AsAny: std::any::Any {
    fn as_any(&self) -> &dyn std::any::Any;
}

impl<T: std::any::Any> AsAny for T {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Trait for all Unreal Engine object types
///
/// Uses concrete archive types (AssetArchiveReader with Cursor, AssetArchiveWriter with Cursor)
/// to maintain dyn-compatibility while avoiding generic parameters.
pub trait ObjectType: std::fmt::Debug + AsAny {
    /// Deserialize this object from the archive (read)
    ///
    /// Uses Cursor<&[u8]> as the concrete reader type used during asset loading
    fn de(&mut self, ar: &mut AAR) -> Result<()>;

    /// Serialize this object to the archive (write)
    ///
    /// Uses AssetArchiveWriter with Cursor<Vec<u8>> as the concrete writer type
    fn ser(&self, ar: &mut AAW) -> Result<()>;

    /// Collect object references from properties for dependency tracking
    fn collect_property_refs(&self, refs: &mut Vec<ObjectRef>);

    /// Get custom preload dependencies for special object types
    fn get_preload_dependencies(&self, deps: &mut Vec<ObjectRef>);

    fn properties(&self) -> &uesave::Properties<AssetArchiveType>;
    fn properties_mut(&mut self) -> &mut uesave::Properties<AssetArchiveType>;

    fn as_uobject(&self) -> &super::uobject::UObject;
    fn as_uobject_mut(&mut self) -> &mut super::uobject::UObject;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FField {
    pub name: String,
    pub flags: u32,
}

impl FField {
    pub fn read(ar: &mut AAR) -> Result<Self> {
        Ok(Self {
            name: ar.read_fname()?,
            flags: ar.read_u32::<LE>()?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FProperty {
    pub base: FField,
    pub array_dim: u32,
    pub element_size: u32,
    pub property_flags: jmap::EPropertyFlags,
    pub rep_index: u16,
    pub rep_notify_func: String,
    pub lifetime_condition: u8,
    pub r#type: FPropertyType,
}

impl FProperty {
    pub fn read(ar: &mut AAR) -> Result<Self> {
        let r#type = ar.read_fname()?;
        Ok(Self {
            base: FField::read(ar)?,
            array_dim: ar.read_u32::<LE>()?,
            element_size: ar.read_u32::<LE>()?,
            property_flags: jmap::EPropertyFlags::from_bits(ar.read_u64::<LE>()?).unwrap(),
            rep_index: ar.read_u16::<LE>()?,
            rep_notify_func: ar.read_fname()?,
            lifetime_condition: ar.read_u8()?,
            r#type: FPropertyType::read(&r#type, ar)?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum FPropertyType {
    FStructProperty(ObjectRef),
    FObjectProperty(ObjectRef),
    FClassProperty(ObjectRef, ObjectRef),
    FInterfaceProperty(ObjectRef),
    FDelegateProperty(ObjectRef),
    FStrProperty,
    FNameProperty,
    FTextProperty,
    FIntProperty,
    FFloatProperty,
    FArrayProperty(Box<FProperty>),
    FSetProperty(Box<FProperty>),
    FMapProperty(Box<FProperty>, Box<FProperty>),
    FByteProperty(ObjectRef),
    FBoolProperty {
        field_size: u8,
        byte_offset: u8,
        byte_mask: u8,
        field_mask: u8,
        native_bool: bool,
        value: bool,
    },
}

impl FPropertyType {
    fn read(r#type: &str, ar: &mut AAR) -> Result<Self> {
        Ok(match r#type {
            "StructProperty" => Self::FStructProperty(ar.read_object_ref()?),
            "ObjectProperty" => Self::FObjectProperty(ar.read_object_ref()?),
            "ClassProperty" => Self::FClassProperty(ar.read_object_ref()?, ar.read_object_ref()?),
            "InterfaceProperty" => Self::FInterfaceProperty(ar.read_object_ref()?),
            "DelegateProperty" => Self::FDelegateProperty(ar.read_object_ref()?),
            "StrProperty" => Self::FStrProperty,
            "NameProperty" => Self::FNameProperty,
            "TextProperty" => Self::FTextProperty,
            "IntProperty" => Self::FIntProperty,
            "FloatProperty" => Self::FFloatProperty,
            "ArrayProperty" => Self::FArrayProperty(FProperty::read(ar)?.into()),
            "SetProperty" => Self::FSetProperty(FProperty::read(ar)?.into()),
            "MapProperty" => {
                Self::FMapProperty(FProperty::read(ar)?.into(), FProperty::read(ar)?.into())
            }
            "ByteProperty" => Self::FByteProperty(ar.read_object_ref()?),
            "BoolProperty" => Self::FBoolProperty {
                field_size: ar.read_u8()?,
                byte_offset: ar.read_u8()?,
                byte_mask: ar.read_u8()?,
                field_mask: ar.read_u8()?,
                native_bool: ar.read_u8()? > 0,
                value: ar.read_u8()? > 0,
            },
            other => todo!("unimplemented property type {other:?}"),
        })
    }
}
