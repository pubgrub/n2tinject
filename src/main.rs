#![no_std]
#![no_main]

mod button;
mod clock;
mod format_str;
mod input_channel;
mod output_channel;
mod program;
use core::array::from_fn;

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
use program::program::{Program, ProgramControl, ProgramMode};
//use string::string::String;

const CRYSTAL_FREQ: u32 = 12_000_000; // System frequency in Hz
const TICKS_SECOND: u32 = 1_000_000; // USB Vendor ID

const PAGE_LINES: usize = format_str::format_str::PAGE_LINES;
const PAGE_STR_WIDTH: usize = format_str::format_str::PAGE_STR_WIDTH;
const PAGE_WIDTH: usize = format_str::format_str::PAGE_WIDTH;

const MAX_PROGRAMS: usize = program::program::MAX_PROGRAMS; // Maximum number of programs

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
        CRYSTAL_FREQ,
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
    let mut output_channels: [OutputChannel; OUTPUT_CHANNELS] = [
        OutputChannel::new(pin_14, pin_15),
        OutputChannel::new(pin_16, pin_17),
        OutputChannel::new(pin_18, pin_19),
        OutputChannel::new(pin_20, pin_21),
    ];

    let blink_interval = 1_000_000u64;
    let mut blink_last = 0u64;

    let answer_str = "Answer:".as_bytes();
    let mut all_channels_str = String::new();
    for i in 0..OUTPUT_CHANNELS {
        all_channels_str
            .push(((i as u8 + 1) + 0x30) as char)
            .unwrap();
    }

    // setup buttons
    let buttons: [Button; 2] = [Button::new(pin_12), Button::new(pin_13)];

    // setup clocks
    let mut now = timer.get_counter().ticks();

    let clocks = [Clock::new(pin_10, now), Clock::new(pin_11, now)];

    // setup input channel
    let mut input_channel = InputChannel::new(pin_6, pin_8, pin_9, pin_7);

    // setup programs
    // Program 0 is special, will be caught by the main loop
    let mut prog = ProgramControl::new(TICKS_SECOND, clocks, buttons);

    prog.add_program(Program::new(
        String::from_str("Sync").unwrap(),
        2,
        [0b01, 0b01],
    ));
    prog.add_program(Program::new(
        String::from_str("Opp Sync").unwrap(),
        2,
        [0b10, 0b01],
    ));
    prog.add_program(Program::new(
        String::from_str("Inner").unwrap(),
        4,
        [0b0111, 0b0010],
    ));
    prog.add_program(Program::new(
        String::from_str("Overlap").unwrap(),
        4,
        [0b0110, 0b0011],
    ));
    prog.add_program(Program::new(
        String::from_str("Sequential").unwrap(),
        4,
        [0b0100, 0b0001],
    ));

    let screen_str = [
        "_____________________________________________________________________________",
        "                                                                             ",
        " OUT 1  -12345  0xFFFF  0b0000111100001111  |                     Mode  Freq ",
        " OUT 2  -12345  0xFFFF  0b0000111100001111  |                                ",
        " OUT 3  -12345  0xFFFF  0b0000111100001111  |  PROG 0 NAME______  AUTO    50 ",
        " OUT 4  -12345  0xFFFF  0b0000111100001111  |                                ",
        "                                            |  CLOCK 1    __XX__  AUTO    20 ",
        " IN     -12345  0xFFFF  0b0000111100001111  |  CLOCK 2    __XX__  AUTO    30 ",
        "_____________________________________________________________________________",
        "                                                                             ",
        "                                                                             ",
        "                                                                             ",
        "                                                                             ",
        "                                                                             ",
        "                                                                             ",
        "                                                                             ",
        "                                                                             ",
        "                                                                             ",
    ];

    // wait for USB monitor
    // while !serial.dtr() {
    //     usb_dev.poll(&mut [&mut serial]);
    // }

    let mut screen: StaticPageText = StaticPageText::new(
        from_fn(|i| {
            let s = String::from_str(screen_str[i]).unwrap();
            let _ = serial.write(&[i as u8]).unwrap();
            s
        }),
        0,
        1,
    );

    // clear screen
    serial.write("\x1B[2J\x1B[H".as_bytes()).unwrap();
    // print background
    for l in screen.get_lines() {
        let _ = serial.write(l.as_bytes());
    }

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

        // handle program control

        prog.update(now);

        // handle input channel
        input_channel.update(now);
        // let _ = serial.write(&u16_to_str(input_channel.data).buf);

        // let _ = serial.write(&u16_to_str(input_channel.data).buf);
        if input_channel.data_changed {
            let data = input_channel.data;
            let data_str = u16_to_str(data);
            //let _ = serial.write(&data_str.buf);
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
                    "a" => {
                        // a
                        if prog.get_current_program() > 0 {
                            prog.mode = match prog.mode {
                                ProgramMode::Manual => ProgramMode::Auto,
                                ProgramMode::Auto => ProgramMode::Manual,
                                ProgramMode::OneShot => ProgramMode::Auto,
                            };
                            prog.reset_state();
                        } else {
                            // MSG TODO
                        }
                    }

                    // f x
                    "f" => {
                        if prog.get_current_program() > 0 {
                            // is other a number?
                            if let Ok(num) = &tokens[1].trim_end().parse::<u64>() {
                                prog.set_freq(*num as u32);
                            }
                        } else {
                            // MSG TODO
                        }
                    }

                    "c" => {
                        if prog.get_current_program() == 0 {
                            match tokens[1].as_str() {
                                "s" => {
                                    prog.clocks_sync();
                                }
                                "so" => {
                                    prog.clocks_sync_opposite();
                                }
                                "a" => {
                                    for i in 0..2 {
                                        prog.clock_toggle_auto(i);
                                    }
                                }
                                "f" => {
                                    // is other a number?
                                    if let Ok(num) = &tokens[2].trim_end().parse::<u64>() {
                                        for i in 0..2 {
                                            prog.clock_set_freq(i, num);
                                        }
                                    }
                                }
                                _ => {}
                            }
                        } else {
                            // MSG TODO
                        }
                    }

                    "c1" | "c2" => {
                        if prog.get_current_program() == 0 {
                            let clock_index = match tokens[0].as_str() {
                                "c1" => 0,
                                "c2" => 1,
                                _ => 0,
                            };
                            match tokens[1].as_str() {
                                // cx a
                                "a" => {
                                    prog.clock_toggle_auto(clock_index);
                                }
                                // cx f y
                                "f" => {
                                    // is other a number?
                                    if let Ok(num) = &tokens[2].trim_end().parse::<u64>() {
                                        prog.clock_set_freq(clock_index, num);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    "p" => {
                        // p x
                        if let Ok(num) = &tokens[1].trim_end().parse::<u8>() {
                            if *num > prog.number_of_programs() as u8 {
                                // MSG TODO
                            } else {
                                prog.set_program(*num as usize);
                                prog.reset_state();
                                prog.mode = ProgramMode::Manual;
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
