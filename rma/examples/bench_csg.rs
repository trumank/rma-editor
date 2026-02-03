//! Benchmark CSG/SDF mesh generation performance
//!
//! Run with: cargo run --release --example bench_csg -- <room.json>
//! Profile with: cargo flamegraph --release --example bench_csg -- <room.json>

use std::path::PathBuf;
use std::time::Instant;

use rma::scene::csg_mesh::{VisibilityCheck, build_csg_from_features, csg_to_three_d_mesh};

struct AllVisible;

impl VisibilityCheck for AllVisible {
    fn is_visible(&self, _path: &[usize]) -> bool {
        true
    }
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("RMA_Motherlode_Center_03.json"));

    println!("Loading room from: {}", path.display());
    let load_start = Instant::now();
    let room = rma::load_room(&path)?;
    let load_time = load_start.elapsed();
    println!("  Load time: {:?}", load_time);

    let feature_count = count_features(&room.room_features);
    println!("  Features: {}", feature_count);

    println!("\nGenerating CSG mesh...");
    let csg_start = Instant::now();
    let csg = build_csg_from_features(&room, &AllVisible);
    let csg_time = csg_start.elapsed();
    println!("  CSG generation time: {:?}", csg_time);

    if let Some(ref csg) = csg {
        println!("  Polygons: {}", csg.polygons.len());

        println!("\nConverting to mesh...");
        let mesh_start = Instant::now();
        let mesh = csg_to_three_d_mesh(csg);
        let mesh_time = mesh_start.elapsed();
        println!("  Mesh conversion time: {:?}", mesh_time);

        let vertex_count = match &mesh.positions {
            three_d::Positions::F32(v) => v.len(),
            three_d::Positions::F64(v) => v.len(),
        };
        let index_count = match &mesh.indices {
            three_d::Indices::U8(i) => i.len(),
            three_d::Indices::U16(i) => i.len(),
            three_d::Indices::U32(i) => i.len(),
            three_d::Indices::None => 0,
        };
        println!("  Vertices: {}, Indices: {}", vertex_count, index_count);

        println!(
            "\n=== Total time: {:?} ===",
            load_time + csg_time + mesh_time
        );
    } else {
        println!("  No mesh features found");
    }

    Ok(())
}

fn count_features(features: &[rma::objects::URoomFeature]) -> usize {
    features
        .iter()
        .fold(0, |acc, f| acc + 1 + count_features(&f.children))
}
