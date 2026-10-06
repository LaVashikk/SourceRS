use std::path::{Path, PathBuf};

use crate::Error;

/// Directories a [`GameInfoProvider`](crate::providers::GameInfoProvider) resolves paths against.
#[derive(Debug, Clone)]
pub struct GameRoots {
    /// Directory holding `gameinfo.txt`, target of `|gameinfo_path|`
    pub game_dir: PathBuf,
    /// Engine root: plain relative paths and `|all_source_engine_paths|` resolve here.
    /// For sourcemods this is the SDK Base install, not the parent of `game_dir`.
    pub base_dir: PathBuf,
}

impl GameRoots {
    /// `base_dir` defaults to the parent of `game_dir`
    pub fn new(game_dir: &Path, base_dir: Option<&Path>) -> Result<Self, Error> {
        let gameinfo = game_dir.join("gameinfo.txt");
        if !gameinfo.is_file() {
            return Err(Error::GameInfoNotFound(gameinfo));
        }

        let game_dir = game_dir.canonicalize().map_err(|source| Error::Io {
            path: game_dir.to_path_buf(),
            source,
        })?;
        let base_dir = match base_dir {
            Some(base) => base.canonicalize().unwrap_or_else(|_| base.to_path_buf()),
            None => game_dir
                .parent()
                .ok_or_else(|| Error::InvalidGamePath(game_dir.clone()))?
                .to_path_buf(),
        };

        Ok(Self { game_dir, base_dir })
    }

    pub fn gameinfo_path(&self) -> PathBuf {
        self.game_dir.join("gameinfo.txt")
    }

    /// Expands `|gameinfo_path|` / `|all_source_engine_paths|` into an absolute path
    pub fn resolve(&self, location: &str) -> PathBuf {
        let location = location.trim().replace('\\', "/");
        let joined = if let Some(rest) = strip_prefix_ignore_case(&location, "|gameinfo_path|") {
            self.game_dir.join(rest.trim_start_matches('/'))
        } else if let Some(rest) = strip_prefix_ignore_case(&location, "|all_source_engine_paths|") {
            self.base_dir.join(rest.trim_start_matches('/'))
        } else {
            self.base_dir.join(location.trim_start_matches('/'))
        };

        // Drops the `.` in `|gameinfo_path|.` so equal dirs compare equal
        joined.components().collect()
    }
}

/// One entry returned by a [`GameInfoProvider`](crate::providers::GameInfoProvider):
/// the path IDs it is mounted under and its absolute location.
#[derive(Debug, Clone)]
pub struct SearchPath {
    /// Lowercase path IDs, `game+mod` becomes `["game", "mod"]`
    pub ids: Vec<String>,
    /// A directory, a `.vpk`, or a `dir/*` wildcard
    pub path: PathBuf,
}

impl SearchPath {
    /// `ids` in gameinfo form, e.g. `"game+mod"`
    pub fn new(ids: &str, path: PathBuf) -> Self {
        let ids = ids.split('+').map(str::to_ascii_lowercase).collect();
        Self { ids, path }
    }

    pub fn has_id(&self, id: &str) -> bool {
        self.ids.iter().any(|i| i.eq_ignore_ascii_case(id))
    }

    fn is_plain_dir(&self) -> bool {
        !is_vpk(&self.path) && self.path.file_name().is_none_or(|name| name != "*")
    }
}

/// Reads `SearchPaths` from `gameinfo.txt` in priority order, applying the rules
/// the engine's gameinfo loader adds on top (see the header comment Valve ships
/// in every `gameinfo.txt`): each `game` dir gets a `gamebin` at `<dir>/bin`,
/// and the first `game` dir is also `mod`.
pub(crate) fn read_search_paths(roots: &GameRoots) -> Result<Vec<SearchPath>, Error> {
    let gameinfo_path = roots.gameinfo_path();
    let text = std::fs::read_to_string(&gameinfo_path).map_err(|source| Error::Io {
        path: gameinfo_path,
        source,
    })?;
    let pairs = read_block(&text, "SearchPaths").ok_or(Error::GameInfoParseError)?;

    let mut paths = Vec::with_capacity(pairs.len());
    let mut mod_assigned = false;
    for (ids, location) in pairs {
        let mut entry = SearchPath::new(&ids, roots.resolve(&location));
        if !entry.has_id("game") || !entry.is_plain_dir() {
            paths.push(entry);
            continue;
        }

        if !mod_assigned {
            mod_assigned = true;
            if !entry.has_id("mod") {
                entry.ids.push("mod".to_owned());
            }
        }
        let gamebin = SearchPath::new("gamebin", entry.path.join("bin"));
        paths.push(entry);
        paths.push(gamebin);
    }

    Ok(paths)
}

/// Key/value pairs of the first block called `name` (case-insensitive), in file order.
/// Nested blocks inside it are skipped.
pub(crate) fn read_block(text: &str, name: &str) -> Option<Vec<(String, String)>> {
    let tokens = tokenize(text);
    let start = tokens
        .windows(2)
        .position(|pair| pair[0].eq_ignore_ascii_case(name) && pair[1] == "{")?;

    let mut tokens = tokens[start + 2..].iter().copied();
    let mut pairs = Vec::new();
    while let Some(key) = tokens.next() {
        if key == "}" {
            break;
        }
        match tokens.next()? {
            "{" => {
                let mut depth = 1;
                while depth > 0 {
                    match tokens.next()? {
                        "{" => depth += 1,
                        "}" => depth -= 1,
                        _ => {}
                    }
                }
            }
            value => pairs.push((key.to_owned(), value.to_owned())),
        }
    }

    Some(pairs)
}

// Own tokenizer, not source-kv: `source_kv::Value` groups repeated keys into one
// map entry, while SearchPaths line order is mount priority.
fn tokenize(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b if b.is_ascii_whitespace() => i += 1,
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'{' | b'}' => {
                tokens.push(&text[i..i + 1]);
                i += 1;
            }
            b'"' => {
                let start = i + 1;
                let end = text[start..].find('"').map_or(text.len(), |len| start + len);
                tokens.push(&text[start..end]);
                i = end + 1;
            }
            _ => {
                let start = i;
                while i < bytes.len()
                    && !bytes[i].is_ascii_whitespace()
                    && !matches!(bytes[i], b'{' | b'}' | b'"')
                {
                    i += 1;
                }
                let token = &text[start..i];
                if !(token.starts_with('[') && token.ends_with(']')) {
                    tokens.push(token);
                }
            }
        }
    }

    tokens
}

pub(crate) fn is_vpk(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("vpk"))
}

fn strip_prefix_ignore_case<'a>(value: &'a str, prefix: &str) -> Option<&'a str> {
    let head = value.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix).then(|| &value[prefix.len()..])
}
