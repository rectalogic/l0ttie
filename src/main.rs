use dotlottie_rs::{Config, DotLottiePlayer};
use std::slice;

fn main() {
    let player = DotLottiePlayer::new(Config {
        use_frame_interpolation: false,
        ..Config::default()
    });
    assert!(player.load_animation_path("test.json", 100, 100));
    assert!(player.play());
    let total_frames = player.total_frames();

    while player.current_frame() < total_frames {
        let next_frame = player.request_frame();

        if player.set_frame(next_frame) {
            player.render();
            let frame =
                unsafe { slice::from_raw_parts(player.buffer(), player.buffer_len() as usize) };
            println!("{next_frame}: {:?}", &frame[0..10]);
        }
    }
}
