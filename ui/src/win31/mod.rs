//! A software renderer that reproduces the look of the original Windows 3.1 program pixel for pixel.
//!
//! The game's windows are *data* (the Delphi forms, exported by `tools/dfm.py`), its fonts and bitmaps belong to the
//! game and to Windows 3.1, so none of them are in this repository: `tools/ui/skin.py` builds a *skin pack* from the
//! user's own copies (`analysis/out/skin/`), and this module draws forms with it into a 16-bit style frame buffer
//! (the original ran at 800x600 with 65536 colours).
pub mod draw;
pub mod fb;
pub mod form;
pub mod json;
pub mod skin;

pub use fb::Fb;
pub use form::{Form, Node};
pub use skin::Skin;
