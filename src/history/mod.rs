mod reader;
mod types;
mod writer;

// validate_for_revert: TODO(feature-60) - revert safety validation
#[allow(unused_imports)]
pub use reader::{read_history, validate_for_revert};
pub use types::*;
pub use writer::{write_history, HistoryError};
