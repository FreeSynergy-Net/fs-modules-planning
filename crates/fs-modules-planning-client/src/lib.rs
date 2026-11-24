//! `fs-modules-planning-client`: the board of the planning module as the host needs it — columns, add card, move card, add column,
//! and what can be done (from the capabilities the service reports). No type of the contract leaves this crate.
//!
//! The host LINKS this crate for now (transition for K2/K3); loading modules at run time replaces that later. The command catalog
//! (`catalog/commands.json` of the module repository) says which call of the contract each command of a frontend description runs.
//!
//! Interface status: draft (ADR 0045).
//!
//! Pattern: Facade — this file only declares modules and re-exports.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]

mod catalog;
mod columns;
mod ensure;
mod model;
mod planning;
mod wire;

pub use catalog::{Catalog, Command, Param, Requirement};
pub use ensure::{
    SERVICE_PROGRAM, SOCKET_PROGRAM, default_socket, ensure_service, ensure_service_found,
    find_service_program,
};
pub use model::{Card, Column, Error, Kind, Operation};
pub use planning::Planning;
