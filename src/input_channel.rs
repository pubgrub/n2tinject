pub mod input_channel {

    use embedded_hal::digital::{InputPin, OutputPin};
    use panic_halt as _;
    use rp2040_hal::gpio::{DynPinId, FunctionSioInput, FunctionSioOutput, Pin, PullDown};

    pub enum InputChannelState {
        Idle,
        LoadingData,
        SerialShiftOff,
        ShiftClockOn,
    }

    pub struct InputChannel {
        pin_load_data: Pin<DynPinId, FunctionSioOutput, PullDown>,
        pin_serial_shift: Pin<DynPinId, FunctionSioOutput, PullDown>,
        pin_shift_clock: Pin<DynPinId, FunctionSioOutput, PullDown>,
        pin_data_in: Pin<DynPinId, FunctionSioInput, PullDown>,
        pub state: InputChannelState,
        temp_data: i16,
        pub data: i16,
        old_data: i16,
        temp_data_mirrored: i16,
        pub mirrored: bool,
        pub data_mirrored: i16,
        next_tick: u64,
        tick_interval: u64,
        bit_count: u8,
        pub data_changed: bool,
    }

    impl InputChannel {
        pub fn new(
            pin_load_data: Pin<DynPinId, FunctionSioOutput, PullDown>,
            pin_serial_shift: Pin<DynPinId, FunctionSioOutput, PullDown>,
            pin_shift_clock: Pin<DynPinId, FunctionSioOutput, PullDown>,
            pin_data_in: Pin<DynPinId, FunctionSioInput, PullDown>,
            now: u64,
        ) -> Self {
            InputChannel {
                pin_load_data,
                pin_serial_shift,
                pin_shift_clock,
                pin_data_in,
                state: InputChannelState::Idle,
                temp_data: 0,
                data: 0,
                old_data: 0,
                temp_data_mirrored: 0,
                mirrored: false,
                data_mirrored: 0,
                next_tick: now,
                tick_interval: 1_000,
                bit_count: 0,
                data_changed: true,
            }
        }

        pub fn update(&mut self, now: u64) {
            if now > self.next_tick {
                match self.state {
                    InputChannelState::Idle => {
                        self.pin_load_data.set_high().unwrap();
                        self.pin_serial_shift.set_low().unwrap();
                        self.pin_shift_clock.set_low().unwrap();
                        self.state = InputChannelState::LoadingData;
                        self.next_tick += self.tick_interval;
                    }
                    InputChannelState::LoadingData => {
                        self.pin_load_data.set_low().unwrap();
                        self.pin_serial_shift.set_high().unwrap();
                        self.state = InputChannelState::SerialShiftOff;
                        self.next_tick += self.tick_interval;
                    }
                    InputChannelState::SerialShiftOff => {
                        self.temp_data =
                            (self.temp_data << 1) | self.pin_data_in.is_high().unwrap() as i16;
                        self.temp_data_mirrored = (self.temp_data_mirrored >> 1)
                            | (self.pin_data_in.is_high().unwrap() as i16) << 15;
                        self.bit_count += 1;
                        if self.bit_count == 16 {
                            self.bit_count = 0;
                            self.old_data = self.data;
                            self.data = self.temp_data;
                            self.data_mirrored = self.temp_data_mirrored;
                            self.data_changed = self.old_data != self.data;
                            self.temp_data = 0;
                            self.temp_data_mirrored = 0;
                            self.state = InputChannelState::Idle;
                            self.next_tick += self.tick_interval;
                        } else {
                            self.pin_shift_clock.set_high().unwrap();
                            self.state = InputChannelState::ShiftClockOn;
                            self.next_tick += self.tick_interval;
                        }
                    }
                    InputChannelState::ShiftClockOn => {
                        self.pin_shift_clock.set_low().unwrap();
                        self.state = InputChannelState::SerialShiftOff;
                        self.next_tick += self.tick_interval;
                    }
                }
            }
        }
    }
}
