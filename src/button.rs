pub mod button {

    use embedded_hal::digital::{InputPin, OutputPin, StatefulOutputPin};
    use panic_halt as _;
    use rp2040_hal::gpio::{AsInputPin, DynPinId, FunctionSioInput, Pin, PullDown};

    pub struct Button {
        pin: Pin<DynPinId, FunctionSioInput, PullDown>,
        pub state: bool,
        last_state: bool,
        update_interval: u64,
        debounce: u8,
        last_tick: u64,
    }

    impl Button {
        pub fn new(pin: Pin<DynPinId, FunctionSioInput, PullDown>) -> Self {
            Button {
                pin,
                state: false,
                last_state: false,
                update_interval: 1000,
                last_tick: 0,
                debounce: 0,
            }
        }

        pub fn update(&mut self, now: u64) -> bool {
            if now > self.last_tick + self.update_interval {
                self.debounce = (self.debounce << 1)
                    + match self.pin.is_high().unwrap() {
                        true => 1,
                        false => 0,
                    };
                if self.debounce == 0b11111111 {
                    self.last_state = self.state;
                    self.state = true;
                } else if self.debounce == 0b00000000 {
                    self.last_state = self.state;
                    self.state = false;
                }
                self.last_tick = now;
            }
            self.state
        }
    }
}
