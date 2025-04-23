pub mod clock {

    static SYS_TICKS: u64 = 1_000_000;

    use embedded_hal::digital::OutputPin;
    use panic_halt as _;
    use rp2040_hal::gpio::{DynPinId, FunctionSioOutput, Pin, PullDown};

    use crate::button::button::ButtonState;

    pub struct Clock {
        pin: Pin<DynPinId, FunctionSioOutput, PullDown>,
        pulse_interval: u64,
        pub auto: bool,
        pub next_tick: u64,
        pub state: bool,
    }

    impl Clock {
        pub fn new(pin: Pin<DynPinId, FunctionSioOutput, PullDown>, now: u64) -> Self {
            Clock {
                pin,
                pulse_interval: 1000,
                auto: false,
                next_tick: now,
                state: false,
            }
        }

        pub fn update(&mut self, now: u64, button_state: &ButtonState) -> bool {
            let mut changed = false;
            if self.auto {
                if now > self.next_tick {
                    self.state = !self.state;
                    self.next_tick += self.pulse_interval;
                    changed = true;
                }
            } else {
                if button_state.state_changed {
                    self.state = button_state.state;
                    changed = true;
                }
            }
            if changed {
                self.set_pin(self.state);
            }
            self.state
        }

        pub fn set_freq(&mut self, interval: &u64) {
            self.pulse_interval = SYS_TICKS / (interval * 2) * 10;
        }

        pub fn sync(&mut self, clock2: &mut Clock) {
            clock2.state = self.state;
            clock2.next_tick = self.next_tick;
        }

        pub fn sync_opposite(&mut self, clock2: &mut Clock) {
            clock2.state = self.state;
            clock2.next_tick = self.next_tick + self.pulse_interval;
        }

        pub fn set_pin(&mut self, state: bool) {
            if state {
                self.pin.set_high().unwrap();
            } else {
                self.pin.set_low().unwrap();
            }
        }
    }
}
