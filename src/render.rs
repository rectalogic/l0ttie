use std::sync::mpsc::{Receiver, Sender};

use crate::{backend::Backend, mode::Mode, processor::Processor};

struct RenderJob {
    time: f64,
    output: (*mut u32, usize),
}

// SAFETY: The caller guarantees input references remain valid until channel signals completion
unsafe impl Send for RenderJob {}

impl RenderJob {
    fn new(time: f64, output: &mut [u32]) -> Self {
        Self {
            time,
            output: (output.as_mut_ptr(), output.len()),
        }
    }
}

pub struct RenderProcessor {
    processor: Processor<RenderJob, anyhow::Result<()>>,
}

impl RenderProcessor {
    pub fn new(
        animation_data: String,
        width: u32,
        height: u32,
        layout: dotlottie_rs::Layout,
        mode: Mode,
        loop_animation: bool,
        background_color: Option<frei0r_rs2::Color>,
    ) -> anyhow::Result<Self> {
        let processor = Processor::new(
            move |rx: Receiver<RenderJob>, tx: Sender<anyhow::Result<()>>| {
                let mut backend_renderer = Backend::new(
                    animation_data,
                    width,
                    height,
                    layout,
                    mode,
                    loop_animation,
                    background_color,
                )?;
                for job in rx {
                    let output =
                        unsafe { std::slice::from_raw_parts_mut(job.output.0, job.output.1) };
                    tx.send(backend_renderer.render(job.time, output))?;
                }
                Ok(())
            },
        )?;
        Ok(Self { processor })
    }

    pub fn render(&mut self, time: f64, output: &mut [u32]) -> anyhow::Result<()> {
        let job = RenderJob::new(time, output);
        self.processor.process(job)?
    }
}
