use dotlottie_rs::{Config, DotLottiePlayer};
use std::{ffi::CString, slice};

#[derive(frei0r_rs::PluginBase)]
pub struct L0ttiePlugin {
    #[frei0r(explain = c"Lottie file path")]
    lottie_file: CString,
    width: usize,
    height: usize,
    player: DotLottiePlayer,
    initialized: bool,
}

impl L0ttiePlugin {
    fn new(width: usize, height: usize) -> Self {
        Self {
            lottie_file: c"".into(),
            width,
            height,
            player: DotLottiePlayer::new(Config {
                //XXX set mode, speed, layout etc
                use_frame_interpolation: false,
                ..Config::default()
            }),
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
            if let Ok(lottie_file) = self.lottie_file.to_str() {
                if self.player.load_animation_path(
                    lottie_file,
                    self.width as u32,
                    self.height as u32,
                ) {
                    if !self.player.play() {
                        eprintln!("Failed to play lottie file {lottie_file}");
                    }
                } else {
                    eprintln!("Failed to load lottie file {lottie_file}");
                }
            } else {
                eprintln!("Invalid lottie file");
            }
        }
        if !self.player.is_playing() {
            return;
        }

        if self.player.set_frame(self.player.request_frame()) && self.player.render() {
            let frame = unsafe {
                slice::from_raw_parts(self.player.buffer(), self.player.buffer_len() as usize)
            };
            outframe.copy_from_slice(frame);
        }
    }
}

frei0r_rs::plugin!(L0ttiePlugin);
