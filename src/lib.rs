// Copyright (C) 2025 Andrew Wason
// SPDX-License-Identifier: GPL-3.0-or-later
use std::ffi::{CStr, CString};

use anyhow::Context;
use dotlottie_rs::{Animation, ColorSpace, Drawable, Renderer, Shape};

pub struct L0ttiePlugin {
    animation_path: CString,
    mode: Mode,
    loop_animation: bool,
    layout: dotlottie_rs::Layout,
    time_scale: f64,
    background_color: Option<frei0r_rs2::Color>,
    width: usize,
    height: usize,
    renderer: dotlottie_rs::TvgRenderer,
    animation: dotlottie_rs::TvgAnimation,
    background_shape: Option<dotlottie_rs::TvgShape>,
    recompute_layout: bool,
    initialized: bool,
    loaded: bool,
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
            c"time_scale",
            c"Time scale multiplier",
            |plugin| plugin.time_scale,
            |plugin, value| {
                plugin.time_scale = value;
            }
        ),
        frei0r_rs2::ParamInfo::new_string(
            c"mode",
            c"Playback mode: 'forward' (default), 'reverse', 'bounce', 'reverse-bounce'",
            |plugin| plugin.mode.into(),
            |plugin, value| {
                plugin.mode = Mode::from(value);
            }
        ),
        frei0r_rs2::ParamInfo::new_bool(
            c"loop",
            c"Loop animation",
            |plugin| plugin.loop_animation,
            |plugin, value| {
                plugin.loop_animation = value;
            }
        ),
        frei0r_rs2::ParamInfo::new_string(
            c"fit",
            c"Fit animation to video frame: 'contain' (default), 'fill', 'cover', 'fit-width', 'fit-height', 'none'",
            |plugin| Fit(plugin.layout.fit).into(),
            |plugin, value| {
                plugin.layout.fit = Fit::from(value).0;
                plugin.recompute_layout = true;
            }
        ),
        frei0r_rs2::ParamInfo::new_color(
            c"background_color",
            c"Background color",
            |plugin| plugin.background_color.unwrap_or(frei0r_rs2::Color { r:0.0, g:0.0, b:0.0}),
            |plugin, value| {
                plugin.background_color = Some(*value);
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
            width,
            height,
            mode: Mode::Forward,
            loop_animation: false,
            time_scale: 1.0,
            layout: dotlottie_rs::Layout::new(dotlottie_rs::Fit::Contain, vec![0.5, 0.5]),
            background_color: None,
            renderer: dotlottie_rs::TvgRenderer::new(dotlottie_rs::TvgEngine::TvgEngineSw, 0),
            animation: dotlottie_rs::TvgAnimation::default(),
            background_shape: None,
            recompute_layout: true,
            initialized: false,
            loaded: false,
        }
    }
}

impl frei0r_rs2::SourcePlugin for L0ttiePlugin {
    fn update_source(&mut self, time: f64, outframe: &mut [u32]) {
        if let Err(err) = self.renderer.set_target(
            outframe,
            self.width as u32,
            self.width as u32,
            self.height as u32,
            ColorSpace::ABGR8888,
        ) {
            eprintln!("Failed to set render target: {:?}", err);
            return;
        }
        if !self.initialized {
            if let Err(err) = self.initialize() {
                eprintln!("Failed to initialize plugin: {}", err);
                return;
            }
        }
        if !self.loaded {
            return;
        }

        if let Err(err) = self.render(time * self.time_scale) {
            eprintln!("Failed to render: {:?}", err);
        }
    }
}

impl L0ttiePlugin {
    fn initialize(&mut self) -> anyhow::Result<()> {
        self.initialized = true;
        let animation_path = self
            .animation_path
            .to_str()
            .with_context(|| format!("Invalid lottie animation path: {:?}", self.animation_path))?;
        let data = std::fs::read_to_string(animation_path)
            .with_context(|| format!("Failed to read lottie animation path: {animation_path}"))?;
        self.animation
            .load_data(&data, "lottie", true)
            .with_context(|| format!("Failed to load lottie animation path: {animation_path}"))?;
        if let Some(background_color) = self.background_color {
            let mut background_shape = dotlottie_rs::TvgShape::default();
            background_shape
                .append_rect(0.0, 0.0, self.width as f32, self.height as f32, 0.0, 0.0)
                .context("Failed to construct background shape")?;
            background_shape
                .fill((
                    (background_color.r * 255.0) as u8,
                    (background_color.g * 255.0) as u8,
                    (background_color.b * 255.0) as u8,
                    255,
                ))
                .context("Failed to fill background shape")?;
            self.renderer
                .push(Drawable::Shape(&background_shape))
                .context("Failed to add background shape")?;
            self.background_shape = Some(background_shape);
        }
        self.renderer
            .push(Drawable::Animation(&self.animation))
            .context("Failed to add animation")?;
        self.loaded = true;
        Ok(())
    }

    fn compute_layout(&mut self) -> anyhow::Result<()> {
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

    fn render(&mut self, time: f64) -> anyhow::Result<()> {
        if self.recompute_layout {
            self.compute_layout().context("Failed to compute layout")?;
            self.recompute_layout = false;
        }

        let duration = self
            .animation
            .get_duration()
            .context("Failed to query duration")?;
        let animation_time = self.mode.next_frame(time, duration, self.loop_animation);

        // Convert animation time to frame number
        let total_frames = self
            .animation
            .get_total_frame()
            .context("Failed to query total frames")?;
        let frame_number = if duration > 0.0 {
            (animation_time / duration) * total_frames
        } else {
            0.0
        };

        // Ignore errors, fails if we set the same frame
        let _ = self.animation.set_frame(frame_number);
        self.renderer.update()?;
        self.renderer.draw(true)?;
        self.renderer.sync()?;

        Ok(())
    }
}

#[derive(Copy, Clone, Debug)]
enum Mode {
    Forward,
    Reverse,
    Bounce,
    ReverseBounce,
}

impl Mode {
    fn next_frame(&self, time: f64, duration: f32, loop_animation: bool) -> f32 {
        let time = time as f32;

        if duration <= 0.0 {
            return 0.0;
        }

        match self {
            Mode::Forward => {
                if loop_animation {
                    time % duration
                } else {
                    time.min(duration)
                }
            }
            Mode::Reverse => {
                if loop_animation {
                    duration - (time % duration)
                } else {
                    (duration - time).max(0.0)
                }
            }
            Mode::Bounce => {
                let cycle_duration = 2.0 * duration;
                if loop_animation {
                    let cycle_time = time % cycle_duration;
                    if cycle_time <= duration {
                        cycle_time
                    } else {
                        cycle_duration - cycle_time
                    }
                } else if time <= duration {
                    time
                } else if time <= cycle_duration {
                    cycle_duration - time
                } else {
                    0.0
                }
            }
            Mode::ReverseBounce => {
                let cycle_duration = 2.0 * duration;
                if loop_animation {
                    let cycle_time = time % cycle_duration;
                    if cycle_time <= duration {
                        duration - cycle_time
                    } else {
                        cycle_time - duration
                    }
                } else if time <= duration {
                    duration - time
                } else if time <= cycle_duration {
                    time - duration
                } else {
                    duration
                }
            }
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
