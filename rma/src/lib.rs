#![allow(special_module_name)]
pub mod rma;
pub mod room_features;

#[cfg(target_arch = "wasm32")]
mod main;

use std::{
    fs,
    io::{Cursor, Read, Seek},
    path::Path,
};

use anyhow::Result;
use rma::RoomGenerator;
use rma_lib::FromExport;
use three_d::{Context, CpuMesh, PhysicalMaterial};
use unreal_asset::{
    engine_version::EngineVersion, exports::ExportBaseTrait, types::PackageIndex, Asset,
};

pub struct RMAContext<'c> {
    pub context: &'c Context,
    pub wireframe_material: PhysicalMaterial,
    pub wireframe_mesh: CpuMesh,
}

pub enum AppMode {
    Gallery { paths: Vec<String> },
    Editor { path: String },
}

pub fn read_asset<P: AsRef<Path>>(
    path: P,
    version: EngineVersion,
) -> Result<Asset<Cursor<Vec<u8>>>> {
    let uasset = Cursor::new(fs::read(&path)?);
    let uexp = Cursor::new(fs::read(path.as_ref().with_extension("uexp"))?);
    let asset = Asset::new(uasset, Some(uexp), version, None, false)?;

    Ok(asset)
}

pub fn read_rma<C: Read + Seek>(asset: Asset<C>) -> Result<RoomGenerator> {
    let root = asset
        .asset_data
        .exports
        .iter()
        .enumerate()
        .find_map(|(i, export)| {
            (export.get_base_export().outer_index.index == 0)
                .then(|| PackageIndex::from_export(i as i32).unwrap())
        })
        .unwrap();

    RoomGenerator::from_export(&asset, root)
}

// Entry point for wasm
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub async fn start() -> Result<(), JsValue> {
    console_log::init_with_level(log::Level::Debug).unwrap();

    use log::info;
    info!("Logging works!");

    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    wasm_main().await.ok();

    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub async fn wasm_main() -> Result<()> {
    use rma_lib::list_dir;

    let mode = AppMode::Gallery {
        paths: list_dir!("assets/rma")
            .into_iter()
            .filter_map(|p| p.strip_suffix(".uasset").map(|p| p.to_string()))
            .collect(),
    };

    main::run(mode)?;

    Ok(())
}
