use dotlottie_rs::{Config, DotLottiePlayer, Fit, Layout, Mode};
use std::{ffi::CString, slice};

#[derive(frei0r_rs::PluginBase)]
pub struct L0ttiePlugin {
    #[frei0r(explain = c"Lottie animation file path")]
    animation_path: CString,
    #[frei0r(
        explain = c"Fit animation to video frame: 'contain' (default), 'fill', 'cover', 'fit-width', 'fit-height', 'none'"
    )]
    fit: CString,
    width: usize,
    height: usize,
    player: DotLottiePlayer,
    frame: f32,
    initialized: bool,
}

impl L0ttiePlugin {
    fn new(width: usize, height: usize) -> Self {
        Self {
            animation_path: c"".into(),
            fit: c"contain".into(),
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
            frame: 0.0,
            initialized: false,
        }
    }
}

impl frei0r_rs::Plugin for L0ttiePlugin {
    type Kind = frei0r_rs::KindSource;

    fn info() -> frei0r_rs::PluginInfo {
        frei0r_rs::PluginInfo {
            name: c"l0ttie",
            author: c"Andrew Wason",
            color_model: frei0r_rs::ColorModel::RGBA8888,
            major_version: env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap(),
            minor_version: env!("CARGO_PKG_VERSION_MINOR").parse().unwrap(),
            explanation: c"Lottie renderer using dotlottie-rs",
        }
    }

    fn new(width: usize, height: usize) -> Self {
        L0ttiePlugin::new(width, height)
    }
}

impl frei0r_rs::SourcePlugin for L0ttiePlugin {
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
                    let frame_rate = self.player.total_frames() / self.player.duration();
                    // let speed = 30.0; //30.0 / frame_rate;
                    println!(
                        "total_frames {} duration {}",
                        self.player.total_frames(),
                        self.player.duration()
                    );
                    let mut config = self.player.config();
                    config.layout = Layout::new(convert_fit(&self.fit), vec![0.5, 0.5]);
                    self.player.set_config(config);

                    //XXX make marker configurable in frei0r params?
                    // if let Some(marker) = self.player.markers().first() {
                    //     let mut config = self.player.config();
                    //     config.marker = marker.name.clone();
                    //     self.player.set_config(config);
                    // }
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
            self.frame += 1.0;
            // https://github.com/LottieFiles/dotlottie-rs/issues/335
            let frame = unsafe {
                &slice::from_raw_parts(self.player.buffer(), self.player.buffer_len() as usize)
                    [0..(self.width * self.height)]
            };
            outframe.copy_from_slice(frame);
            for pixel in outframe {
                // Rotate left by 8 bits: ARGB -> RGBA
                // dotlottie_rs::ColorSpace::ARGB8888 -> frei0r_rs::ColorModel::RGBA8888
                // XXX no rotation seems correct, but it should be 8
                *pixel = pixel.rotate_left(0);
            }
        } else {
            println!("RENDER FAILED {}", self.frame);
        }
    }
}

fn convert_fit(fit: &CString) -> Fit {
    if let Ok(fit) = fit.to_str() {
        match fit {
            "contain" => Fit::Contain,
            "fill" => Fit::Fill,
            "cover" => Fit::Cover,
            "fit-width" => Fit::FitWidth,
            "fit-height" => Fit::FitHeight,
            "none" => Fit::None,
            _ => Fit::Contain,
        }
    } else {
        Fit::Contain
    }
}
frei0r_rs::plugin!(L0ttiePlugin);
