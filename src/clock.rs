pub mod clock {

    use crate::button::button::Button;
    use embedded_hal::digital::{InputPin, OutputPin, StatefulOutputPin};
    use panic_halt as _;
    use rp2040_hal::gpio::{AsInputPin, DynPinId, FunctionSioOutput, Pin, PullDown};
    pub struct Clock {
        pin: Pin<DynPinId, FunctionSioOutput, PullDown>,
        button: Button,
        pulse_interval: u64,
        pub auto: bool,
        last_tick: u64,
        state: bool,
    }

    impl Clock {
        pub fn new(pin: Pin<DynPinId, FunctionSioOutput, PullDown>, button: Button) -> Self {
            Clock {
                pin,
                button,
                pulse_interval: 1000,
                auto: false,
                last_tick: 0,
                state: false,
            }
        }

        pub fn update(&mut self, now: u64) -> bool {
            self.button.update(now);
            if self.auto {
                if now > self.last_tick + self.pulse_interval {
                    self.state = !self.state;
                    self.last_tick = now;
                }
            } else {
                self.state = self.button.state;
            }
            match self.state {
                true => self.pin.set_high().unwrap(),
                false => self.pin.set_low().unwrap(),
            }
            self.state
        }
    }
}
