pub mod output_channel {
    use embedded_hal::digital::OutputPin;
    use rp2040_hal::gpio::{DynPinId, FunctionSioOutput, Pin, PullDown};

    //     STATE:  IDLE  (new data)->   PAUSE   DATASET   ENABLESET    PAUSE
    //
    //     DATA:              hi or lo  ________|‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾ -> unchanged
    //
    //     ENABLE: _______________________________________|‾‾‾‾‾‾‾‾‾‾‾‾|_______

    const OUTPUT_TICK_INTERVAL: u64 = 10_000u64; // 10ms tick interval

    enum OutputChannelState {
        Idle,
        DataSet,
        EnableSet,
        Pause,
    }

    pub struct OutputChannel {
        data_pin: Pin<DynPinId, FunctionSioOutput, PullDown>, // data pin
        enable_pin: Pin<DynPinId, FunctionSioOutput, PullDown>, // enable pin
        state: OutputChannelState,                            // state of channel
        data: i16,                                            // number to output
        bit: u8,                                              // current bit
        next_tick: u64,                                       // next tick to output
        reverse: bool,                                        // reverse output bits
        last: bool, // double output last bit to trigger last shift
    }

    impl OutputChannel {
        pub fn new(
            data_pin: Pin<DynPinId, FunctionSioOutput, PullDown>,
            enable_pin: Pin<DynPinId, FunctionSioOutput, PullDown>,
        ) -> Self {
            OutputChannel {
                data_pin,
                enable_pin,
                state: OutputChannelState::Pause,
                data: 0,
                bit: 0,
                next_tick: 0,
                reverse: false,
                last: false,
            }
        }

        pub fn set(&mut self, data: i16) {
            self.data = data;
            self.bit = 0;
            self.next_tick = 0;
            self.state = OutputChannelState::Pause;
            self.last = false;
        }

        pub fn reverse(&mut self) {
            self.reverse = !self.reverse;
            self.bit = 0;
            self.next_tick = 0;
            self.state = OutputChannelState::Pause;
            self.last = false;
        }

        pub fn is_reversed(&self) -> bool {
            self.reverse
        }

        pub fn update(&mut self, now: u64) {
            if now > self.next_tick {
                match self.state {
                    OutputChannelState::Idle => {}
                    OutputChannelState::Pause => {
                        let text_bit: i16;
                        if self.reverse {
                            text_bit = 1 << (15 - self.bit);
                        } else {
                            text_bit = 1 << self.bit;
                        }
                        if self.data & text_bit != 0 {
                            self.data_pin.set_high().unwrap();
                        } else {
                            self.data_pin.set_low().unwrap();
                        }
                        self.bit += 1;
                        self.next_tick = now + OUTPUT_TICK_INTERVAL;
                        self.state = OutputChannelState::DataSet;
                    }
                    OutputChannelState::DataSet => {
                        self.enable_pin.set_high().unwrap();
                        self.next_tick = now + OUTPUT_TICK_INTERVAL;
                        self.state = OutputChannelState::EnableSet;
                    }
                    OutputChannelState::EnableSet => {
                        self.enable_pin.set_low().unwrap();
                        self.next_tick = now + OUTPUT_TICK_INTERVAL;
                        if self.bit < 16 {
                            self.state = OutputChannelState::Pause;
                        } else {
                            if self.last == false {
                                self.last = true;
                                self.bit = 15;
                                self.next_tick = now + OUTPUT_TICK_INTERVAL;
                                self.state = OutputChannelState::Pause;
                            } else {
                                self.state = OutputChannelState::Idle;
                                self.last = false;
                            }
                        }
                    }
                }
            }
        }
    }
}
