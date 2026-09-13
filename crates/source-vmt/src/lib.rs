pub mod interner;
pub mod vmt;
pub mod block;
pub mod proxies;
pub mod patch;

pub use vmt::Vmt;
pub use interner::{VmtKey, intern_key};
pub use proxies::Proxy;

#[cfg(feature = "material_system")]
pub mod system;
#[cfg(feature = "material_system")]
pub use system::MaterialSystem;

pub use source_kv::Error;
pub use source_kv::Value;
