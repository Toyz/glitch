#![deny(clippy::perf, clippy::correctness)]
#![warn(rust_2018_idioms, clippy::complexity, clippy::nursery)]

pub mod bounds;
pub mod eval;
pub mod parser;
pub mod rgb;
pub mod token;

pub use eval::EvalContext;
pub use token::Token;
pub use rgb::Rgb;
