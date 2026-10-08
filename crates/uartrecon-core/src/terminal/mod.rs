//! Terminal: model command, klasifikasi risiko, dan console buffer.

pub mod commands;
pub mod console;

pub use commands::{Command, CommandRisk, classify_risk};
pub use console::{ConsoleBuffer, OutputMode};
