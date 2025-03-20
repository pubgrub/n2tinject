#![no_std]
#![no_main]

use core::fmt::Write;
use embedded_hal::digital::{OutputPin, StatefulOutputPin as _};
use panic_halt as _;
use rp2040_hal::{
    clocks::init_clocks_and_plls,
    gpio::{DynPinId, FunctionSioOutput, Pin, PinState, PullDown},
    pac::{self, dma::ch},
    sio::Sio,
    watchdog::Watchdog,
    Timer,
};
use usb_device::{class_prelude::*, prelude::*};
use usbd_serial::{SerialPort, USB_CLASS_CDC};

#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GENERIC_03H;

static CHANNELS: usize = 4;

struct Buffer {
    buf: [u8; 20],
    pos: usize,
}

impl Buffer {
    fn new() -> Self {
        Buffer {
            buf: [0u8; 20],
            pos: 0,
        }
    }

    fn clear(&mut self) {
        self.pos = 0;
    }

    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[0..self.pos]).unwrap()
    }
}

impl Write for Buffer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        let len = bytes.len().min(self.buf.len() - self.pos);
        self.buf[self.pos..self.pos + len].copy_from_slice(&bytes[0..len]);
        self.pos += len;
        Ok(())
    }
}

fn u64_to_str(n: u64) -> Buffer {
    let mut buf = Buffer::new();
    write!(&mut buf, "{}", n).unwrap();
    buf
}

fn setup_systick(syst: &mut pac::SYST, reload_value: u32) {
    syst.set_clock_source(cortex_m::peripheral::syst::SystClkSource::Core);
    syst.set_reload(reload_value); // Must be ≤ 0xFFFFFF (2^24 - 1)
    syst.clear_current(); // Reset counter
    syst.enable_counter(); // Start counting
}

fn tokenize(input: [u8; 64]) -> [[u8; 64]; 4] {
    let mut tokens: [[u8; 64]; 4] = [[b' '; 64]; 4];
    let mut token: [u8; 64] = [b' '; 64];
    let mut token_index = 0;
    let mut token_count = 0;
    input = input.trim_end().to_owned();
    for i in 0..64 {
        if input[i] == 0x20 {
            tokens[token_count] = token;
            token = [0; 64];
            token_index = 0;
            token_count += 1;
        } else {
            token[token_index] = input[i];
            token_index += 1;
        }
    }
    tokens[token_count] = token;
    tokens
}

fn get_channels_from_text(text: [u8; 64]) -> [bool; CHANNELS] {
    let mut channels: [bool; CHANNELS] = [false; CHANNELS];
    for &t in text.iter() {
        if let Some(channel) = (t as char).to_digit(10) {
            if channel >= 1 && channel <= CHANNELS as u32 {
                channels[channel as usize - 1] = true;
            }
        }
    }
    channels
}

#[rp2040_hal::entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);

    // Initialisiere Systemtakt
    let clocks = init_clocks_and_plls(
        12_000_000u32,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .unwrap();

    let timer = Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

    let sio = Sio::new(pac.SIO);
    let pins = rp2040_hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    //USB-Device Setup
    let usb_bus = UsbBusAllocator::new(rp2040_hal::usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    let mut serial = SerialPort::new(&usb_bus);
    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
        //        .manufacturer("RP2040")
        //  .product("USB-Serial")
        //.serial_number("123456")
        .device_class(USB_CLASS_CDC)
        .build();

    let mut blink_pin = pins.gpio25.into_push_pull_output_in_state(PinState::High);

    let pin_14 = pins
        .gpio14
        .into_push_pull_output_in_state(PinState::Low)
        .into_dyn_pin();
    let pin_15 = pins
        .gpio15
        .into_push_pull_output_in_state(PinState::Low)
        .into_dyn_pin();
    let pin_16 = pins
        .gpio16
        .into_push_pull_output_in_state(PinState::Low)
        .into_dyn_pin();
    let pin_17 = pins
        .gpio17
        .into_push_pull_output_in_state(PinState::Low)
        .into_dyn_pin();
    let pin_18 = pins
        .gpio18
        .into_push_pull_output_in_state(PinState::Low)
        .into_dyn_pin();
    let pin_19 = pins
        .gpio19
        .into_push_pull_output_in_state(PinState::Low)
        .into_dyn_pin();
    let pin_20 = pins
        .gpio20
        .into_push_pull_output_in_state(PinState::Low)
        .into_dyn_pin();
    let pin_21 = pins
        .gpio21
        .into_push_pull_output_in_state(PinState::Low)
        .into_dyn_pin();

    enum ChannelState {
        Idle,
        DataSet,
        EnableSet,
        Pause,
    }

    struct Channel {
        data_pin: Pin<DynPinId, FunctionSioOutput, PullDown>, // data pin
        enable_pin: Pin<DynPinId, FunctionSioOutput, PullDown>, // enable pin
        state: ChannelState,                                  // state of channel
        data: u16,                                            // number to output
        bit: u8,                                              // current bit
        next_tick: u64,                                       // next tick to output
        reverse: bool,                                        // reverse output bits
        last: bool, // double output last bit to trigger last shift
    }

    let mut channel1 = Channel {
        data_pin: pin_14,
        enable_pin: pin_15,
        state: ChannelState::Idle,
        data: 0,
        bit: 0,
        next_tick: 0,
        reverse: false,
        last: false,
    };
    let mut channel2 = Channel {
        data_pin: pin_16,
        enable_pin: pin_17,
        state: ChannelState::Idle,
        data: 0,
        bit: 0,
        next_tick: 0,
        reverse: false,
        last: false,
    };
    let mut channel3 = Channel {
        data_pin: pin_18,
        enable_pin: pin_19,
        state: ChannelState::Idle,
        data: 0,
        bit: 0,
        next_tick: 0,
        reverse: false,
        last: false,
    };
    let mut channel4 = Channel {
        data_pin: pin_20,
        enable_pin: pin_21,
        state: ChannelState::Idle,
        data: 0,
        bit: 0,
        next_tick: 0,
        reverse: false,
        last: false,
    };

    let mut channels: [&mut Channel; CHANNELS] =
        [&mut channel1, &mut channel2, &mut channel3, &mut channel4];

    let blink_interval = 1_000_000u64;
    let mut blink_last = 0u64;

    let t1 = 10_000u64; // time from data out to enable on and from enable off to end of cycle
    let t2 = 10_000u64; // time from enable on to enable off

    let mut now;

    let answer_str = "Answer:".as_bytes();
    let empty_command = [0u8; 64];
    let mut all_channels_str = empty_command;
    for i in 0..CHANNELS {
        all_channels_str[i] = (i as u8 + 1) + 0x30;
    }

    //test data
    channels[0].data = 0b1111011101101;
    channels[0].state = ChannelState::Pause;
    channels[0].next_tick = 0;
    channels[0].bit = 0;
    channels[0].reverse = true;

    loop {
        now = timer.get_counter().ticks();

        // handle blinking LED
        if now > blink_last + blink_interval {
            blink_pin.toggle().unwrap();
            blink_last = now;
        }

        // handle output channels
        for channel in channels.iter_mut() {
            match channel.state {
                ChannelState::Idle => {
                    // do nothing
                }
                ChannelState::Pause => {
                    if now > channel.next_tick {
                        let test_bit;
                        if channel.reverse {
                            test_bit = 1 << (15 - channel.bit);
                        } else {
                            test_bit = 1 << channel.bit;
                        }
                        if channel.data & test_bit != 0 {
                            channel.data_pin.set_high().unwrap();
                        } else {
                            channel.data_pin.set_low().unwrap();
                        }
                        channel.bit += 1;
                        channel.next_tick = now + t1;
                        channel.state = ChannelState::DataSet;
                    }
                }
                ChannelState::DataSet => {
                    if now > channel.next_tick {
                        channel.enable_pin.set_high().unwrap();
                        channel.next_tick = now + t2;
                        channel.state = ChannelState::EnableSet;
                    }
                }
                ChannelState::EnableSet => {
                    if now > channel.next_tick {
                        channel.enable_pin.set_low().unwrap();
                        channel.next_tick = now + t2;
                        if channel.bit < 16 {
                            channel.state = ChannelState::Pause;
                        } else {
                            if channel.last == false {
                                channel.last = true;
                                channel.bit = 15;
                                channel.next_tick = now + t1;
                                channel.state = ChannelState::Pause;
                            } else {
                                channel.state = ChannelState::Idle;
                                channel.last = false;
                            }
                        }
                    }
                }
            }
        }

        // handle USB communication
        if !usb_dev.poll(&mut [&mut serial]) {
            //            debug!("waiting...");
            continue;
        }

        let mut buf = [b' '; 64];

        if let Ok(count) = serial.read(&mut buf) {
            // Echo zurücksenden

            let mut tokens = tokenize(buf); // split input into tokens
            let mut changed = true;
            while changed {
                changed = false;
                match tokens[0][0] {
                    b'z' | b'0' | b'r' => {
                        tokens[1] = tokens[0];
                        tokens[0] = all_channels_str;
                        changed = true;
                    }
                    _ => {
                        let mut active_channels = get_channels_from_text(tokens[0]);
                        for (i, channel) in active_channels.iter_mut().enumerate() {
                            if *channel {
                                match tokens[1][0] {
                                    b'0' | b'z' => {
                                        channels[i].state = ChannelState::Pause;
                                        channels[i].next_tick = 0;
                                        channels[i].bit = 0;
                                        channels[i].data = 0;
                                    }
                                    b'r' => {
                                        channels[i].state = ChannelState::Pause;
                                        channels[i].next_tick = 0;
                                        channels[i].bit = 0;
                                        channels[i].reverse = !channels[i].reverse;
                                    }
                                    _ => {
                                        // is other a number?
                                        if let Ok(num) = core::str::from_utf8(&tokens[1])
                                            .unwrap()
                                            .trim_end()
                                            .parse::<u16>()
                                        {
                                            let _ = serial.write("got a number".as_bytes());

                                            channels[i].data = num;
                                            channels[i].state = ChannelState::Pause;
                                            channels[i].next_tick = 0;
                                            channels[i].bit = 0;
                                        } else {
                                            let _ = serial.write("got something else".as_bytes());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            let _ = serial.write(&u64_to_str(now / 1000).buf);
            let _ = serial.write(answer_str);
            let _ = serial.write(&buf[0..count]);
        }
    }
}
