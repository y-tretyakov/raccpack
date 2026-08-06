//! Versioned file caches for facade use-cases.
//!
//! Currently only the sniff report cache ([`try_load_sniff_cache`],
//! [`store_sniff_cache`]); future phases add their own backends here.

mod sniff_cache;

pub use sniff_cache::{store_sniff_cache, try_load_sniff_cache};
