pub mod clock {

    static SYS_TICKS: u64 = 1_000_000;

    use embedded_hal::digital::OutputPin;
    use panic_halt as _;
    use rp2040_hal::gpio::{DynPinId, FunctionSioOutput, Pin, PullDown};

    use crate::button::button::ButtonState;
    pub enum ClockMode {
        Manual,
        Auto,
    }

    pub struct Clock {
        pin: Pin<DynPinId, FunctionSioOutput, PullDown>,
        ticks_per_step: u64,
        pub mode: ClockMode,
        pub next_tick: u64,
        pub state: bool,
        pub freq: u64,
    }

    impl Clock {
        pub fn new(pin: Pin<DynPinId, FunctionSioOutput, PullDown>, now: u64) -> Self {
            let mut cl = Clock {
                pin,
                ticks_per_step: 1000,
                mode: ClockMode::Manual,
                next_tick: now,
                state: false,
                freq: 10,
            };
            cl.set_ticks_per_step();
            cl
        }

        pub fn update(&mut self, now: u64, button_state: &ButtonState) -> bool {
            let mut changed = false;
            match self.mode {
                ClockMode::Manual => {
                    if button_state.state_changed {
                        self.state = button_state.state;
                        changed = true;
                    }
                }
                ClockMode::Auto => {
                    if now > self.next_tick {
                        self.state = !self.state;
                        self.next_tick += self.ticks_per_step;
                        changed = true;
                    }
                }
            }
            if changed {
                self.set_pin(self.state);
            }
            self.state
        }

        pub fn set_freq(&mut self, interval: &u64) {
            self.freq = *interval;
            self.set_ticks_per_step();
        }

        pub fn sync(&mut self, clock2: &mut Clock) {
            clock2.state = self.state;
            clock2.next_tick = self.next_tick;
        }

        pub fn sync_opposite(&mut self, clock2: &mut Clock) {
            clock2.state = self.state;
            clock2.next_tick = self.next_tick + self.ticks_per_step;
        }

        pub fn set_pin(&mut self, state: bool) {
            if state {
                self.pin.set_high().unwrap();
            } else {
                self.pin.set_low().unwrap();
            }
        }
        fn set_ticks_per_step(&mut self) {
            self.ticks_per_step = SYS_TICKS * 10 / (self.freq * 2);
        }
    }
}
