//! Startup for the Raspberry Pi 3 (BCM2837), compatible with QNX OS 8.0.
//!
//! The board's part of startup: what the BCM2837 and the Raspberry Pi
//! firmware are, on top of the generic `oscraft-startup` library.

#![cfg_attr(not(test), no_std)]

#[cfg(target_arch = "aarch64")]
mod board;
#[cfg(target_arch = "aarch64")]
mod hwinfo;
#[cfg(target_arch = "aarch64")]
mod intr;
#[cfg(target_arch = "aarch64")]
mod mbox;
#[cfg(target_arch = "aarch64")]
mod miniuart;
#[cfg(target_arch = "aarch64")]
mod pm;
mod revision;
