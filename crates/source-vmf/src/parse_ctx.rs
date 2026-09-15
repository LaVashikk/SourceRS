//! Parser context for recording non-fatal syntax warnings and defaults during VMF parsing.

use crate::VmfBlock;

/// Maximum number of warnings stored in [`ParseCtx`] before suppressing further entries.
const MAX_WARNINGS: usize = 1000;

/// Parsing context tracking the current block hierarchy and diagnostic warnings.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ParseCtx {
    /// Path segments of the block being parsed, e.g. `["world", "solid[3]", "side[1]"]`.
    path: Vec<String>,
    /// Warnings recorded during parsing, prefixed with their block path.
    pub warnings: Vec<String>,
    /// Number of warnings suppressed after reaching [`MAX_WARNINGS`].
    pub suppressed: usize,
}

impl ParseCtx {
    /// Enters a nested block. Must be paired with [`ParseCtx::leave`].
    pub(crate) fn enter(&mut self, segment: impl Into<String>) {
        self.path.push(segment.into());
    }

    /// Enters the `index`-th nested block of a repeated kind, e.g. `side[2]`.
    pub(crate) fn enter_indexed(&mut self, segment: &str, index: usize) {
        self.path.push(format!("{}[{}]", segment, index));
    }

    /// Leaves the current block.
    pub(crate) fn leave(&mut self) {
        self.path.pop();
    }

    /// Records a problem at the current path, up to [`MAX_WARNINGS`].
    pub(crate) fn warn(&mut self, msg: impl std::fmt::Display) {
        if self.warnings.len() < MAX_WARNINGS {
            self.warnings.push(format!("{}: {}", self.path.join("/"), msg));
        } else {
            self.suppressed += 1;
        }
    }

    /// Records a missing key that was replaced by `default`.
    pub(crate) fn warn_missing(&mut self, key: &str, default: impl std::fmt::Display) {
        self.warn(format!("missing '{}', using '{}'", key, default));
    }

    /// Records a value that could not be parsed and was replaced by `default`.
    pub(crate) fn warn_unparsable(&mut self, key: &str, value: &str, default: impl std::fmt::Display) {
        self.warn(format!(
            "key '{}' has unparsable value '{}', using '{}'",
            key, value, default
        ));
    }
}

/// Converts a [`VmfBlock`] into a typed structure, recording warnings in [`ParseCtx`].
pub trait FromBlock: Sized {
    /// Parses `block`, substituting defaults and logging warnings for missing or invalid values.
    fn from_block(block: VmfBlock, ctx: &mut ParseCtx) -> Self;
}

/// Implements `TryFrom<VmfBlock>` for types implementing [`FromBlock`].
macro_rules! impl_try_from_block {
    ($($t:ty),* $(,)?) => {$(
        impl TryFrom<$crate::VmfBlock> for $t {
            type Error = $crate::VmfError;

            fn try_from(block: $crate::VmfBlock) -> $crate::VmfResult<Self> {
                let mut ctx = $crate::parse_ctx::ParseCtx::default();
                Ok(<$t as $crate::parse_ctx::FromBlock>::from_block(block, &mut ctx))
            }
        }
    )*};
}

pub(crate) use impl_try_from_block;
