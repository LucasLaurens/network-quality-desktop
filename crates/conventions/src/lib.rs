//! The house rules a machine can check, so they hold without anyone remembering them.

#![forbid(unsafe_code)]

mod method_names;

pub use method_names::find_misnamed_functions;
