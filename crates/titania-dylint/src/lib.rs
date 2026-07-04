//! `titania_dylint` — a Dylint plugin cdylib for the Titania CI lane.
//!
//! This crate is the stub shared library that Dylint will load at runtime
//! to execute Titania's custom lint rules. No lint logic is implemented here;
//! the crate exists to establish the cdylib target and crate name.

#![forbid(unsafe_code)]
