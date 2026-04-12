# source-kv

A straightforward `serde` implementation for parsing and serializing Valve's textual Key-Values formats (such as `.vmf`, `.vmt`, `gameinfo.txt`, etc.).
## Example

Add this to your `Cargo.toml`:

```toml
[dependencies]
source_kv = "0.1.0"
serde = { version = "1.0", features = ["derive"] }
````

### Parsing a basic Map Entity

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct MapData {
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

fn main() {
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

    let parsed_entity: MapData = source_kv::from_str(vmf_data).unwrap();
    println!("{:#?}", parsed_entity.entities);
}
```

Repeated keys (very common in VMF) are supported using `Vec<T>` with `#[serde(rename = "...")]`.

More examples are available in the [`examples/`](examples/) directory.

## API

- `source_kv::from_str` — deserialize from `&str`
- `source_kv::to_string` — serialize to `String`

## License
MIT License.
