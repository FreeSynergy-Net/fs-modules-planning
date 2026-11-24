//! `fs-modules-planning`: the planning module as a service — one process per person, the contract `tasks.capnp` (`Items` for the
//! cards, `Boards` for the columns) on a Unix socket through fs-rpc, over the store of `fs-tasks-core`.
//!
//! The actor of a request is the user of the connection (fs-rpc reads it from the system), never a field of a message. Storage
//! stays behind the `TaskItems` and `Boards` traits of the core: nothing here knows which store holds the tasks. The command
//! catalog (command id to contract call) is the data file `catalog/commands.json` of this repository, read by the client.
//!
//! Interface status: draft (ADR 0045). Windows: the transport is fs-rpc's named pipe; the tests are written to run there too.
//!
//! Pattern: Facade — this file only declares modules and re-exports.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
// The generated `Server` traits are async for every method, also those that never wait.
#![allow(clippy::missing_errors_doc, clippy::unused_async_trait_impl)]

mod args;
mod boards;
mod both;
mod cards;
mod errors;
mod facts;
mod formsource;
mod items;
mod level2;
mod level2_calls;
mod level2_watch;
mod place;
#[cfg(test)]
mod place_tests;
mod root;
mod scope;
mod serve;
mod wire;
mod write;
#[cfg(test)]
mod write_tests;

pub use formsource::{FORM_CARDS, FORM_NAME, LEVEL, PROVIDER};
pub use root::{CAPABILITIES, CONTRACT_VERSION, Planning, actor_of};
pub use serve::{PROGRAM, serve_command, serve_with};
