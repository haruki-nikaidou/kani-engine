//! Tag-handler sub-systems.
//!
//! The `poll_interpreter` system emits every passthrough `KagEvent::Tag` as an
//! [`EvTagRouted`].  Each handler module reads `EvTagRouted`, destructures the
//! inner [`ResolvedTag`] by variant, and emits a strongly-typed action event.
//! Tags not matched by any built-in handler are available as
//! `ResolvedTag::Extension` — game-specific code matches on that variant.

pub mod audio;
pub mod chara;
pub mod effect;
pub mod image;
pub mod message;
pub mod transition;
