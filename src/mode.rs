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

#[cfg(test)]
mod tests {
    use super::*;

    const DURATION: f32 = 10.0;

    #[test]
    fn test_forward_mode_no_loop() {
        let mode = Mode::Forward;

        // Within duration
        assert_eq!(mode.next_frame(5.0, DURATION, false), 5.0);
        assert_eq!(mode.next_frame(0.0, DURATION, false), 0.0);
        assert_eq!(mode.next_frame(10.0, DURATION, false), 10.0);

        // Beyond duration
        assert_eq!(mode.next_frame(15.0, DURATION, false), 10.0);
        assert_eq!(mode.next_frame(25.0, DURATION, false), 10.0);
    }

    #[test]
    fn test_forward_mode_with_loop() {
        let mode = Mode::Forward;

        // Within duration
        assert_eq!(mode.next_frame(5.0, DURATION, true), 5.0);
        assert_eq!(mode.next_frame(0.0, DURATION, true), 0.0);
        assert_eq!(mode.next_frame(10.0, DURATION, true), 0.0);

        // Beyond duration - should wrap around
        assert_eq!(mode.next_frame(15.0, DURATION, true), 5.0);
        assert_eq!(mode.next_frame(25.0, DURATION, true), 5.0);
        assert_eq!(mode.next_frame(30.0, DURATION, true), 0.0);
    }

    #[test]
    fn test_reverse_mode_no_loop() {
        let mode = Mode::Reverse;

        // Within duration
        assert_eq!(mode.next_frame(5.0, DURATION, false), 5.0);
        assert_eq!(mode.next_frame(0.0, DURATION, false), 10.0);
        assert_eq!(mode.next_frame(10.0, DURATION, false), 0.0);

        // Beyond duration
        assert_eq!(mode.next_frame(15.0, DURATION, false), 0.0);
        assert_eq!(mode.next_frame(25.0, DURATION, false), 0.0);
    }

    #[test]
    fn test_reverse_mode_with_loop() {
        let mode = Mode::Reverse;

        // Within duration
        assert_eq!(mode.next_frame(5.0, DURATION, true), 5.0);
        assert_eq!(mode.next_frame(0.0, DURATION, true), 10.0);
        assert_eq!(mode.next_frame(10.0, DURATION, true), 10.0);

        // Beyond duration - should wrap around
        assert_eq!(mode.next_frame(15.0, DURATION, true), 5.0);
        assert_eq!(mode.next_frame(25.0, DURATION, true), 5.0);
        assert_eq!(mode.next_frame(30.0, DURATION, true), 10.0);
    }

    #[test]
    fn test_bounce_mode_no_loop() {
        let mode = Mode::Bounce;

        // First half of bounce cycle (0 -> duration)
        assert_eq!(mode.next_frame(0.0, DURATION, false), 0.0);
        assert_eq!(mode.next_frame(5.0, DURATION, false), 5.0);
        assert_eq!(mode.next_frame(10.0, DURATION, false), 10.0);

        // Second half of bounce cycle (duration -> 0)
        assert_eq!(mode.next_frame(15.0, DURATION, false), 5.0);
        assert_eq!(mode.next_frame(20.0, DURATION, false), 0.0);

        // Beyond one complete bounce cycle
        assert_eq!(mode.next_frame(25.0, DURATION, false), 0.0);
        assert_eq!(mode.next_frame(30.0, DURATION, false), 0.0);
    }

    #[test]
    fn test_bounce_mode_with_loop() {
        let mode = Mode::Bounce;

        // First bounce cycle
        assert_eq!(mode.next_frame(0.0, DURATION, true), 0.0);
        assert_eq!(mode.next_frame(5.0, DURATION, true), 5.0);
        assert_eq!(mode.next_frame(10.0, DURATION, true), 10.0);
        assert_eq!(mode.next_frame(15.0, DURATION, true), 5.0);
        assert_eq!(mode.next_frame(20.0, DURATION, true), 0.0);

        // Second bounce cycle (should repeat)
        assert_eq!(mode.next_frame(25.0, DURATION, true), 5.0);
        assert_eq!(mode.next_frame(30.0, DURATION, true), 10.0);
        assert_eq!(mode.next_frame(35.0, DURATION, true), 5.0);
        assert_eq!(mode.next_frame(40.0, DURATION, true), 0.0);
    }

    #[test]
    fn test_reverse_bounce_mode_no_loop() {
        let mode = Mode::ReverseBounce;

        // First half of reverse bounce cycle (duration -> 0)
        assert_eq!(mode.next_frame(0.0, DURATION, false), 10.0);
        assert_eq!(mode.next_frame(5.0, DURATION, false), 5.0);
        assert_eq!(mode.next_frame(10.0, DURATION, false), 0.0);

        // Second half of reverse bounce cycle (0 -> duration)
        assert_eq!(mode.next_frame(15.0, DURATION, false), 5.0);
        assert_eq!(mode.next_frame(20.0, DURATION, false), 10.0);

        // Beyond one complete reverse bounce cycle
        assert_eq!(mode.next_frame(25.0, DURATION, false), 10.0);
        assert_eq!(mode.next_frame(30.0, DURATION, false), 10.0);
    }

    #[test]
    fn test_reverse_bounce_mode_with_loop() {
        let mode = Mode::ReverseBounce;

        // First reverse bounce cycle
        assert_eq!(mode.next_frame(0.0, DURATION, true), 10.0);
        assert_eq!(mode.next_frame(5.0, DURATION, true), 5.0);
        assert_eq!(mode.next_frame(10.0, DURATION, true), 0.0);
        assert_eq!(mode.next_frame(15.0, DURATION, true), 5.0);
        assert_eq!(mode.next_frame(20.0, DURATION, true), 10.0);

        // Second reverse bounce cycle (should repeat)
        assert_eq!(mode.next_frame(25.0, DURATION, true), 5.0);
        assert_eq!(mode.next_frame(30.0, DURATION, true), 0.0);
        assert_eq!(mode.next_frame(35.0, DURATION, true), 5.0);
        assert_eq!(mode.next_frame(40.0, DURATION, true), 10.0);
    }

    #[test]
    fn test_zero_duration() {
        let modes = [
            Mode::Forward,
            Mode::Reverse,
            Mode::Bounce,
            Mode::ReverseBounce,
        ];

        for mode in modes {
            assert_eq!(mode.next_frame(5.0, 0.0, false), 0.0);
            assert_eq!(mode.next_frame(5.0, 0.0, true), 0.0);
        }
    }

    #[test]
    fn test_negative_duration() {
        let modes = [
            Mode::Forward,
            Mode::Reverse,
            Mode::Bounce,
            Mode::ReverseBounce,
        ];

        for mode in modes {
            assert_eq!(mode.next_frame(5.0, -1.0, false), 0.0);
            assert_eq!(mode.next_frame(5.0, -1.0, true), 0.0);
        }
    }

    #[test]
    fn test_mode_from_cstr() {
        assert!(matches!(Mode::from(MODE_FORWARD), Mode::Forward));
        assert!(matches!(Mode::from(MODE_REVERSE), Mode::Reverse));
        assert!(matches!(Mode::from(MODE_BOUNCE), Mode::Bounce));
        assert!(matches!(
            Mode::from(MODE_REVERSE_BOUNCE),
            Mode::ReverseBounce
        ));

        // Test unknown mode defaults to Forward
        let unknown = c"unknown";
        assert!(matches!(Mode::from(unknown), Mode::Forward));
    }

    #[test]
    fn test_mode_to_cstr() {
        assert_eq!(<&CStr>::from(Mode::Forward), MODE_FORWARD);
        assert_eq!(<&CStr>::from(Mode::Reverse), MODE_REVERSE);
        assert_eq!(<&CStr>::from(Mode::Bounce), MODE_BOUNCE);
        assert_eq!(<&CStr>::from(Mode::ReverseBounce), MODE_REVERSE_BOUNCE);
    }
}
