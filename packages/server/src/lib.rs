//! Server-side implementations: database access and operation logic for
//! CookIt's HTTP endpoints. Reused by the server function bodies in `api`.
pub mod auth;
pub mod db;
pub mod middleware;
pub mod ops;
