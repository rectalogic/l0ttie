# l0ttie

A [frei0r](https://dyne.org/software/frei0r/) video source plugin that renders  [Lottie](https://lottie.github.io) animation files.

Supported parameters are:

0. `animation` - a path or URL to a Lottie animation JSON file
1. `time_scale` - time scale multiplier (default 1.0). `ffmpeg` reports frei0r times in milliseconds so use `0.001` to convert to seconds.
2. `mode` - playback mode `forward` (default), `reverse`, `bounce` or `reverse-bounce`
3. `loop` - loop animation (default false)
4. `fit` - how to fit animation to video frame: `contain` (default), `fill`, `cover`, `fit-width`, `fit-height` or `none`
5. `background_color` - background color of animation, default is transparent

## Example

Download the plugin [release](https://github.com/rectalogic/l0ttie/releases) and extract into a `frei0r-plugin` directory.

On macOS you must then `xattr -dr com.apple.quarantine l0ttie.so`.

Example using [ffmpeg](https://ffmpeg.org) to overlay a transparent animation over a video. The animation is half the size of the video and positioned in the lower right corner.

```sh
FREI0R_PATH=frei0r-plugin/ ffmpeg \
  -i https://sample-videos.com/video321/mp4/360/big_buck_bunny_360p_1mb.mp4 \
  -f lavfi -i "frei0r_src=size=320x180:framerate=25:filter_name=l0ttie:filter_params=https\\\\://lottie.host/b5100a40-ab25-4a1e-8ac4-88b63f3f1018/Nd2wTOGBRS.json|0.001" \
  -filter_complex overlay=x=main_w-overlay_w:y=main_h-overlay_h:eval=init \
  -t 2 -y output.mp4
```


https://github.com/user-attachments/assets/563f34bc-3e24-4d65-a82d-950a0b4675fb


A similar command using `melt` from [MLT framework](https://www.mltframework.org)

```sh
FREI0R_PATH=frei0r-plugin/ melt \
  https://sample-videos.com/video321/mp4/360/big_buck_bunny_360p_1mb.mp4 out=60 \
  frei0r.l0ttie 0=https://lottie.host/b5100a40-ab25-4a1e-8ac4-88b63f3f1018/Nd2wTOGBRS.json out=60 \
  -mix 60 -mixer affine scale_x=2 scale_y=2 halign=right valign=bottom
```
