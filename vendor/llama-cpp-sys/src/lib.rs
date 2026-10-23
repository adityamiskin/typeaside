#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]

// Keep transcribe's single ggml provider in the native link graph.
extern crate transcribe_cpp_sys;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
