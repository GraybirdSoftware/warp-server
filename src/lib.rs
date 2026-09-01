//! A self-hostable implementation of the Binary Ninja WARP server API.
//!
//! The HTTP surface mirrors the public server's OpenAPI document
//! (`https://warp.binary.ninja/api/openapi.json`) so the stock Binary Ninja
//! WARP plugin can talk to it unmodified.

pub mod auth;
pub mod bootstrap;
pub mod config;
pub mod db;
pub mod error;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod startup;
pub mod telemetry;
