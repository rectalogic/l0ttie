// Copyright (C) 2025 Andrew Wason
// SPDX-License-Identifier: GPL-3.0-or-later
mod fit;
mod mode;
use std::ffi::CString;

use anyhow::Context;
use dotlottie_rs::{Animation, ColorSpace, Drawable, Renderer, Shape};
use ureq::http::Uri;

pub struct L0ttiePlugin {
    animation_path: CString,
    mode: mode::Mode,
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
            c"animation",
            c"Lottie animation file path or URL",
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
                plugin.mode = mode::Mode::from(value);
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
            |plugin| fit::Fit(plugin.layout.fit).into(),
            |plugin, value| {
                plugin.layout.fit = fit::Fit::from(value).0;
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
            mode: mode::Mode::Forward,
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

        let data = if let Ok(animation_uri) = animation_path.parse::<Uri>() {
            if animation_uri.scheme().is_some() {
                ureq::get(animation_path)
                    .call()
                    .with_context(|| {
                        format!("Failed to load lottie animation url: {animation_path}")
                    })?
                    .body_mut()
                    .read_to_string()
                    .with_context(|| {
                        format!("Failed to read lottie animation url: {animation_path}")
                    })?
            } else {
                std::fs::read_to_string(animation_path).with_context(|| {
                    format!("Failed to read lottie animation path: {animation_path}")
                })?
            }
        } else {
            std::fs::read_to_string(animation_path).with_context(|| {
                format!("Failed to read lottie animation path: {animation_path}")
            })?
        };

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

frei0r_rs2::plugin!(L0ttiePlugin);
