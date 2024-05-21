use ordered_float::OrderedFloat;
pub use rma_proc::*;

use std::{
    collections::{HashMap, HashSet},
    io::{Read, Seek},
};

use anyhow::{bail, Context, Result};
use unreal_asset::{
    exports::BaseExport,
    properties::{
        array_property::ArrayProperty,
        int_property::{BoolProperty, FloatProperty, IntProperty},
        Property, PropertyDataTrait,
    },
    types::{FName, PackageIndex},
    unversioned::Ancestry,
    Asset, Export,
};

pub struct ImportChain<'a> {
    pub outer: Option<&'a ImportChain<'a>>,
    pub class_package: &'a str,
    pub class_name: &'a str,
    pub object_name: &'a str,
}

pub fn get_import<R: Read + Seek>(asset: &mut Asset<R>, import: &ImportChain) -> PackageIndex {
    let outer = import
        .outer
        .map(|outer| get_import(asset, outer))
        .unwrap_or_default();
    let existing = &asset
        .imports
        .iter()
        .enumerate()
        .find(|(_, ai)| {
            ai.class_package.get_content(|n| n == import.class_package)
                && ai.class_name.get_content(|n| n == import.class_name)
                && ai.object_name.get_content(|n| n == import.object_name)
                && ai.outer_index == outer
        })
        .map(|(index, _)| PackageIndex::from_import(index as i32).unwrap());
    existing.unwrap_or_else(|| {
        let new_import = unreal_asset::Import::new(
            asset.add_fname(import.class_package),
            asset.add_fname(import.class_name),
            outer,
            asset.add_fname(import.object_name),
            false,
        );
        asset.add_import(new_import)
    })
}

pub fn from_object_property<C: Read + Seek, T: FromExport<C>>(
    asset: &Asset<C>,
    property: &Property,
) -> Result<T> {
    match property {
        Property::ObjectProperty(property) => T::from_export(asset, property.value),
        _ => bail!("wrong property type"),
    }
}

pub fn resolve_package_index<C: Read + Seek>(
    asset: &Asset<C>,
    package_index: PackageIndex,
) -> Result<&Export> {
    asset
        .get_export(package_index)
        .with_context(|| format!("package index does not point to an export {package_index:?}"))
}

pub trait FromExport<C: Seek + Read> {
    fn from_export(asset: &Asset<C>, package_index: PackageIndex) -> Result<Self>
    where
        Self: Sized;
}
pub trait ToExport<C: Seek + Read> {
    fn to_export(&self, ctx: &mut CtxSer<C>) -> Result<PackageIndex>;
}
pub trait BaseExportGetter<C: Seek + Read> {
    fn base_export(&self, ctx: &mut CtxSer<C>) -> Result<BaseExport>;
}
pub trait FromProperty<C: Seek + Read> {
    fn from_property(asset: &Asset<C>, property: &Property) -> Result<Self>
    where
        Self: Sized;
}
pub trait ToProperty<C: Seek + Read> {
    fn get_type() -> Option<&'static str>;
    fn to_property(
        &self,
        ctx: &mut CtxSer<C>,
        name: FName,
        ancestry: Ancestry,
    ) -> Result<Option<Property>>;
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
pub trait ToProperties<C: Seek + Read> {
    fn to_properties(&self, ctx: &mut CtxSer<C>, ancestry: Ancestry) -> Result<Vec<Property>>;
}
pub struct CtxSer<'a, C: Seek + Read> {
    pub asset: &'a mut Asset<C>,
    pub name_counter: &'a mut NameCounter,
    pub new_exports: Vec<PackageIndex>,
    pub serialization_before_create_dependencies: Vec<PackageIndex>,
}
#[derive(Default)]
pub struct NameCounter {
    names: HashMap<String, i32>,
}
impl NameCounter {
    pub fn next(&mut self, name: &str) -> i32 {
        let num = self.names.entry(name.to_string()).or_default();
        *num += 1;
        *num
    }
}

impl<'a, C: Seek + Read> CtxSer<'a, C> {
    pub fn new(asset: &'a mut Asset<C>, name_counter: &'a mut NameCounter) -> Self {
        Self {
            asset,
            name_counter,
            new_exports: vec![],
            serialization_before_create_dependencies: vec![],
        }
    }
    pub fn ser_dep(&mut self, pi: PackageIndex) -> PackageIndex {
        self.serialization_before_create_dependencies.push(pi);
        pi
    }
}

/// Useful for ignoring properties
impl<C: Read + Seek> FromProperty<C> for () {
    fn from_property(_asset: &Asset<C>, _property: &Property) -> Result<Self> {
        Ok(())
    }
}
impl<C: Read + Seek> ToProperty<C> for () {
    fn get_type() -> Option<&'static str> {
        todo!()
    }

    fn to_property(
        &self,
        ctx: &mut CtxSer<C>,
        name: FName,
        ancestry: Ancestry,
    ) -> Result<Option<Property>> {
        Ok(None)
    }
}

impl<C: Read + Seek> FromProperty<C> for bool {
    fn from_property(_asset: &Asset<C>, property: &Property) -> Result<Self> {
        match property {
            Property::BoolProperty(property) => Ok(property.value),
            _ => bail!("{property:#?}"),
        }
    }
}
impl<C: Read + Seek> ToProperty<C> for bool {
    fn get_type() -> Option<&'static str> {
        todo!()
    }
    fn to_property(
        &self,
        _ctx: &mut CtxSer<C>,
        name: FName,
        ancestry: Ancestry,
    ) -> Result<Option<Property>> {
        if *self == Self::default() {
            return Ok(None);
        }
        Ok(Some(
            BoolProperty {
                name,
                ancestry,
                property_guid: None,
                duplication_index: 0,
                value: *self,
            }
            .into(),
        ))
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

impl<C: Read + Seek> FromProperty<C> for OrderedFloat<f32> {
    fn from_property(_asset: &Asset<C>, property: &Property) -> Result<Self> {
        match property {
            Property::FloatProperty(property) => Ok(property.value.0.into()),
            _ => bail!("{property:#?}"),
        }
    }
}
impl<C: Read + Seek> ToProperty<C> for OrderedFloat<f32> {
    fn get_type() -> Option<&'static str> {
        todo!()
    }
    fn to_property(
        &self,
        _ctx: &mut CtxSer<C>,
        name: FName,
        ancestry: Ancestry,
    ) -> Result<Option<Property>> {
        //if *self == Self::default() {
        //    return Ok(None);
        //}
        Ok(Some(
            FloatProperty {
                name,
                ancestry,
                property_guid: None,
                duplication_index: 0,
                value: self.0.into(),
            }
            .into(),
        ))
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
impl<C: Read + Seek> ToProperty<C> for i32 {
    fn get_type() -> Option<&'static str> {
        todo!()
    }
    fn to_property(
        &self,
        _ctx: &mut CtxSer<C>,
        name: FName,
        ancestry: Ancestry,
    ) -> Result<Option<Property>> {
        if *self == Self::default() {
            return Ok(None);
        }
        Ok(Some(
            IntProperty {
                name,
                ancestry,
                property_guid: None,
                duplication_index: 0,
                value: *self,
            }
            .into(),
        ))
    }
}

impl<C: Read + Seek, T: FromProperty<C>> FromProperty<C> for Vec<T> {
    fn from_property(asset: &Asset<C>, property: &Property) -> Result<Self> {
        let mut values = vec![];
        match property {
            Property::ArrayProperty(property) => {
                for value in &property.value {
                    match value {
                        Property::ObjectProperty(obj) if 0 == obj.value.index => {
                            continue; // TODO hack to omit null objects from arrays
                        }
                        _ => {}
                    }
                    values.push(T::from_property(asset, value)?);
                }
            }
            _ => bail!("wrong property type"),
        }
        Ok(values)
    }
}

impl<C: Read + Seek, T: ToProperty<C>> ToProperty<C> for Vec<T> {
    fn get_type() -> Option<&'static str> {
        todo!()
    }
    fn to_property(
        &self,
        ctx: &mut CtxSer<C>,
        name: FName,
        ancestry: Ancestry,
    ) -> Result<Option<Property>> {
        if self.is_empty() {
            return Ok(None);
        }
        let t = T::get_type();
        dbg!((t, name.get_owned_content()));
        let values = self
            .iter()
            .enumerate()
            .map(|(i, v)| {
                v.to_property(
                    ctx,
                    if t == Some("StructProperty") {
                        name.clone()
                    } else {
                        FName::new_dummy(i.to_string(), i32::MIN)
                    },
                    ancestry.clone(),
                )
                .transpose()
                .expect("non-empty array entries")
            })
            .collect::<Result<Vec<_>>>()?;
        let t = T::get_type().map(|t| ctx.asset.add_fname(t));
        Ok(Some(
            ArrayProperty::from_arr(name, ancestry, t, values).into(),
        ))
    }
}

impl<C: Read + Seek, T: FromProperty<C>> FromProperty<C> for Option<T> {
    fn from_property(asset: &Asset<C>, property: &Property) -> Result<Self> {
        Ok(Some(T::from_property(asset, property)?))
    }
}
impl<C: Read + Seek, T: ToProperty<C>> ToProperty<C> for Option<T> {
    fn get_type() -> Option<&'static str> {
        todo!()
    }
    fn to_property(
        &self,
        ctx: &mut CtxSer<C>,
        name: FName,
        ancestry: Ancestry,
    ) -> Result<Option<Property>> {
        self.as_ref()
            .map(|s| s.to_property(ctx, name, ancestry).transpose())
            .flatten()
            .transpose()
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
