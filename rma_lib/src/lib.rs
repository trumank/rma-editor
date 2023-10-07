pub use rma_proc::*;

use std::io::{Read, Seek};

use anyhow::{bail, Result};
use unreal_asset::{
    properties::{Property, PropertyDataTrait},
    types::PackageIndex,
    Asset,
};

pub fn from_object_property<C: Read + Seek, T: FromExport<C>>(
    asset: &Asset<C>,
    property: &Property,
) -> Result<T> {
    match property {
        Property::ObjectProperty(property) => T::from_export(asset, property.value),
        _ => bail!("wrong property type"),
    }
}

pub trait FromExport<C: Seek + Read> {
    fn from_export(asset: &Asset<C>, package_index: PackageIndex) -> Result<Self>
    where
        Self: Sized;
}
pub trait FromProperty<C: Seek + Read> {
    fn from_property(asset: &Asset<C>, property: &Property) -> Result<Self>
    where
        Self: Sized;
}

impl<C: Read + Seek> FromProperty<C> for f32 {
    fn from_property(asset: &Asset<C>, property: &Property) -> Result<Self> {
        match property {
            Property::FloatProperty(property) => Ok(property.value.0),
            _ => bail!("{property:#?}"),
        }
    }
}

impl<C: Read + Seek> FromProperty<C> for i32 {
    fn from_property(asset: &Asset<C>, property: &Property) -> Result<Self> {
        match property {
            Property::IntProperty(property) => Ok(property.value),
            _ => bail!("{property:#?}"),
        }
    }
}

impl<C: Read + Seek, T: FromProperty<C>> FromProperty<C> for Vec<T> {
    fn from_property(asset: &Asset<C>, property: &Property) -> Result<Self> {
        let mut values = vec![];
        match property {
            Property::ArrayProperty(property) => {
                for value in &property.value {
                    values.push(T::from_property(asset, value)?);
                }
            }
            _ => bail!("wrong property type"),
        }
        Ok(values)
    }
}

pub fn property_or_default<C: Read + Seek, T: Default + FromProperty<C>>(
    asset: &Asset<C>,
    properties: &[Property],
    name: &str,
) -> Result<T> {
    for property in properties {
        if property.get_name().get_content(|c| c == name) {
            return T::from_property(asset, property);
        }
    }
    Ok(T::default())
}
