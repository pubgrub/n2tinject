pub mod button {

    use embedded_hal::digital::InputPin;
    use panic_halt as _;
    use rp2040_hal::gpio::{DynPinId, FunctionSioInput, Pin, PullDown};

    pub struct Button {
        pin: Pin<DynPinId, FunctionSioInput, PullDown>,
        pub state: bool,
        pub state_changed: bool,
        update_interval: u64,
        debounce: u8,
        next_tick: u64,
    }

    impl Button {
        pub fn new(pin: Pin<DynPinId, FunctionSioInput, PullDown>) -> Self {
            Button {
                pin,
                state: false,
                state_changed: false,
                update_interval: 1000,
                next_tick: 0,
                debounce: 0,
            }
        }

        pub fn update(&mut self, now: u64) -> bool {
            if now > self.next_tick {
                self.debounce = (self.debounce << 1)
                    + match self.pin.is_high().unwrap() {
                        true => 1,
                        false => 0,
                    };
                if self.debounce == 0b11111111 {
                    let last_state = self.state;
                    self.state = true;
                    self.state_changed = last_state != self.state;
                } else if self.debounce == 0b00000000 {
                    let last_state = self.state;
                    self.state = false;
                    self.state_changed = last_state != self.state;
                }
                self.next_tick += self.update_interval;
            }
            self.state
        }
        pub fn has_changed(&mut self) -> bool {
            let state_changed = self.state_changed;
            self.state_changed = false;
            state_changed
        }
    }
}
