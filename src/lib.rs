#![allow(ambiguous_glob_reexports)]
#![allow(clippy::should_implement_trait)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::type_complexity)]

#[path = "core/mod.rs"]
pub mod core;

#[path = "ui/mod.rs"]
pub mod ui;

pub use core::*;

pub use ui::*;
