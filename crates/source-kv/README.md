# source-kv

A straightforward, high-performance `serde` implementation for parsing and serializing Valve's textual Key-Values formats (such as `.vmf`, `.vmt`, `gameinfo.txt`, `GameConfig.txt`, etc.).

## Key Features

* **Full Serde Integration**: Seamlessly deserialize into custom Rust structs and serialize back using standard `#[derive(Deserialize, Serialize)]` attributes.
* **Duplicate Key Support**: Valve's format heavily relies on duplicate keys (especially in `.vmf` files). `source-kv` natively handles this-just map repeated keys to a `Vec<T>`.
* **Order Preservation**: `Value::Obj` is a plain `Vec` of entries, so the file order (duplicates included) survives a parse and write.
* **Zero-copy parsing**: `from_str` borrows tokens straight from the input and builds no intermediate tree of owned strings; `&str` fields work with `#[serde(borrow)]`.
* **Syntax Tolerance**: Safely handles unquoted keys, C-style line comments (`//`), and implicitly converts integer/boolean strings into native Rust types.
* **Dynamic / Untyped API**: Don't want to write static structs? Parse files directly into a generic `source_kv::Value` (AST) and easily convert specific nodes to typed structs later using `from_value`.

## Quick Start

### 1. Parsing Strongly-Typed Structures (e.g., VMF Entities)

Repeated keys are easily captured using `Vec<T>` alongside `#[serde(rename = "...")]`.

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct MapData {
    // VMF files can contain multiple 'entity' blocks
    #[serde(rename = "entity", default)]
    entities: Vec<Entity>,
}

#[derive(Debug, Deserialize)]
struct Entity {
    id: u32,
    classname: String,
    origin: Option<String>,
    #[serde(default)]
    connections: Vec<Connection>,
}

#[derive(Debug, Deserialize)]
struct Connection {
    target: String,
    action: String,
}

fn main() -> Result<(), source_kv::Error> {
    let vmf_data = r#"
    entity
    {
        "id" "1337"
        "classname" "prop_physics"
        "origin" "0 0 64"
        
        "connections"
        {
            "target" "door_1"
            "action" "Open"
        }
    }
    "#;

    let parsed: MapData = source_kv::from_str(vmf_data)?;
    println!("{:#?}", parsed.entities);
    
    Ok(())
}
```

### 2. Using the Dynamic AST (`source_kv::Value`)

Sometimes you don't know the exact structure upfront. You can parse the document into an untyped tree and deserialize specific parts on demand.

```rust
use serde::Deserialize;
use source_kv::{Value, from_value};

#[derive(Debug, Deserialize)]
struct ToolConfig {
    #[serde(rename = "GameExe")]
    game_exe: String,
}

fn main() -> Result<(), source_kv::Error> {
    let input = r#"
        "Configs"
        {
            "Games"
            {
                "Portal 2"
                {
                    "GameExe" "portal2.exe"
                }
            }
        }
    "#;

    // Parse into an untyped Value tree
    let root: Value = input.parse()?;

    // Traverse the AST natively
    if let Some(portal_config) = root.get("Configs")
        .and_then(|c| c.get("Games"))
        .and_then(|g| g.get("Portal 2")) 
    {
        // Convert the specific untyped block into a typed Rust struct
        let config: ToolConfig = from_value(portal_config)?;
        println!("Executable: {}", config.game_exe);
    }

    Ok(())
}
```

## API Overview

- `source_kv::from_str` - Deserialize a KeyValues string directly into a typed struct `T`.
- `source_kv::to_string` - Serialize a typed struct `T` back into KeyValues format.
- `source_kv::Value` - An enum representing the AST: `Str(String)` or `Obj(Vec<(String, Value)>)`, with case-insensitive getters like `.get(key)`, `.get_all(key)` and `.get_string(key)`. Parse a whole document with `text.parse::<Value>()`.
- `source_kv::from_value` - Convert an existing AST `Value` node (owned or borrowed) into a Serde-compatible struct `T`.
- `source_kv::to_value` - Serialize a typed struct `T` into a `Value`.

## License
MIT License.