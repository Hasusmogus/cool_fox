//! What became of a patch site: shared by `frame_rate` (Windows only) and
//! `resolution` (portable apart from the writing).

/// What became of a site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Patched {
    Done,
    /// Its bytes are not the ones expected: left alone.
    Differs,
    Failed(&'static str),
}
