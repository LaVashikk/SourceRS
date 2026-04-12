#![allow(dead_code)]
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct GameInfo {
    #[serde(rename = "GameInfo")]
    info: Info,
}

#[derive(Debug, Deserialize)]
struct Info {
    game: String,
    title: String,
    #[serde(rename = "GameData")]
    game_data: Option<String>,
    gamelogo: Option<i32>, // 1 or 0
    #[serde(rename = "SupportsDX8")]
    supports_dx8: Option<i32>,
    #[serde(rename = "SupportsXbox360")]
    supports_xbox360: Option<i32>,
    #[serde(rename = "FileSystem")]
    file_system: FileSystem,
}

#[derive(Debug, Deserialize)]
struct FileSystem {
    #[serde(rename = "SteamAppId")]
    steam_app_id: i32,
    #[serde(rename = "ToolsAppId")]
    tools_app_id: Option<i32>,
    #[serde(rename = "SearchPaths")]
    search_paths: SearchPaths,
}

#[derive(Debug, Deserialize)]
struct SearchPaths {
    #[serde(rename = "Game", default)]
    game: Vec<String>,
    #[serde(rename = "GameBin", default)]
    game_bin: Vec<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new("examples/data/gameinfo.txt");
    let content = fs::read_to_string(path)?;

    println!("Reading GameInfo from: {:?}", path);

    let game_info: GameInfo = source_kv::from_str(&content)?;

    println!("Game: {}", game_info.info.game);
    println!("Title: {}", game_info.info.title);
    println!("Steam App ID: {}", game_info.info.file_system.steam_app_id);
    println!("Search Paths:");
    for path in game_info.info.file_system.search_paths.game {
        println!("  - Game: {}", path);
    }

    Ok(())
}
