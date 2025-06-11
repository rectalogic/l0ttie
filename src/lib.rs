// Copyright (C) 2025 Andrew Wason
// SPDX-License-Identifier: GPL-3.0-or-later
use dotlottie_rs::{Config, DotLottiePlayer, Fit, Layout};
use std::{
    ffi::{CStr, CString},
    slice,
};

pub struct L0ttiePlugin {
    animation_path: CString,
    video_fps: f64,
    width: usize,
    height: usize,
    player: DotLottiePlayer,
    frame: f32,
    frame_step: f32,
    initialized: bool,
}

impl frei0r_rs2::Plugin for L0ttiePlugin {
    type Kind = frei0r_rs2::KindSource;

    const PARAMS: &'static [frei0r_rs2::ParamInfo<Self>] = &[
        frei0r_rs2::ParamInfo::new_string(
            c"animation_path",
            c"Lottie animation file path",
            |plugin| plugin.animation_path.as_c_str(),
            |plugin, value| plugin.animation_path = value.to_owned(),
        ),
        frei0r_rs2::ParamInfo::new_double(
            c"video_fps",
            c"Framerate of the generated video",
            |plugin| plugin.video_fps,
            |plugin, value| plugin.video_fps = value,
        ),
        frei0r_rs2::ParamInfo::new_string(
            c"fit",
            c"Fit animation to video frame: 'contain' (default), 'fill', 'cover', 'fit-width', 'fit-height', 'none'",
            |plugin| fit_to_cstr(plugin.player.config().layout.fit),
            |plugin, value| {
                let mut config = plugin.player.config();
                config.layout.fit = cstr_to_fit(value);
                plugin.player.set_config(config);
            }
        ),
    ];

    fn info() -> frei0r_rs2::PluginInfo {
        frei0r_rs2::PluginInfo {
            name: c"l0ttie",
            author: c"Andrew Wason",
            color_model: frei0r_rs2::ColorModel::RGBA8888,
            major_version: env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap(),
            minor_version: env!("CARGO_PKG_VERSION_MINOR").parse().unwrap(),
            explanation: Some(c"Lottie renderer using dotlottie-rs"),
        }
    }

    fn new(width: usize, height: usize) -> Self {
        Self {
            animation_path: c"".into(),
            video_fps: 30.0,
            width,
            height,
            player: DotLottiePlayer::new(Config {
                //XXX set mode, speed, layout etc
                // XXX what is segment? start/end times
                // XXX marker is basically a named segment (start/end)
                // mode: Mode::Bounce,
                autoplay: true,
                // loop_animation: true,
                layout: Layout::new(Fit::Contain, vec![0.5, 0.5]),
                ..Config::default()
            }),
            frame_step: 0.0,
            frame: 0.0,
            initialized: false,
        }
    }
}

impl frei0r_rs2::SourcePlugin for L0ttiePlugin {
    fn update_source(&mut self, _time: f64, outframe: &mut [u32]) {
        if !self.initialized {
            self.initialized = true;
            if let Ok(animation_path) = self.animation_path.to_str() {
                let loaded = if animation_path.ends_with(".lottie") {
                    if let Ok(data) = std::fs::read(animation_path) {
                        self.player.load_dotlottie_data(
                            &data,
                            self.width as u32,
                            self.height as u32,
                        )
                    } else {
                        false
                    }
                } else {
                    self.player.load_animation_path(
                        animation_path,
                        self.width as u32,
                        self.height as u32,
                    )
                };
                if loaded {
                    let player_fps = self.player.total_frames() / self.player.duration();
                    self.frame_step = player_fps / self.video_fps as f32;
                } else {
                    eprintln!("Failed to load lottie animation path {animation_path}");
                }
            } else {
                eprintln!("Invalid lottie animation path");
            }
        }
        if !self.player.is_loaded() {
            return;
        }

        // XXX validate frame is valid. also 0.0 is current but has never been rendered
        self.player.set_frame(self.frame);
        if self.player.render() {
            self.frame += self.frame_step;
            // https://github.com/LottieFiles/dotlottie-rs/issues/335
            let frame = unsafe {
                &slice::from_raw_parts(self.player.buffer(), self.player.buffer_len() as usize)
                    [0..(self.width * self.height)]
            };
            outframe.copy_from_slice(frame);
            for pixel in outframe {
                // Rotate left by 8 bits: ARGB -> RGBA
                // dotlottie_rs::ColorSpace::ARGB8888 -> frei0r_rs2::ColorModel::RGBA8888
                // XXX no rotation seems correct, but it should be 8
                *pixel = pixel.rotate_left(0);
            }
        } else {
            println!("RENDER FAILED {}", self.frame);
        }
    }
}

const FIT_CONTAIN: &CStr = c"contain";
const FIT_FILL: &CStr = c"fill";
const FIT_COVER: &CStr = c"cover";
const FIT_WIDTH: &CStr = c"fit-width";
const FIT_HEIGHT: &CStr = c"fit-height";
const FIT_NONE: &CStr = c"none";

fn fit_to_cstr(fit: Fit) -> &'static CStr {
    match fit {
        Fit::Contain => FIT_CONTAIN,
        Fit::Fill => FIT_FILL,
        Fit::Cover => FIT_COVER,
        Fit::FitWidth => FIT_WIDTH,
        Fit::FitHeight => FIT_HEIGHT,
        Fit::None => FIT_NONE,
    }
}

fn cstr_to_fit(fit: &CStr) -> Fit {
    if fit == FIT_CONTAIN {
        Fit::Contain
    } else if fit == FIT_FILL {
        Fit::Fill
    } else if fit == FIT_COVER {
        Fit::Cover
    } else if fit == FIT_WIDTH {
        Fit::FitWidth
    } else if fit == FIT_HEIGHT {
        Fit::FitHeight
    } else if fit == FIT_NONE {
        Fit::None
    } else {
        Fit::Contain
    }
}

frei0r_rs2::plugin!(L0ttiePlugin);
