// Purpose: Crate-level ownership roots for the Modus backend.

pub(crate) mod adapters;
pub mod app;
pub(crate) mod commands;
pub(crate) mod domains;
pub mod platform;
pub mod scenario;

pub use app::run;
