// Copyright (C) 2025 Andrew Wason
// SPDX-License-Identifier: GPL-3.0-or-later
use std::{
    ffi::{CStr, CString},
    slice,
};

pub struct L0ttiePlugin {
    animation_path: CString,
    video_fps: f64,
    mode: Mode,
    width: usize,
    height: usize,
    player: dotlottie_rs::DotLottiePlayer,
    frame_number: f32,
    frame_step: f32,
    direction: Direction,
    loop_animation: bool,
    initialized: bool,
}

impl frei0r_rs2::Plugin for L0ttiePlugin {
    type Kind = frei0r_rs2::KindSource;

    const PARAMS: &'static [frei0r_rs2::param::ParamInfo<Self>] = &[
        frei0r_rs2::param::ParamInfo::new_string(
            c"animation_path",
            c"Lottie animation file path",
            |plugin| plugin.animation_path.as_c_str(),
            |plugin, value| plugin.animation_path = value.to_owned(),
        ),
        frei0r_rs2::param::ParamInfo::new_double(
            c"video_fps",
            c"Framerate of the generated video",
            |plugin| plugin.video_fps,
            |plugin, value| plugin.video_fps = value,
        ),
        frei0r_rs2::param::ParamInfo::new_string(
            c"mode",
            c"Playback mode: 'forward' (default), 'reverse', 'bounce', 'reverse-bounce'",
            |plugin| plugin.mode.into(),
            |plugin, value| {
                plugin.mode = Mode::from(value);
            }
        ),
        frei0r_rs2::param::ParamInfo::new_bool(
            c"loop",
            c"Loop animation",
            |plugin| plugin.loop_animation,
            |plugin, value| {
                plugin.loop_animation = value;
            }
        ),
        frei0r_rs2::param::ParamInfo::new_string(
            c"fit",
            c"Fit animation to video frame: 'contain' (default), 'fill', 'cover', 'fit-width', 'fit-height', 'none'",
            |plugin| Fit(plugin.player.config().layout.fit).into(),
            |plugin, value| {
                let mut config = plugin.player.config();
                config.layout.fit = Fit::from(value).0;
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
            player: dotlottie_rs::DotLottiePlayer::new(dotlottie_rs::Config {
                autoplay: true,
                layout: dotlottie_rs::Layout::new(dotlottie_rs::Fit::Contain, vec![0.5, 0.5]),
                ..dotlottie_rs::Config::default()
            }),
            frame_step: 0.0,
            frame_number: 0.0,
            mode: Mode::Forward,
            direction: Direction::Forward,
            loop_animation: false,
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

        self.player.set_frame(self.frame_number);
        if !self.player.render() {
            println!("l0ttie render failed frame {}", self.frame_number);
            return;
        }

        (self.frame_number, self.direction) = self.mode.next_frame(
            self.frame_number,
            self.frame_step,
            self.direction,
            0.0,
            self.player.total_frames(),
            self.loop_animation,
        );

        let frame = unsafe {
            &slice::from_raw_parts(self.player.buffer(), self.player.buffer_len() as usize)
                [0..(self.width * self.height)]
        };
        outframe.copy_from_slice(frame);
        // XXX no rotation seems correct, but it should be 8
        #[cfg(any())]
        for pixel in outframe {
            // Rotate left by 8 bits: ARGB -> RGBA
            // dotlottie_rs::ColorSpace::ARGB8888 -> frei0r_rs2::ColorModel::RGBA8888
            *pixel = pixel.rotate_left(8);
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum Direction {
    Forward,
    Reverse,
}

#[derive(Copy, Clone, Debug)]
enum Mode {
    Forward,
    Reverse,
    Bounce,
    ReverseBounce,
}

impl Mode {
    fn next_frame(
        &self,
        current_frame: f32,
        frame_step: f32,
        direction: Direction,
        start_frame: f32,
        end_frame: f32,
        loop_animation: bool,
    ) -> (f32, Direction) {
        let next_frame = match direction {
            Direction::Forward => current_frame + frame_step,
            Direction::Reverse => current_frame - frame_step,
        };

        match self {
            Mode::Forward => {
                if next_frame >= end_frame {
                    if loop_animation {
                        (start_frame, direction)
                    } else {
                        (end_frame, direction)
                    }
                } else {
                    (next_frame, direction)
                }
            }
            Mode::Reverse => {
                if next_frame <= start_frame {
                    if loop_animation {
                        (end_frame, direction)
                    } else {
                        (start_frame, direction)
                    }
                } else {
                    (next_frame, direction)
                }
            }
            Mode::Bounce => match direction {
                Direction::Forward => {
                    if next_frame >= end_frame {
                        (end_frame, Direction::Reverse)
                    } else {
                        (next_frame, direction)
                    }
                }
                Direction::Reverse => {
                    if next_frame <= start_frame {
                        if loop_animation {
                            (start_frame, Direction::Forward)
                        } else {
                            (start_frame, direction)
                        }
                    } else {
                        (next_frame, direction)
                    }
                }
            },
            Mode::ReverseBounce => match direction {
                Direction::Reverse => {
                    if next_frame <= start_frame {
                        (start_frame, Direction::Forward)
                    } else {
                        (next_frame, direction)
                    }
                }
                Direction::Forward => {
                    if next_frame >= end_frame {
                        if loop_animation {
                            (end_frame, Direction::Reverse)
                        } else {
                            (end_frame, direction)
                        }
                    } else {
                        (next_frame, direction)
                    }
                }
            },
        }
    }
}

const MODE_FORWARD: &CStr = c"forward";
const MODE_REVERSE: &CStr = c"reverse";
const MODE_BOUNCE: &CStr = c"bounce";
const MODE_REVERSE_BOUNCE: &CStr = c"reverse-bounce";
impl From<&CStr> for Mode {
    fn from(value: &CStr) -> Self {
        if value == MODE_FORWARD {
            Mode::Forward
        } else if value == MODE_REVERSE {
            Mode::Reverse
        } else if value == MODE_BOUNCE {
            Mode::Bounce
        } else if value == MODE_REVERSE_BOUNCE {
            Mode::ReverseBounce
        } else {
            Mode::Forward
        }
    }
}
impl From<Mode> for &'static CStr {
    fn from(mode: Mode) -> Self {
        match mode {
            Mode::Forward => MODE_FORWARD,
            Mode::Reverse => MODE_REVERSE,
            Mode::Bounce => MODE_BOUNCE,
            Mode::ReverseBounce => MODE_REVERSE_BOUNCE,
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct Fit(dotlottie_rs::Fit);
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

frei0r_rs2::plugin!(L0ttiePlugin);
