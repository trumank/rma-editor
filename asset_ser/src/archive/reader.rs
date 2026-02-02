use crate::AssetVersionInfo;
use crate::core::object_pool::{AssetArchiveType, ObjectPool, ObjectRef};
use byteorder::{LE, ReadBytesExt};
use retoc::legacy_asset::{FLegacyPackageHeader, FMinimalName};
use retoc::zen::FPackageIndex;
use std::io::{Read, Seek};
use uesave::{ArchiveReader, Error, PropertyTagPartial, Scope, StructType, VersionInfo};

pub struct AssetArchiveReader<'a, R: Read + Seek> {
    stream: R,
    version: AssetVersionInfo,
    package_header: &'a FLegacyPackageHeader,
    pool: &'a ObjectPool,
    scope: Scope,
    pub log: bool,
    pub error_to_raw: bool,
}

impl<'a, R: Read + Seek> AssetArchiveReader<'a, R> {
    pub fn new(stream: R, package_header: &'a FLegacyPackageHeader, pool: &'a ObjectPool) -> Self {
        let version = AssetVersionInfo::from_package_header(package_header);

        Self {
            stream,
            version,
            package_header,
            pool,
            scope: Scope::root(),
            log: true,
            error_to_raw: false,
        }
    }
}

impl<'a, R: Read + Seek> AssetArchiveReader<'a, R> {
    pub fn read_fname(&mut self) -> Result<String, Error> {
        Ok(self
            .package_header
            .name_map
            .get(FMinimalName {
                index: self.stream.read_i32::<LE>()?,
                number: self.stream.read_i32::<LE>()?,
            })
            .map_err(|e| Error::Other(e.to_string()))?
            .to_string())
    }

    pub fn read_package_index(&mut self) -> Result<ObjectRef, Error> {
        let index = self.stream.read_i32::<LE>()?;
        let pkg_idx = FPackageIndex { index };

        // Convert to ObjectRef
        let path = crate::get_package_index_path(self.package_header, pkg_idx)
            .map_err(|e| Error::Other(e.to_string()))?;

        // Check if this object is already loaded
        if let Some(handle) = self.pool.find_by_path(&path) {
            Ok(ObjectRef::loaded(handle))
        } else {
            // Not loaded yet - keep as path reference
            Ok(ObjectRef::unloaded(path))
        }
    }
}

impl<'a, R: Read + Seek> Read for AssetArchiveReader<'a, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.stream.read(buf)
    }
}

impl<'a, R: Read + Seek> Seek for AssetArchiveReader<'a, R> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.stream.seek(pos)
    }
}

impl<'a, R: Read + Seek> ArchiveReader for AssetArchiveReader<'a, R> {
    type ArchiveType = AssetArchiveType;

    fn version(&self) -> &dyn VersionInfo {
        &self.version
    }

    fn scope(&mut self) -> &mut Scope {
        &mut self.scope
    }

    fn get_type_or(&mut self, default: &StructType) -> Result<StructType, Error> {
        Ok(default.clone())
    }

    fn read_string(&mut self) -> Result<String, Error> {
        self.read_fname()
    }

    fn read_string_trailing(&mut self) -> Result<(String, Vec<u8>), Error> {
        Ok((self.read_fname()?, Vec::new()))
    }

    fn read_object_ref(&mut self) -> Result<ObjectRef, Error> {
        self.read_package_index()
    }

    fn read_soft_object_path(&mut self) -> Result<(String, i32), Error> {
        Ok((self.read_fname()?, self.read_i32::<LE>()?))
    }

    fn record_schema(&mut self, _path: String, _tag: PropertyTagPartial) {}

    fn path(&self) -> String {
        self.scope.path()
    }

    fn log(&self) -> bool {
        self.log
    }

    fn error_to_raw(&self) -> bool {
        self.error_to_raw
    }
}
