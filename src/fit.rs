// Copyright (C) 2025 Andrew Wason
// SPDX-License-Identifier: GPL-3.0-or-later

use std::ffi::CStr;

#[derive(Copy, Clone, Debug)]
pub(crate) struct Fit(pub dotlottie_rs::Fit);
const FIT_CONTAIN: &CStr = c"contain";
const FIT_FILL: &CStr = c"fill";
const FIT_COVER: &CStr = c"cover";
const FIT_WIDTH: &CStr = c"fit-width";
const FIT_HEIGHT: &CStr = c"fit-height";
const FIT_NONE: &CStr = c"none";
impl From<&CStr> for Fit {
    fn from(value: &CStr) -> Self {
        let fit = if value == FIT_CONTAIN {
            dotlottie_rs::Fit::Contain
        } else if value == FIT_FILL {
            dotlottie_rs::Fit::Fill
        } else if value == FIT_COVER {
            dotlottie_rs::Fit::Cover
        } else if value == FIT_WIDTH {
            dotlottie_rs::Fit::FitWidth
        } else if value == FIT_HEIGHT {
            dotlottie_rs::Fit::FitHeight
        } else if value == FIT_NONE {
            dotlottie_rs::Fit::None
        } else {
            dotlottie_rs::Fit::Contain
        };
        Fit(fit)
    }
}
impl From<Fit> for &'static CStr {
    fn from(fit: Fit) -> Self {
        match fit.0 {
            dotlottie_rs::Fit::Contain => FIT_CONTAIN,
            dotlottie_rs::Fit::Fill => FIT_FILL,
            dotlottie_rs::Fit::Cover => FIT_COVER,
            dotlottie_rs::Fit::FitWidth => FIT_WIDTH,
            dotlottie_rs::Fit::FitHeight => FIT_HEIGHT,
            dotlottie_rs::Fit::None => FIT_NONE,
        }
    }
}
