// Purpose: Product-domain module for tool discovery, management, config proxy, and dashboard aggregation.

use crate::platform::config as app_config;

mod commands;
mod config_proxy;
mod dashboard;
mod discovery;
mod management;
mod types;

pub use commands::*;
pub use types::*;

pub(crate) use config_proxy::*;
pub(crate) use dashboard::*;
pub(crate) use discovery::*;
pub(crate) use management::*;
