use anyhow::{Context, Result};
use asset_ser::{
    AssetVersionInfo, archive::reader::AssetArchiveReader, core::object_path::ObjectPath,
    core::object_pool::ObjectPool, get_package_index_path, parse_legacy_asset,
    util::printer::ObjectPrinter,
};
use clap::{Parser, Subcommand};
use rayon::prelude::*;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Mutex;
use uesave::read_properties_until_none;

#[derive(Parser, Debug)]
#[command(name = "asset_ser")]
#[command(about = "Parse UE4/UE5 legacy assets (.uasset + .uexp)", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Parse a single asset file
    Parse {
        /// Path to the .uasset file
        #[arg(value_name = "ASSET_PATH")]
        asset_path: PathBuf,

        /// Path to jmap file for dependency resolution (optional)
        #[arg(short, long)]
        jmap: PathBuf,

        /// Pretty print JSON output
        #[arg(short, long)]
        pretty: bool,

        /// Show detailed information about the asset structure
        #[arg(short, long)]
        verbose: bool,

        /// Use object printer to print root object instead of JSON output
        #[arg(long)]
        print: bool,
    },
    /// Parse all assets in a directory recursively and tally failures by class
    Tally {
        /// Directory to scan for .uasset files
        #[arg(value_name = "DIRECTORY")]
        directory: PathBuf,

        /// Pretty print JSON output
        #[arg(short, long)]
        pretty: bool,

        /// Show detailed information during processing
        #[arg(short, long)]
        verbose: bool,
    },
}

#[derive(Serialize)]
struct LoadedObjectInfo {
    path: String,
    name: String,
    class_path: ObjectPath,
    template_path: Option<ObjectPath>,
    outer_path: Option<ObjectPath>,
    properties: serde_json::Value,
}

#[derive(Serialize)]
struct AssetInfo {
    asset_path: String,
    version: VersionOutput,
    total_objects: usize,
    objects: Vec<LoadedObjectInfo>,
}

#[derive(Serialize)]
struct VersionOutput {
    package_file_version_ue4: u32,
    package_file_version_ue5: u32,
    engine_version_major: u16,
    engine_version_minor: u16,
    engine_version_patch: u16,
}

impl From<&AssetVersionInfo> for VersionOutput {
    fn from(v: &AssetVersionInfo) -> Self {
        Self {
            package_file_version_ue4: v.package_file_version_ue4,
            package_file_version_ue5: v.package_file_version_ue5,
            engine_version_major: v.engine_version_major,
            engine_version_minor: v.engine_version_minor,
            engine_version_patch: v.engine_version_patch,
        }
    }
}

#[derive(Serialize)]
struct TallyOutput {
    total_assets_scanned: usize,
    total_exports_processed: usize,
    total_exports_failed: usize,
    failures_by_class: HashMap<ObjectPath, ClassFailureStats>,
}

#[derive(Serialize)]
struct ClassFailureStats {
    total_exports: usize,
    failed_exports: usize,
    failure_rate: f64,
    example_assets: Vec<String>,
}

fn find_uasset_files(dir: &PathBuf) -> Result<Vec<PathBuf>> {
    let mut uasset_files = Vec::new();

    fn visit_dirs(dir: &PathBuf, uasset_files: &mut Vec<PathBuf>) -> Result<()> {
        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    visit_dirs(&path, uasset_files)?;
                } else if path.extension().and_then(|s| s.to_str()) == Some("uasset") {
                    uasset_files.push(path);
                }
            }
        }
        Ok(())
    }

    visit_dirs(dir, &mut uasset_files)?;
    Ok(uasset_files)
}

fn process_tally(directory: PathBuf, pretty: bool, verbose: bool) -> Result<()> {
    let uasset_files = find_uasset_files(&directory)?;

    if verbose {
        eprintln!("Found {} .uasset files", uasset_files.len());
    }

    let total_exports_processed = Mutex::new(0usize);
    let total_exports_failed = Mutex::new(0usize);
    let class_stats: Mutex<HashMap<ObjectPath, (usize, usize, Vec<String>)>> =
        Mutex::new(HashMap::new());

    uasset_files
        .par_iter()
        .enumerate()
        .for_each(|(file_idx, asset_path)| {
            if verbose {
                eprintln!(
                    "[{}/{}] Processing: {}",
                    file_idx + 1,
                    uasset_files.len(),
                    asset_path.display()
                );
            }

            // Parse the asset header
            let header = match parse_legacy_asset(asset_path) {
                Ok(h) => h,
                Err(e) => {
                    if verbose {
                        eprintln!("  Failed to parse asset header: {}", e);
                    }
                    return;
                }
            };

            // Read the .uasset and .uexp files
            let uasset_data = match fs::read(asset_path) {
                Ok(d) => d,
                Err(e) => {
                    if verbose {
                        eprintln!("  Failed to read .uasset file: {}", e);
                    }
                    return;
                }
            };
            let uasset_size = uasset_data.len();

            let uexp_path = asset_path.with_extension("uexp");
            let uexp_data = match fs::read(&uexp_path) {
                Ok(d) => d,
                Err(e) => {
                    if verbose {
                        eprintln!("  Failed to read .uexp file: {}", e);
                    }
                    return;
                }
            };

            // Process each export
            for export_idx in 0..header.exports.len() {
                *total_exports_processed.lock().unwrap() += 1;
                let export = &header.exports[export_idx];

                // Get class name
                let class_name = match get_package_index_path(&header, export.class_index) {
                    Ok(name) => name,
                    Err(_) => "Unknown".into(),
                };

                // Calculate export data offset in .uexp file
                let export_start = (export.serial_offset as usize).saturating_sub(uasset_size);
                let export_end = export_start + export.serial_size as usize;

                let mut parse_failed = false;

                if export_end > uexp_data.len() {
                    parse_failed = true;
                } else {
                    let export_data = &uexp_data[export_start..export_end];

                    // Use LoadedObjectArchive with empty pool
                    let pool = ObjectPool::new();
                    let mut archive =
                        AssetArchiveReader::new(Cursor::new(export_data), &header, &pool);
                    archive.log = false;
                    archive.error_to_raw = false;

                    // Try to parse properties
                    if read_properties_until_none(&mut archive).is_err() {
                        parse_failed = true;
                    }
                }

                // Update stats
                let mut stats = class_stats.lock().unwrap();
                let entry = stats
                    .entry(class_name.clone())
                    .or_insert((0, 0, Vec::new()));
                entry.0 += 1; // total exports for this class
                if parse_failed {
                    entry.1 += 1; // failed exports for this class
                    *total_exports_failed.lock().unwrap() += 1;

                    // Add example asset (limit to 5 examples)
                    if entry.2.len() < 5 {
                        entry.2.push(asset_path.display().to_string());
                    }
                }
            }
        });

    // Build output
    let mut failures_by_class = HashMap::new();
    let class_stats = class_stats.into_inner().unwrap();
    for (class_name, (total, failed, examples)) in class_stats {
        if failed > 0 {
            failures_by_class.insert(
                class_name,
                ClassFailureStats {
                    total_exports: total,
                    failed_exports: failed,
                    failure_rate: (failed as f64 / total as f64) * 100.0,
                    example_assets: examples,
                },
            );
        }
    }

    let tally_output = TallyOutput {
        total_assets_scanned: uasset_files.len(),
        total_exports_processed: total_exports_processed.into_inner().unwrap(),
        total_exports_failed: total_exports_failed.into_inner().unwrap(),
        failures_by_class,
    };

    // Output as JSON
    if pretty {
        println!("{}", serde_json::to_string_pretty(&tally_output)?);
    } else {
        println!("{}", serde_json::to_string(&tally_output)?);
    }

    Ok(())
}

fn process_parse(
    asset_path: PathBuf,
    jmap_path: PathBuf,
    pretty: bool,
    verbose: bool,
    print: bool,
) -> Result<()> {
    use asset_ser::loader::asset_loader;

    // Parse the asset header
    let header = parse_legacy_asset(&asset_path).context("Failed to parse asset")?;
    let version = AssetVersionInfo::from_package_header(&header);

    if verbose {
        eprintln!("Asset: {:?}", asset_path);
        eprintln!("Package Version UE4: {}", version.package_file_version_ue4);
        eprintln!("Package Version UE5: {}", version.package_file_version_ue5);
        eprintln!(
            "Engine Version: {}.{}.{}",
            version.engine_version_major,
            version.engine_version_minor,
            version.engine_version_patch
        );
        eprintln!("Total Exports: {}", header.exports.len());
        eprintln!();
    }

    // Create object pool
    let mut pool = ObjectPool::new();

    if verbose {
        eprintln!("Loading jmap from {:?}", jmap_path);
    }

    let jmap_data = fs::read_to_string(&jmap_path)
        .with_context(|| format!("Failed to read jmap file: {:?}", jmap_path))?;
    let _jmap: jmap::Jmap =
        serde_json::from_str(&jmap_data).context("Failed to parse jmap JSON")?;

    if verbose {
        eprintln!("Loading asset with dependency resolution...");
    }

    let root_handles = asset_loader::load_asset_all_roots(&asset_path, &mut pool)
        .context("Failed to load asset with dependencies")?;

    if verbose {
        eprintln!("Loaded {} objects", pool.len());
        eprintln!("Found {} root objects", root_handles.len());
    }

    // If print flag is set, use ObjectPrinter
    if print {
        let mut printer = ObjectPrinter::new(&pool);

        // FIXME temp
        // let root_handles = pool.iter().map(|h| h.0).collect::<Vec<_>>();

        for (idx, root_handle) in root_handles.iter().enumerate() {
            if root_handles.len() > 1 {
                println!("\n=== Root Object {} ===", idx + 1);
            }
            let output = printer
                .print_object(*root_handle)
                .context("Failed to print object")?;
            print!("{}", output);
        }
        return Ok(());
    }

    // Build output from pool
    let mut objects = Vec::new();
    for (handle, loaded_obj) in pool.iter() {
        let path = pool.build_path(handle);

        let class_path = pool.resolve_path(&loaded_obj.class);

        let template_path = loaded_obj.template.as_ref().map(|r| pool.resolve_path(r));

        let outer_path = loaded_obj.outer.as_ref().map(|r| pool.resolve_path(r));

        objects.push(LoadedObjectInfo {
            path: path.to_string(),
            name: loaded_obj.name.to_string(),
            class_path,
            template_path,
            outer_path,
            properties: serde_json::to_value(loaded_obj.properties())?,
        });
    }

    let asset_info = AssetInfo {
        asset_path: asset_path.display().to_string(),
        version: (&version).into(),
        total_objects: objects.len(),
        objects,
    };

    // Output as JSON
    if pretty {
        println!("{}", serde_json::to_string_pretty(&asset_info)?);
    } else {
        println!("{}", serde_json::to_string(&asset_info)?);
    }

    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Parse {
            asset_path,
            jmap,
            pretty,
            verbose,
            print,
        } => process_parse(asset_path, jmap, pretty, verbose, print),
        Commands::Tally {
            directory,
            pretty,
            verbose,
        } => process_tally(directory, pretty, verbose),
    }
}
