#![allow(dead_code)]
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
struct GameConfig {
    #[serde(rename = "Configs")]
    configs: Configs,
}

#[derive(Debug, Deserialize, Serialize)]
struct Configs {
    #[serde(rename = "Games")]
    games: IndexMap<String, Game>,
    #[serde(rename = "SDKVersion")]
    sdk_version: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Game {
    #[serde(rename = "GameDir")]
    game_dir: String,
    #[serde(rename = "Hammer")]
    hammer: HammerConfig, // or we can use IndexMap for that
}

#[derive(Debug, Deserialize, Serialize)]
struct HammerConfig {
    #[serde(rename = "GameData0")]
    game_data0: Option<String>,
    #[serde(rename = "GameExe")]
    game_exe: String,
    // Capture other fields if needed, or ignore unknown
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new("examples/data/GameConfig.txt");
    let content = fs::read_to_string(path)?;

    println!("Reading GameConfig from: {:?}", path);

    let config: GameConfig = source_kv::from_str(&content)?;

    println!("SDK Version: {}", config.configs.sdk_version);
    println!("Found {} games:", config.configs.games.len());

    for (name, game) in &config.configs.games {
        println!("- {}:", name);
        println!("    Game Dir: {}", game.game_dir);
        println!("    Game Exe: {}", game.hammer.game_exe);
    }

    let serialized = source_kv::to_string(&config);
    println!("\nSerialized Config:\n{}", serialized.unwrap());

    Ok(())
}
