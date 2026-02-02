use crate::{
    AssetVersionInfo,
    archive::property_schema::PropertySchemaProvider,
    core::{
        object_path::ObjectPath,
        object_pool::{AssetArchiveType, ObjectRef},
    },
    saver::object_ref_resolver::ObjectRefResolver,
};
use anyhow::Result;
use byteorder::{LE, WriteBytesExt};
use jmap::Jmap;
use retoc::legacy_asset::FPackageNameMap;
use std::io::{Seek, Write};
use uesave::{ArchiveWriter, Error, PropertyTagPartial, Scope, VersionInfo};

pub struct AssetArchiveWriter<'a, W: Write + Seek> {
    stream: W,
    version: AssetVersionInfo,
    resolver: &'a mut ObjectRefResolver,
    name_map: &'a mut FPackageNameMap,
    schema_provider: PropertySchemaProvider<'a>,
    current_struct: ObjectPath,
    scope: Scope,
    log: bool,
}

impl<'a, W: Write + Seek> AssetArchiveWriter<'a, W> {
    pub fn new(
        stream: W,
        version: AssetVersionInfo,
        resolver: &'a mut ObjectRefResolver,
        name_map: &'a mut FPackageNameMap,
        jmap: &'a Jmap,
        current_struct: ObjectPath,
    ) -> Self {
        Self {
            stream,
            version,
            resolver,
            name_map,
            schema_provider: PropertySchemaProvider::new(jmap),
            current_struct,
            scope: Scope::root(),
            log: false,
        }
    }

    /// Get the serialized data
    pub fn into_inner(self) -> W {
        self.stream
    }

    /// Write an FMinimalName (index + number)
    fn write_fname(&mut self, name: &str) -> Result<(), Error> {
        let minimal_name = self.name_map.store(name);
        self.stream
            .write_i32::<LE>(minimal_name.index)
            .map_err(Error::Io)?;
        self.stream
            .write_i32::<LE>(minimal_name.number)
            .map_err(Error::Io)?;
        Ok(())
    }

    /// Write an FPackageIndex
    fn write_package_index(&mut self, object_ref: &ObjectRef) -> Result<(), Error> {
        let pkg_index = self
            .resolver
            .resolve(object_ref)
            .map_err(|e| Error::Other(e.to_string()))?;

        self.stream
            .write_i32::<LE>(pkg_index.index)
            .map_err(Error::Io)?;
        Ok(())
    }
}

impl<W: Write + Seek> Write for AssetArchiveWriter<'_, W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.stream.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.stream.flush()
    }
}

impl<W: Write + Seek> Seek for AssetArchiveWriter<'_, W> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.stream.seek(pos)
    }
}

impl<W: Write + Seek> ArchiveWriter for AssetArchiveWriter<'_, W> {
    type ArchiveType = AssetArchiveType;

    fn version(&self) -> &dyn VersionInfo {
        &self.version
    }

    fn set_version(&mut self, _header: uesave::Header) {
        // Version is fixed for asset serialization
    }

    fn scope(&mut self) -> &mut Scope {
        &mut self.scope
    }

    fn write_string(&mut self, string: &str) -> Result<(), Error> {
        self.write_fname(string)
    }

    fn write_string_trailing(
        &mut self,
        string: &str,
        _trailing: Option<&[u8]>,
    ) -> Result<(), Error> {
        self.write_fname(string)
    }

    fn write_object_ref(&mut self, object_ref: &ObjectRef) -> Result<(), Error> {
        self.write_package_index(object_ref)
    }

    fn write_soft_object_path(&mut self, soft_object_path: &(String, i32)) -> Result<(), Error> {
        // Write the path string as FName
        self.write_fname(&soft_object_path.0)?;
        // Write the subpath index
        self.stream
            .write_i32::<LE>(soft_object_path.1)
            .map_err(Error::Io)?;
        Ok(())
    }

    fn get_schema(&self, path: &str) -> Option<PropertyTagPartial> {
        self.schema_provider
            .get_schema(self.current_struct.as_str(), path)
    }

    fn path(&self) -> String {
        self.scope.path()
    }

    fn log(&self) -> bool {
        self.log
    }
}
