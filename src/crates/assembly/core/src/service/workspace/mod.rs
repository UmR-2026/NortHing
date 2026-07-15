//! Workspace service module
//!
//! Full workspace management system: open, manage, scan, statistics, etc.

pub(crate) mod accessors;
pub(crate) mod admin;
mod admin_discovery;
mod admin_migration;
pub(crate) mod factory;
pub(crate) mod identity_watch;
pub(crate) mod lifecycle;
pub(crate) mod manager;
pub(crate) mod manager_accessors;
pub(crate) mod manager_lifecycle;
pub(crate) mod provider;
pub(crate) mod service;
mod service_init;
mod service_invoke;
mod service_state;
mod service_types;
pub(crate) mod types;
pub(crate) mod update;
pub(crate) mod workspace_info_impl;

mod facade;

pub use facade::*;
