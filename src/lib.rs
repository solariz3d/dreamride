//! Luminesce engine (headless): music written as mathematical functions of time.
//!
//! - [`expr`]: the expression language (parse once into a tree, evaluate per sample).
//! - [`expressive`]: performance shaping (phrase arch, ritard, melodic charge, vibrato).
//! - [`song`]: the song file format (settings, definitions, voices).
//! - [`render`]: synthesis and mixing into a stereo buffer.
//! - [`effects`]: reverb (Freeverb), ping-pong delay, tape wow and warmth on the mix.
//! - [`wav`]: the WAV writer (16-bit PCM or 32-bit float).
//! - [`stream`]: stream mode, an endless song rendered stage by stage ahead of the playhead.
//! - [`ffi`]: the C ABI the native world loads songs through (`luminesce.dll`).
//!
//! The format is documented in README.md.

pub mod compose;
pub mod effects;
pub mod error;
pub mod expr;
pub mod expressive;
pub mod ffi;
pub mod render;
pub mod song;
pub mod stream;
pub mod wav;

pub use error::Error;
