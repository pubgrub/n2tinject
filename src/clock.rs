pub mod clock {

    static SYS_TICKS: u64 = 1_000_000;

    use crate::button::button::Button;
    use embedded_hal::digital::OutputPin;
    use panic_halt as _;
    use rp2040_hal::gpio::{DynPinId, FunctionSioOutput, Pin, PullDown};

    pub struct Clock {
        pin: Pin<DynPinId, FunctionSioOutput, PullDown>,
        button: Button,
        pulse_interval: u64,
        pub auto: bool,
        pub next_tick: u64,
        pub state: bool,
    }

    impl Clock {
        pub fn new(pin: Pin<DynPinId, FunctionSioOutput, PullDown>, button: Button) -> Self {
            Clock {
                pin,
                button,
                pulse_interval: 1000,
                auto: false,
                next_tick: 0,
                state: false,
            }
        }

        pub fn update(&mut self, now: u64) -> bool {
            self.button.update(now);
            if self.auto {
                if now > self.next_tick {
                    self.state = !self.state;
                    self.next_tick += self.pulse_interval;
                }
            } else {
                if self.button.has_changed() {
                    self.state = self.button.state;
                }
            }
            match self.state {
                true => self.pin.set_high().unwrap(),
                false => self.pin.set_low().unwrap(),
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
    }
}
