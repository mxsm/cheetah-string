mod construct;
mod convert;
mod pattern;
mod query;
mod repr;
mod traits;

pub use pattern::{SplitPattern, SplitStr, StrPattern};
use repr::InnerString;

/// Immutable string value with inline, static, or shared backing.
///
/// All clones are allocation-free. Use `CheetahBuilder` or `String` while
/// contents are still being mutated.
#[derive(Clone)]
#[repr(transparent)]
pub struct CheetahString {
    inner: InnerString,
}
