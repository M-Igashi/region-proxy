pub mod preferences;
pub mod regions;

pub use preferences::{Preferences, DEFAULT_INSTANCE_TYPE, DEFAULT_PORT};
pub use regions::{region_name, require_region, REGIONS};
