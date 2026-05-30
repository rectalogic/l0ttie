// Copyright (C) 2025 Andrew Wason
// SPDX-License-Identifier: GPL-3.0-or-later
mod backend;
mod fit;
mod mode;
mod processor;
use std::ffi::CString;

use anyhow::Context;
use dotlottie_rs::{ColorSpace, Renderer};
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
}

impl frei0r_rs2::Plugin<0> for L0ttiePlugin {
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
        }
    }

    fn update(&mut self, time: f64, _inframes: [&[u32]; 0], outframe: &mut [u32]) {
        if let Err(err) = self.renderer.set_target(
            outframe,
            self.width as u32,
            self.width as u32,
            self.height as u32,
            ColorSpace::ABGR8888,
        ) {
            eprintln!("Failed to set render target: {err:?}");
            return;
        }
        if !self.initialized {
            if let Err(err) = self.initialize() {
                eprintln!("Failed to initialize plugin: {err:?}");
                return;
            }
        }
        if !self.loaded {
            return;
        }

        if let Err(err) = self.render(time * self.time_scale) {
            eprintln!("Failed to render: {err:?}");
        }
    }
}

impl L0ttiePlugin {
    fn initialize(&mut self) -> anyhow::Result<()> {
        let animation_path = self
            .animation_path
            .to_str()
            .with_context(|| format!("Invalid lottie animation path: {:?}", self.animation_path))?;

        let animation_data = if let Ok(animation_uri) = animation_path.parse::<Uri>() {
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

        Ok(())
    }
}

frei0r_rs2::plugin!(L0ttiePlugin);
