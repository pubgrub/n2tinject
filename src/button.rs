pub mod button {

    use embedded_hal::digital::InputPin;
    use panic_halt as _;
    use rp2040_hal::gpio::{DynPinId, FunctionSioInput, Pin, PullDown};

    pub struct ButtonState {
        pub state: bool,
        pub state_changed: bool,
    }

    pub struct Button {
        pin: Pin<DynPinId, FunctionSioInput, PullDown>,
        pub state: bool,
        update_interval: u64,
        debounce: u8,
        next_tick: u64,
    }

    impl Button {
        pub fn new(pin: Pin<DynPinId, FunctionSioInput, PullDown>) -> Self {
            Button {
                pin,
                state: false,
                update_interval: 1000,
                next_tick: 0,
                debounce: 0,
            }
        }

        pub fn update(&mut self, now: u64) -> ButtonState {
            let mut state_changed = false;
            if now > self.next_tick {
                self.debounce = (self.debounce << 1)
                    + match self.pin.is_high().unwrap() {
                        true => 1,
                        false => 0,
                    };
                if self.debounce == 0b11111111 {
                    let last_state = self.state;
                    self.state = true;
                    state_changed = last_state != self.state;
                } else if self.debounce == 0b00000000 {
                    let last_state = self.state;
                    self.state = false;
                    state_changed = last_state != self.state;
                }
                self.next_tick = now + self.update_interval;
            }
            ButtonState {
                state: self.state,
                state_changed,
            }
        }
    }
}
