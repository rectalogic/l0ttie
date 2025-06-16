// Copyright (C) 2025 Andrew Wason
// SPDX-License-Identifier: GPL-3.0-or-later

use std::ffi::CStr;

#[derive(Copy, Clone, Debug)]
pub(crate) enum Mode {
    Forward,
    Reverse,
    Bounce,
    ReverseBounce,
}

impl Mode {
    pub(crate) fn next_frame(&self, time: f64, duration: f32, loop_animation: bool) -> f32 {
        let time = time as f32;

        if duration <= 0.0 {
            return 0.0;
        }

        match self {
            Mode::Forward => {
                if loop_animation {
                    time % duration
                } else {
                    time.min(duration)
                }
            }
            Mode::Reverse => {
                if loop_animation {
                    duration - (time % duration)
                } else {
                    (duration - time).max(0.0)
                }
            }
            Mode::Bounce => {
                let cycle_duration = 2.0 * duration;
                if loop_animation {
                    let cycle_time = time % cycle_duration;
                    if cycle_time <= duration {
                        cycle_time
                    } else {
                        cycle_duration - cycle_time
                    }
                } else if time <= duration {
                    time
                } else if time <= cycle_duration {
                    cycle_duration - time
                } else {
                    0.0
                }
            }
            Mode::ReverseBounce => {
                let cycle_duration = 2.0 * duration;
                if loop_animation {
                    let cycle_time = time % cycle_duration;
                    if cycle_time <= duration {
                        duration - cycle_time
                    } else {
                        cycle_time - duration
                    }
                } else if time <= duration {
                    duration - time
                } else if time <= cycle_duration {
                    time - duration
                } else {
                    duration
                }
            }
        }
    }
}

pub(crate) const MODE_FORWARD: &CStr = c"forward";
pub(crate) const MODE_REVERSE: &CStr = c"reverse";
pub(crate) const MODE_BOUNCE: &CStr = c"bounce";
pub(crate) const MODE_REVERSE_BOUNCE: &CStr = c"reverse-bounce";

impl From<&CStr> for Mode {
    fn from(value: &CStr) -> Self {
        if value == MODE_FORWARD {
            Mode::Forward
        } else if value == MODE_REVERSE {
            Mode::Reverse
        } else if value == MODE_BOUNCE {
            Mode::Bounce
        } else if value == MODE_REVERSE_BOUNCE {
            Mode::ReverseBounce
        } else {
            Mode::Forward
        }
    }
}

impl From<Mode> for &'static CStr {
    fn from(mode: Mode) -> Self {
        match mode {
            Mode::Forward => MODE_FORWARD,
            Mode::Reverse => MODE_REVERSE,
            Mode::Bounce => MODE_BOUNCE,
            Mode::ReverseBounce => MODE_REVERSE_BOUNCE,
        }
    }
}
