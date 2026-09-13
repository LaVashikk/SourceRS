#[cfg(feature = "intern_keys")]
use std::sync::{Arc, LazyLock};

#[cfg(feature = "intern_keys")]
static KEY_POOL: LazyLock<dashmap::DashSet<Arc<str>>> = LazyLock::new(|| dashmap::DashSet::new());

#[cfg(feature = "intern_keys")]
pub type VmtKey = Arc<str>;

#[cfg(not(feature = "intern_keys"))]
pub type VmtKey = String;

/// Interns a string into a global pool if "intern_keys" is enabled.
/// Converts the key to lowercase for normalized lookups.
pub fn intern_key(s: &str) -> VmtKey {
    #[cfg(feature = "intern_keys")]
    {
        // First try to find the key without allocating a new lowercase string.
        if let Some(existing) = KEY_POOL.get(s) {
            return Arc::clone(&existing);
        }

        let lower = s.to_lowercase();
        // Check again after lowercasing (in case it wasn't lowercase)
        if let Some(existing) = KEY_POOL.get(lower.as_str()) {
            return Arc::clone(&*existing);
        }

        let arc: Arc<str> = Arc::from(lower);
        KEY_POOL.insert(Arc::clone(&arc));
        arc
    }
    #[cfg(not(feature = "intern_keys"))]
    {
        s.to_lowercase()
    }
}
