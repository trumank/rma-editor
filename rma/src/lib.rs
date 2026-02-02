#![allow(special_module_name)]
pub mod convert;
pub mod objects;
pub mod scene;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CameraMode {
    #[default]
    Orbit,
    Fly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderMode {
    #[default]
    Wireframe,
    CsgMesh,
}

#[cfg(target_arch = "wasm32")]
mod main;

use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use anyhow::Result;
use asset_ser::AssetVersionInfo;
use asset_ser::core::object_pool::{ObjectHandle, ObjectPool};
use asset_ser::loader::asset_loader;
use asset_ser::saver::asset_saver::save_asset;
use three_d::{Context, CpuMesh, PhysicalMaterial};

use crate::convert::{load_room_generator, save_room_generator};
use crate::objects::URoomGenerator;

pub struct RMAContext<'c> {
    pub context: &'c Context,
    pub wireframe_material: PhysicalMaterial,
    pub wireframe_mesh: CpuMesh,
}

pub enum AppMode {
    Gallery { paths: Vec<String> },
    Editor { path: String },
}

static RMA_JMAP: &str = include_str!("../jmap/rma.jmap");

/// Load an RMA asset using asset_ser
pub fn load_rma_asset(path: &Path, pool: &mut ObjectPool) -> Result<ObjectHandle> {
    let handle = asset_loader::load_asset(path, pool)?;
    Ok(handle)
}

/// Load a room from a file (supports .json and .uasset)
pub fn load_room(path: &Path) -> Result<URoomGenerator> {
    if path.extension().is_some_and(|ext| ext == "json") {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Ok(serde_json::from_reader(reader)?)
    } else {
        let mut pool = ObjectPool::new();
        let handle = load_rma_asset(path, &mut pool)?;
        load_room_generator(&pool, handle)
    }
}

/// Save a room to a file (supports .json and .uasset)
pub fn save_room(path: &Path, room: &URoomGenerator) -> Result<()> {
    if path.extension().is_some_and(|ext| ext == "json") {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, room)?;
    } else {
        let asset_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Room");
        let package_name = format!("/Game/{}", asset_name);

        let mut pool = ObjectPool::new();
        let handle = save_room_generator(&mut pool, room, None, asset_name)?;

        let jmap: jmap::Jmap = serde_json::from_str(RMA_JMAP)?;

        let version = AssetVersionInfo {
            package_file_version_ue4: 522,
            package_file_version_ue5: 0,
            engine_version_major: 4,
            engine_version_minor: 27,
            engine_version_patch: 0,
        };

        save_asset(path, &pool, vec![handle], version, package_name, &jmap)?;
    }
    Ok(())
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
    // TODO: Update for asset_ser - wasm loading needs different approach
    let mode = AppMode::Gallery {
        paths: vec![], // TODO: restore list_dir macro or use different loading
    };

    main::run(mode)?;

    Ok(())
}
