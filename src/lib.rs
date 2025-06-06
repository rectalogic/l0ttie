use dotlottie_rs::{Config, DotLottiePlayer, Fit, Layout, Mode};
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
                // XXX what is segment?
                // mode: Mode::Bounce,
                autoplay: true,
                loop_animation: true,
                // layout: Layout::new(Fit::Fill, vec![0.5, 0.5]),
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
            // ColorModel does not match LottieRenderer::get_color_space_for_target()
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
                // XXX should use load_dotlottie_data
                if self.player.load_animation_path(
                    lottie_file,
                    self.width as u32,
                    self.height as u32,
                ) {
                    let frame_rate = self.player.total_frames() / self.player.duration();
                    let speed = 30.0; //30.0 / frame_rate;
                    println!("speed {speed}");
                    if speed != 1.0 {
                        let mut config = self.player.config();
                        config.speed = speed;
                        self.player.set_config(config);
                    }
                    //XXX make marker configurable in frei0r params?
                    // if let Some(marker) = self.player.markers().first() {
                    //     let mut config = self.player.config();
                    //     config.marker = marker.name.clone();
                    //     self.player.set_config(config);
                    // }
                } else {
                    eprintln!("Failed to load lottie file {lottie_file}");
                }
            } else {
                eprintln!("Invalid lottie file");
            }
        }
        if !self.player.is_loaded() {
            return;
        }

        if self.player.tick() {
            // https://github.com/LottieFiles/dotlottie-rs/issues/335
            let frame = unsafe {
                &slice::from_raw_parts(self.player.buffer(), self.player.buffer_len() as usize)
                    [0..(self.width * self.height)]
            };
            outframe.copy_from_slice(frame);
        } else {
            println!("TICK FAILED");
        }
    }
}

frei0r_rs::plugin!(L0ttiePlugin);
