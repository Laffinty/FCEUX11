//! L5 — scheduling and execution.
//!
//! `scheduler` runs manifest entries through the subprocess adapter;
//! `direct` is the shared in-process execution core used by both the
//! CLI `--direct` mode and the C-ABI `direct_entry`.

pub mod direct;
pub mod scheduler;
