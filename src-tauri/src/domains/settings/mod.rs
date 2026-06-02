// Purpose: Product-domain module for General settings behavior.

use crate::platform::config as app_config;

mod commands;
mod preferences;

pub use commands::*;

pub(crate) use preferences::*;
