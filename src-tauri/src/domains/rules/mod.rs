// Purpose: Product-domain module for Rules file operations and default-rule injection behavior.

use crate::platform::config as app_config;

mod commands;
mod defaults;
mod diff;
mod effective_targets;
mod files;
mod injection;
mod management;
mod types;

pub use commands::*;
pub use types::*;

pub(crate) use defaults::*;
pub(crate) use diff::*;
pub(crate) use effective_targets::*;
pub(crate) use files::*;
pub(crate) use injection::*;
pub(crate) use management::*;
