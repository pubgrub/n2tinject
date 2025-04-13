#![no_std]
#![no_main]

mod button;
mod clock;
mod format_str;
mod input_channel;
mod output_channel;
mod program;
//mod string;

use format_str::format_str::{DataText, ScrollText, StaticPageText};

use core::str;
use core::{fmt::Write, str::FromStr};
use dyn_fmt;
use embedded_hal::digital::{OutputPin, StatefulOutputPin as _};
use heapless::{String, Vec};
use panic_halt as _;
use rand_chacha::ChaCha8Rng;
use rand_core::{RngCore, SeedableRng};
use rp2040_hal::{
    clocks::init_clocks_and_plls,
    gpio::{DynPinId, FunctionSioOutput, Pin, PinState, PullDown},
    pac,
    sio::Sio,
    watchdog::Watchdog,
    Timer,
};
use usb_device::{class_prelude::*, prelude::*};
use usbd_serial::{SerialPort, USB_CLASS_CDC};

use button::button::Button;
use clock::clock::Clock;
use input_channel::input_channel::InputChannel;
use output_channel::output_channel::OutputChannel;
use program::program::Program;
//use string::string::String;

const PAGE_LINES: usize = format_str::format_str::PAGE_LINES;
const PAGE_STR_WIDTH: usize = format_str::format_str::PAGE_STR_WIDTH;

#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GENERIC_03H;

static OUTPUT_CHANNELS: usize = 4;

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

fn u8_to_str(n: u8) -> Buffer {
    let mut buf = Buffer::new();
    write!(&mut buf, "{}", n).unwrap();
    buf
}

fn u16_to_str(n: u16) -> Buffer {
    let mut buf = Buffer::new();
    write!(&mut buf, "{}", n).unwrap();
    buf
}

fn tokenize(input: String<PAGE_STR_WIDTH>) -> Vec<String<PAGE_STR_WIDTH>, 4> {
    let mut tokens: Vec<String<PAGE_STR_WIDTH>, 4> = Vec::new();
    for i in input.split(" ") {
        if i.len() > 0 {
            let mut token = String::new();
            token.push_str(i);
            tokens.push(token).unwrap();
        }
    }
    tokens
}

fn get_channels_from_text(text: &String<PAGE_STR_WIDTH>) -> [bool; OUTPUT_CHANNELS] {
    let mut channels: [bool; OUTPUT_CHANNELS] = [false; OUTPUT_CHANNELS];
    for &t in text.as_bytes() {
        if let Some(channel) = (t as char).to_digit(10) {
            if channel >= 1 && channel <= OUTPUT_CHANNELS as u32 {
                channels[channel as usize - 1] = true;
            }
        }
    }
    channels
}

#[rp2040_hal::entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);

    // Initialise System Clock
    let sys_clocks = init_clocks_and_plls(
        12_000_000u32,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .unwrap();

    let timer = Timer::new(pac.TIMER, &mut pac.RESETS, &sys_clocks);

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
        sys_clocks.usb_clock,
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

    // On-Board blinking LED
    let mut blink_pin = pins.gpio25.into_push_pull_output_in_state(PinState::High);

    // Data Output Pins Channels 1 - 4
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

    // Clock Button Pins
    let pin_12 = pins.gpio12.into_pull_down_input().into_dyn_pin();
    let pin_13 = pins.gpio13.into_pull_down_input().into_dyn_pin();

    // Clock Pulse Output Pins
    let pin_10 = pins
        .gpio10
        .into_push_pull_output_in_state(PinState::Low)
        .into_dyn_pin();
    let pin_11 = pins
        .gpio11
        .into_push_pull_output_in_state(PinState::Low)
        .into_dyn_pin();

    // Input Channel Pins
    let pin_6 = pins
        .gpio6
        .into_push_pull_output_in_state(PinState::Low)
        .into_dyn_pin();
    let pin_8 = pins
        .gpio8
        .into_push_pull_output_in_state(PinState::Low)
        .into_dyn_pin();
    let pin_9 = pins
        .gpio9
        .into_push_pull_output_in_state(PinState::Low)
        .into_dyn_pin();
    let pin_7 = pins.gpio7.into_pull_down_input().into_dyn_pin();

    //setup output channels
    let output_channel_1 = OutputChannel::new(pin_14, pin_15);
    let output_channel_2 = OutputChannel::new(pin_16, pin_17);
    let output_channel_3 = OutputChannel::new(pin_18, pin_19);
    let output_channel_4 = OutputChannel::new(pin_20, pin_21);

    let mut output_channels: [OutputChannel; OUTPUT_CHANNELS] = [
        output_channel_1,
        output_channel_2,
        output_channel_3,
        output_channel_4,
    ];

    let blink_interval = 1_000_000u64;
    let mut blink_last = 0u64;

    let mut now;

    let answer_str = "Answer:".as_bytes();
    let mut all_channels_str = String::new();
    for i in 0..OUTPUT_CHANNELS {
        all_channels_str
            .push(((i as u8 + 1) + 0x30) as char)
            .unwrap();
    }

    // setup buttons
    let button1 = Button::new(pin_12);
    let button2 = Button::new(pin_13);

    // setup clocks
    let mut clock1 = Clock::new(pin_10, button1);
    let mut clock2 = Clock::new(pin_11, button2);

    let mut clocks = [&mut clock1, &mut clock2];

    // setup input channel
    let mut input_channel = InputChannel::new(pin_6, pin_8, pin_9, pin_7);

    // setup programs
    // Program 0 is special, will be caught by the main loop
    const PROGRAMS: usize = 6;
    let program00 = Program::new(String::from_str("Manual").unwrap(), 0, [0, 0]);
    let program01 = Program::new(String::from_str("Sync").unwrap(), 3, [0b011, 0b011]);
    let program02 = Program::new(String::from_str("Opp Sync").unwrap(), 5, [0b01100, 0b00011]);
    let program03 = Program::new(String::from_str("Inner").unwrap(), 5, [0b01111, 0b00110]);
    let program04 = Program::new(String::from_str("Overlap").unwrap(), 4, [0b0110, 0b0011]);
    let program05 = Program::new(
        String::from_str("Sequential").unwrap(),
        6,
        [0b011000, 0b000011],
    );

    let mut programs: [Program; PROGRAMS] = [
        program00, program01, program02, program03, program04, program05,
    ];

    let mut active_program = 0;

    loop {
        now = timer.get_counter().ticks();

        // handle blinking LED
        if now > blink_last + blink_interval {
            blink_pin.toggle().unwrap();
            blink_last = now;
        }

        // handle output channels
        for channel in output_channels.iter_mut() {
            channel.update(now);
        }

        // handle clocks
        for clock in clocks.iter_mut() {
            clock.update(now);
        }

        // handle input channel
        input_channel.update(now);
        let _ = serial.write(&u16_to_str(input_channel.data).buf);

        let _ = serial.write(&u16_to_str(input_channel.data).buf);
        if input_channel.data_changed {
            let data = input_channel.data;
            let data_str = u16_to_str(data);
            let _ = serial.write(&data_str.buf);
            input_channel.data_changed = false;
        }

        // handle USB communication
        if !usb_dev.poll(&mut [&mut serial]) {
            //            debug!("waiting...");
            continue;
        }

        // get keyboard input
        let mut buf = [b' '; 64];

        // Handle Input
        //
        // any input found?
        if let Ok(count) = serial.read(&mut buf) {
            let mut input_str: String<PAGE_STR_WIDTH> = String::new();
            for i in 0..count {
                input_str.push(buf[i] as char).unwrap();
            }

            let mut tokens = tokenize(input_str); // split input into tokens
            let mut changed = true;
            while changed {
                changed = false;

                match tokens[0].as_str() {
                    // for all channels
                    "z" | "0" | "r" => {
                        tokens[1] = tokens[0].clone();
                        tokens[0] = all_channels_str.clone();
                        changed = true;
                    }

                    "c" => {
                        match tokens[1].as_str() {
                            "sync" => {
                                match tokens[2].as_str() {
                                    // c sync opp
                                    "opp" => {
                                        let (clock1, clock2) = clocks.split_at_mut(1);
                                        clock1[0].sync_opposite(&mut clock2[0]);
                                    }
                                    // c sync
                                    _ => {
                                        let (clock1, clock2) = clocks.split_at_mut(1);
                                        clock1[0].sync(&mut clock2[0]);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    "c1" | "c2" => {
                        let clock_index = match tokens[0].as_str() {
                            "c1" => 0,
                            "c2" => 1,
                            _ => 0,
                        };
                        match tokens[1].as_str() {
                            // cx auto
                            "auto" => {
                                clocks[clock_index].auto = true;
                                clocks[clock_index].next_tick = now;
                                clocks[clock_index].state = false;
                            }
                            // cx on
                            "on" => {
                                clocks[clock_index].auto = false;
                                clocks[clock_index].state = true;
                            }
                            // cx off
                            "off" => {
                                clocks[clock_index].auto = false;
                                clocks[clock_index].state = false;
                            }
                            // cx f y
                            "f" => {
                                // is other a number?
                                if let Ok(num) = &tokens[2].trim_end().parse::<u64>() {
                                    clocks[clock_index].set_freq(num);
                                }
                            }
                            _ => {}
                        }
                    }
                    "p" => {
                        // p x
                        if let Ok(num) = &tokens[1].trim_end().parse::<u8>() {
                            if *num > PROGRAMS as u8 {
                            } else {
                                active_program = *num as usize;
                            }
                        }
                    }

                    // starts with channel numbers
                    _ => {
                        let mut active_channels = get_channels_from_text(&tokens[0]);
                        for (i, channel) in active_channels.iter_mut().enumerate() {
                            if *channel {
                                match tokens[1].as_str() {
                                    // reverse bit order
                                    // x r
                                    "r" => {
                                        output_channels[i].reverse();
                                    }
                                    // x y
                                    _ => {
                                        // is other a number?
                                        if let Ok(num) = &tokens[1].trim_end().parse::<i16>() {
                                            let _ = serial.write("got a number".as_bytes());

                                            output_channels[i].set(*num);
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
