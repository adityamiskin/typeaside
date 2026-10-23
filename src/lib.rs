pub mod web;

pub mod audio;
pub mod autostart;
mod cleanup;
pub mod config;
mod context;
pub mod corrections;
pub mod credentials;
pub mod engine;
pub mod hotkeys;
pub mod learn;
pub mod paths;
pub mod replacements;
pub mod text_injector;
pub mod transcribe;

#[doc(hidden)]
pub mod cues;
pub mod http;
mod media;
pub mod pipeline;
mod snippets;
pub mod update;

pub mod local;

pub mod notes;
