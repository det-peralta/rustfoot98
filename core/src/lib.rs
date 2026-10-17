//! Elifoot 98 core. Pure, dependency-free, `wasm32`-friendly.
//!
//! Every rule here was verified against the original `ELIFOOT.EXE`; see `docs/match-engine.md`.
pub mod chance;
pub mod cipher;
pub mod eft;
pub mod evolution;
pub mod outcome;
pub mod rng;
pub mod sector;
pub mod txe;
