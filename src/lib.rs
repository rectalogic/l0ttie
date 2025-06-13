// Copyright (C) 2025 Andrew Wason
// SPDX-License-Identifier: GPL-3.0-or-later
use std::{
    error::Error,
    ffi::{CStr, CString},
};

use dotlottie_rs::{Animation, ColorSpace, Drawable, Renderer, TvgError};

pub struct L0ttiePlugin {
    animation_path: CString,
    video_fps: f64,
    mode: Mode,
    width: usize,
    height: usize,
    renderer: dotlottie_rs::TvgRenderer,
    animation: dotlottie_rs::TvgAnimation,
    layout: dotlottie_rs::Layout,
    recompute_layout: bool,
    frame_number: f32,
    frame_step: f32,
    direction: Direction,
    loop_animation: bool,
    initialized: bool,
    loaded: bool,
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
            |plugin| Fit(plugin.layout.fit).into(),
            |plugin, value| {
                plugin.layout.fit = Fit::from(value).0;
                plugin.recompute_layout = true;
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
            renderer: dotlottie_rs::TvgRenderer::new(dotlottie_rs::TvgEngine::TvgEngineSw, 0),
            animation: dotlottie_rs::TvgAnimation::default(),
            layout: dotlottie_rs::Layout::new(dotlottie_rs::Fit::Contain, vec![0.5, 0.5]),
            recompute_layout: true,
            frame_step: 0.0,
            frame_number: 0.0,
            mode: Mode::Forward,
            direction: Direction::Forward,
            loop_animation: false,
            initialized: false,
            loaded: false,
        }
    }
}

impl frei0r_rs2::SourcePlugin for L0ttiePlugin {
    fn update_source(&mut self, _time: f64, outframe: &mut [u32]) {
        if !self.initialized {
            if let Err(err) = self.initialize() {
                eprintln!("Failed to initialize plugin: {}", err);
                return;
            }
        }
        if !self.loaded {
            return;
        }

        if let Err(err) = self.render(outframe) {
            eprintln!("Failed to render: {:?}", err);
        }
    }
}

impl L0ttiePlugin {
    fn initialize(&mut self) -> Result<(), Box<dyn Error>> {
        self.initialized = true;
        let animation_path = self
            .animation_path
            .to_str()
            .map_err(|err| format!("Invalid lottie animation path: {}", err))?;
        let data = std::fs::read_to_string(animation_path)
            .map_err(|err| format!("Failed to read lottie animation path: {}", err))?;
        self.animation
            .load_data(&data, "lottie", true)
            .map_err(|err| format!("Failed to load lottie animation path: {:?}", err))?;
        let player_fps = self
            .animation
            .get_total_frame()
            .map_err(|err| format!("Failed to query frame count: {:?}", err))?
            / self
                .animation
                .get_duration()
                .map_err(|err| format!("Failed to query duration: {:?}", err))?;
        self.frame_step = player_fps / self.video_fps as f32;
        //XXX support background color, push a shape
        self.renderer
            .push(Drawable::Animation(&self.animation))
            .map_err(|err| format!("Failed to add animation: {:?}", err))?;
        self.loaded = true;
        Ok(())
    }

    fn compute_layout(&mut self) -> Result<(), TvgError> {
        let (animation_width, animation_height) = self.animation.get_size()?;
        let (sx, sy, tx, ty) = self.layout.compute_layout_transform(
            self.width as f32,
            self.height as f32,
            animation_width,
            animation_height,
        );
        self.animation.set_size(sx, sy)?;
        self.animation.translate(tx, ty)?;
        Ok(())
    }

    fn render(&mut self, framebuffer: &mut [u32]) -> Result<(), TvgError> {
        // XXX pass slice directly https://github.com/LottieFiles/dotlottie-rs/pull/344
        let mut vecbuffer = Vec::from(framebuffer);
        self.renderer.set_target(
            &mut vecbuffer,
            self.width as u32,
            self.width as u32,
            self.height as u32,
            ColorSpace::ARGB8888,
        )?;

        if self.recompute_layout {
            if let Err(err) = self.compute_layout() {
                eprintln!("Failed to compute layout: {:?}", err);
            } else {
                self.recompute_layout = false;
            }
        }

        // Ignore errors, fails if we set the same frame
        let _ = self.animation.set_frame(self.frame_number);
        self.renderer.update()?;
        self.renderer.draw(true)?;
        self.renderer.sync()?;

        (self.frame_number, self.direction) = self.mode.next_frame(
            self.frame_number,
            self.frame_step,
            self.direction,
            0.0,
            self.animation.get_total_frame()?,
            self.loop_animation,
        );

        // XXX no rotation seems correct, but it should be 8
        #[cfg(any())]
        for pixel in framebuffer {
            // Rotate left by 8 bits: ARGB -> RGBA
            // dotlottie_rs::ColorSpace::ARGB8888 -> frei0r_rs2::ColorModel::RGBA8888
            *pixel = pixel.rotate_left(8);
        }

        Ok(())
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
