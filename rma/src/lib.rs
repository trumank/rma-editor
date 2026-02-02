#![allow(special_module_name)]
pub mod convert;
pub mod debug_lines;
pub mod fly_control;
pub mod objects;
pub mod room_features;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CameraMode {
    #[default]
    Orbit,
    Fly,
}

#[cfg(target_arch = "wasm32")]
mod main;

use std::path::Path;

use anyhow::Result;
use asset_ser::core::object_pool::{ObjectHandle, ObjectPool};
use asset_ser::loader::asset_loader;
use three_d::{Context, CpuMesh, PhysicalMaterial};

pub struct RMAContext<'c> {
    pub context: &'c Context,
    pub wireframe_material: PhysicalMaterial,
    pub wireframe_mesh: CpuMesh,
}

pub enum AppMode {
    Gallery { paths: Vec<String> },
    Editor { path: String },
}

/// Load an RMA asset using asset_ser
pub fn load_rma_asset(path: &Path, pool: &mut ObjectPool) -> Result<ObjectHandle> {
    let handle = asset_loader::load_asset(path, pool)?;
    Ok(handle)
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
