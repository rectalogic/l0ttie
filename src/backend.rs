// Copyright (C) 2026 Andrew Wason
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::mode::Mode;
use anyhow::Context;
use dotlottie_rs::{Animation, ColorSpace, Drawable, Renderer, Shape};

pub struct Backend {
    mode: Mode,
    loop_animation: bool,
    width: u32,
    height: u32,
    renderer: dotlottie_rs::TvgRenderer,
    animation: dotlottie_rs::TvgAnimation,
    _background_shape: Option<dotlottie_rs::TvgShape>,
}

impl Backend {
    pub fn new(
        animation_data: String,
        width: u32,
        height: u32,
        layout: dotlottie_rs::Layout,
        mode: Mode,
        loop_animation: bool,
        background_color: Option<frei0r_rs2::Color>,
    ) -> anyhow::Result<Self> {
        let mut renderer = dotlottie_rs::TvgRenderer::new(dotlottie_rs::TvgEngine::TvgEngineSw, 0);
        let mut animation = dotlottie_rs::TvgAnimation::default();
        animation
            .load_data(&animation_data, "lottie", true)
            .context("Failed to load lottie animation")?;
        let background_shape = background_color
            .map(
                |background_color| -> anyhow::Result<dotlottie_rs::TvgShape> {
                    let mut background_shape = dotlottie_rs::TvgShape::default();
                    background_shape
                        .append_rect(0.0, 0.0, width as f32, height as f32, 0.0, 0.0)
                        .context("Failed to construct background shape")?;
                    background_shape
                        .fill((
                            (background_color.r * 255.0) as u8,
                            (background_color.g * 255.0) as u8,
                            (background_color.b * 255.0) as u8,
                            255,
                        ))
                        .context("Failed to fill background shape")?;
                    renderer
                        .push(Drawable::Shape(&background_shape))
                        .context("Failed to add background shape")?;
                    Ok(background_shape)
                },
            )
            .transpose()?;

        renderer
            .push(Drawable::Animation(&animation))
            .context("Failed to add animation")?;

        let (animation_width, animation_height) = animation.get_size()?;
        let (sx, sy, tx, ty) = layout.compute_layout_transform(
            width as f32,
            height as f32,
            animation_width,
            animation_height,
        );
        animation.set_size(sx, sy)?;
        animation.translate(tx, ty)?;

        renderer.sync().context("Canvas sync failed")?;

        Ok(Self {
            mode,
            loop_animation,
            width,
            height,
            renderer,
            animation,
            _background_shape: background_shape,
        })
    }

    pub fn render(&mut self, time: f64, outframe: &mut [u32]) -> anyhow::Result<()> {
        self.renderer
            .set_target(
                outframe,
                self.width,
                self.width,
                self.height,
                ColorSpace::ABGR8888,
            )
            .context("Failed to set render target")?;

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
        self.renderer.update().context("Render update failed")?;
        self.renderer.draw(true).context("Render draw failed")?;
        self.renderer.sync().context("Render sync failed")?;

        Ok(())
    }
}
