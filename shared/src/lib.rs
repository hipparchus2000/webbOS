#![no_std]
#![feature(doc_auto_cfg)]

//! WebbOS Shared Library
//! 
//! Common types and structures shared between bootloader and kernel.

pub mod bootinfo;
pub mod types;

pub use types::*;
