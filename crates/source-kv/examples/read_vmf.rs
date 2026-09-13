#![allow(dead_code)]
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct Vmf {
    versioninfo: VersionInfo,
    viewsettings: ViewSettings,
    world: World,
    #[serde(rename = "entity", default)]
    entities: Vec<Entity>,
}

#[derive(Debug, Deserialize)]
struct VersionInfo {
    editorversion: i32,
    mapversion: i32,
    formatversion: i32,
    prefab: bool,
}

#[derive(Debug, Deserialize)]
struct ViewSettings {
    #[serde(rename = "bSnapToGrid")]
    snap_to_grid: bool,
    #[serde(rename = "bShowGrid")]
    show_grid: bool,
    #[serde(rename = "nGridSpacing")]
    grid_spacing: i32,
}

#[derive(Debug, Deserialize)]
struct World {
    id: i32,
    mapversion: i32,
    classname: String,
    skyname: String,
    #[serde(rename = "solid", default)]
    solids: Vec<Solid>,
}

#[derive(Debug, Deserialize)]
struct Entity {
    id: i32,
    classname: String,
    origin: Option<String>, // Some entities might not have origin
    #[serde(rename = "solid", default)]
    solids: Vec<Solid>,
}

#[derive(Debug, Deserialize)]
struct Solid {
    id: i32,
    #[serde(rename = "side", default)]
    sides: Vec<Side>,
}

#[derive(Debug, Deserialize)]
struct Side {
    id: i32,
    plane: String,
    material: String,
    uaxis: String,
    vaxis: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new("examples/data/smoll.vmf");
    let content = fs::read_to_string(path)?;

    println!("Reading VMF from: {:?}", path);

    let vmf: Vmf = source_kv::from_str(&content)?;

    println!("Map Version: {}", vmf.versioninfo.mapversion);
    println!("Skyname: {}", vmf.world.skyname);
    println!("World Solids: {}", vmf.world.solids.len());
    println!("Entities: {}", vmf.entities.len());

    for (i, ent) in vmf.entities.iter().enumerate() {
        println!("Entity {}: {} (ID: {})", i, ent.classname, ent.id);
        if !ent.solids.is_empty() {
            println!("  Solids: {}", ent.solids.len());
            for solid in &ent.solids {
                println!("    Solid ID: {} has {} sides", solid.id, solid.sides.len());
            }
        }
    }

    Ok(())
}
