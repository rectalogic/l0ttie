// Copyright (C) 2026 Andrew Wason
// SPDX-License-Identifier: GPL-3.0-or-later

use std::ffi::CString;

use crate::mode::Mode;
use anyhow::Context;
use dotlottie_rs::{ColorSpace, Layout, LottieRenderer, Rgba, TvgRenderer};

pub struct Backend {
    animation_data: Option<CString>,
    mode: Mode,
    _layout: Layout,
    loop_animation: bool,
    width: u32,
    height: u32,
    renderer: Box<dyn LottieRenderer>,
}

impl Backend {
    pub fn new(
        animation_data: CString,
        width: u32,
        height: u32,
        layout: Layout,
        mode: Mode,
        loop_animation: bool,
        background_color: Option<frei0r_rs2::Color>,
    ) -> anyhow::Result<Self> {
        let mut renderer = <dyn LottieRenderer>::new(TvgRenderer::new(0));
        if let Some(background_color) = background_color {
            renderer.set_background(Rgba::new(
                (background_color.r * 255.0) as u8,
                (background_color.g * 255.0) as u8,
                (background_color.b * 255.0) as u8,
                255,
            ))?;
        }
        renderer.set_layout(&layout)?;

        Ok(Self {
            animation_data: Some(animation_data),
            mode,
            _layout: layout,
            loop_animation,
            width,
            height,
            renderer,
        })
    }

    pub fn render(&mut self, time: f64, outframe: &mut [u32]) -> anyhow::Result<()> {
        // Safety: set_sw_target should be unsafe. It holds a *mut ptr, but we reset on each render.
        self.renderer
            .set_sw_target(
                outframe,
                self.width,
                self.width,
                self.height,
                ColorSpace::ABGR8888,
            )
            .context("Failed to set render target")?;

        if let Some(animation_data) = self.animation_data.take() {
            self.renderer
                .load_data(&animation_data)
                .context("Failed to load lottie animation")?;
        }

        let duration = self
            .renderer
            .duration()
            .context("Failed to query duration")?;
        let animation_time = self.mode.next_frame(time, duration, self.loop_animation);

        // Convert animation time to frame number
        let total_frames = self
            .renderer
            .total_frames()
            .context("Failed to query total frames")?;
        let frame_number = if duration > 0.0 {
            (animation_time / duration) * total_frames
        } else {
            0.0
        };

        // Ignore errors, fails if we set the same frame
        let _ = self.renderer.set_frame(frame_number);
        self.renderer.render().context("Frame render failed")?;

        Ok(())
    }
}
