/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! A glyph rasterizer for webrender
//!
//! ## Overview
//!
//! ## Usage
//!

#[cfg(any(target_os = "macos", target_os = "windows"))]
mod gamma_lut;
mod rasterizer;
mod telemetry;
mod types;

pub mod profiler;

pub use rasterizer::*;
pub use types::*;

#[macro_use]
extern crate malloc_size_of_derive;
#[macro_use]
extern crate tracy_rs;
#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate smallvec;

#[cfg(any(feature = "serde"))]
#[macro_use]
extern crate serde;

extern crate malloc_size_of;

#[cfg(feature = "backend_native")]
pub mod platform {
    #[cfg(target_os = "macos")]
    pub use crate::platform::macos::font;
    #[cfg(any(target_os = "android", all(unix, not(target_os = "macos"))))]
    pub use crate::platform::unix::font;
    #[cfg(target_os = "windows")]
    pub use crate::platform::windows::font;

    #[cfg(target_os = "macos")]
    pub mod macos {
        pub mod font;
    }
    #[cfg(any(target_os = "android", all(unix, not(target_os = "macos"))))]
    pub mod unix {
        pub mod font;
    }
    #[cfg(target_os = "windows")]
    pub mod windows {
        pub mod font;
    }
}

#[cfg(not(feature = "backend_native"))]
pub mod backend {
    #[cfg(feature = "backend_swash")]
    pub use crate::backend::swash::font;
    #[cfg(feature = "backend_fontdue")]
    pub use crate::backend::fontdue::font;

    #[cfg(feature = "backend_swash")]
    pub mod swash {
        pub mod font;
    }

    #[cfg(feature = "backend_fontdue")]
    pub mod fontdue {
        pub mod font;
    }
}
