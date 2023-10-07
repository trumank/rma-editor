use std::io::{Read, Seek};

use anyhow::{bail, Result};
pub use rma_proc::*;
use unreal_asset::{properties::Property, types::PackageIndex, Asset};

pub trait HeapSize {
    /// Total number of bytes of heap memory owned by `self`.
    ///
    /// Does not include the size of `self` itself, which may or may not be on
    /// the heap. Includes only children of `self`, meaning things pointed to by
    /// `self`.
    fn heap_size_of_children(&self) -> usize;
}

//
// In a real version of this library there would be lots more impls here, but
// here are some interesting ones.
//

impl HeapSize for u8 {
    /// A `u8` does not own any heap memory.
    fn heap_size_of_children(&self) -> usize {
        0
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
