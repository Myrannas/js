#![forbid(unsafe_code)]

mod builtins;
mod context;
mod debugging;
mod function;
mod ops;
mod primordials;
mod result;
mod values;
mod vm;

extern crate ahash;
extern crate anyhow;
extern crate builtin;
extern crate colored;
extern crate instruction_set;
extern crate rand;

pub use function::{BuiltIn, JsFunction};
pub use primordials::GlobalThis;
pub use result::{ExecutionError, InternalError};
pub use values::object::JsObject;
pub use values::string::JsPrimitiveString;
pub use values::value::RuntimeValue;
pub use vm::JsThread;