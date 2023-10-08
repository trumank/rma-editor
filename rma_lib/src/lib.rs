pub use rma_proc::*;

use std::{
    collections::HashSet,
    io::{Read, Seek},
};

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
pub trait FromProperties<C: Seek + Read> {
    fn from_properties(
        asset: &Asset<C>,
        property: &[Property],
        expected_properties: &mut HashSet<&str>,
    ) -> Result<Self>
    where
        Self: Sized;
}

impl<C: Read + Seek> FromProperty<C> for bool {
    fn from_property(_asset: &Asset<C>, property: &Property) -> Result<Self> {
        match property {
            Property::BoolProperty(property) => Ok(property.value),
            _ => bail!("{property:#?}"),
        }
    }
}

impl<C: Read + Seek> FromProperty<C> for f32 {
    fn from_property(_asset: &Asset<C>, property: &Property) -> Result<Self> {
        match property {
            Property::FloatProperty(property) => Ok(property.value.0),
            _ => bail!("{property:#?}"),
        }
    }
}

impl<C: Read + Seek> FromProperty<C> for i32 {
    fn from_property(_asset: &Asset<C>, property: &Property) -> Result<Self> {
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

impl<C: Read + Seek, T: FromProperty<C>> FromProperty<C> for Option<T> {
    fn from_property(asset: &Asset<C>, property: &Property) -> Result<Self> {
        Ok(Some(T::from_property(asset, property)?))
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

pub fn property_or_default_notify<C: Read + Seek, T: Default + FromProperty<C>>(
    asset: &Asset<C>,
    properties: &[Property],
    name: &'static str,
    expected_properties: &mut HashSet<&str>,
) -> Result<T> {
    expected_properties.insert(name);
    if let Some(property) = properties
        .iter()
        .find(|p| p.get_name().get_content(|c| c == name))
    {
        T::from_property(asset, property)
    } else {
        Ok(T::default())
    }
}

pub fn checked_read<C: Read + Seek, T: Default + FromProperties<C>>(
    asset: &Asset<C>,
    properties: &[Property],
) -> Result<T> {
    let mut expected_properties = ::std::collections::HashSet::new();
    let res = FromProperties::from_properties(asset, properties, &mut expected_properties)?;
    for p in properties {
        dbg!(&expected_properties);
        p.get_name().get_content(|c| {
            ::anyhow::ensure!(
                expected_properties.contains(&c),
                "unread property: {c:?} {properties:?}"
            );
            Ok(())
        })?;
    }
    Ok(res)
}
