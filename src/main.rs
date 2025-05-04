#![no_std]
#![no_main]

mod button;
mod clock;
mod format_str;
mod input_channel;
mod output_channel;
mod program;
mod text_input;
use core::array::from_fn;

//mod string;

use format_str::format_str::{DataText, ScrollText, StaticPageText};

use core::{fmt::Write, str::FromStr};
use embedded_hal::digital::StatefulOutputPin as _;
use heapless::{String, Vec};
use panic_halt as _;
use rp2040_hal::{
    clocks::init_clocks_and_plls, gpio::PinState, pac, sio::Sio, watchdog::Watchdog, Timer,
};
use usb_device::{class_prelude::*, prelude::*};
use usbd_serial::{SerialPort, USB_CLASS_CDC};

use button::button::Button;
use clock::clock::{Clock, ClockMode};
use input_channel::input_channel::InputChannel;
use output_channel::output_channel::OutputChannel;
use program::program::{Program, ProgramControl, ProgramMode};
use text_input::text_input::{TextInput, TextInputState};
//use string::string::String;

const CRYSTAL_FREQ: u32 = 12_000_000; // System frequency in Hz
const TICKS_SECOND: u32 = 1_000_000; // USB Vendor ID

const PAGE_STR_WIDTH: usize = format_str::format_str::PAGE_STR_WIDTH;

#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GENERIC_03H;

static OUTPUT_CHANNELS: usize = 4;

fn tokenize(input: String<PAGE_STR_WIDTH>) -> Vec<String<PAGE_STR_WIDTH>, 4> {
    let mut tokens: Vec<String<PAGE_STR_WIDTH>, 4> = Vec::new();
    for s in input.trim().split(" ") {
        tokens.push(String::from_str(s).unwrap()).unwrap();
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
    /////////////////////////////////////
    // Setup Hardware
    /////////////////////////////////////

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

    /////////////////////////////////////
    // Setup USB
    /////////////////////////////////////

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

    //wait for USB monitor
    loop {
        // ── 1. service USB: needs &mut serial ───────────────────────────
        usb_dev.poll(&mut [&mut serial]);

        // ── 2. test DTR: needs only &serial ─────────────────────────────
        if serial.dtr() {
            break; // host terminal is ready
        }
    }

    // write to USB: needs &mut serial ───────────────────────────
    // write_all is a closure that writes all bytes to the serial port
    // it will block until all bytes are written
    // it will poll the USB stack until all bytes are written
    // it will return an error if the write fails
    // it will return Ok(()) if the write is successful
    // it will return Err(UsbError) if the write fails
    // it will return Ok(0) if the write is successful but no bytes were written
    // it will return Err(UsbError::WouldBlock) if the write would block
    let mut write_all = |mut buf: &[u8]| -> core::fmt::Result {
        while !buf.is_empty() {
            match serial.write(buf) {
                Ok(0) => {}                            // nothing accepted yet
                Ok(n) => buf = &buf[n..],              // advance by n bytes
                Err(UsbError::WouldBlock) => continue, // fifo full – just poll
                Err(_) => return Err(core::fmt::Error),
            }
            // let the USB stack move the IN packet to the host
            while !usb_dev.poll(&mut [&mut serial]) {}
        }
        Ok(())
    };

    /////////////////////////////////////
    // Setup Pins
    /////////////////////////////////////

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

    /////////////////////////////////////
    // Initialize timing (needed to set up clocks)
    /////////////////////////////////////

    let mut now = timer.get_counter().ticks();

    /////////////////////////////////////
    // "Connect" Pins to Logic
    /////////////////////////////////////

    //setup output channels
    let mut output_channels: [OutputChannel; OUTPUT_CHANNELS] = [
        OutputChannel::new(pin_14, pin_15),
        OutputChannel::new(pin_16, pin_17),
        OutputChannel::new(pin_18, pin_19),
        OutputChannel::new(pin_20, pin_21),
    ];
    for i in 0..OUTPUT_CHANNELS {
        output_channels[i].set(0);
    }

    // generate string with all output channel numbers
    let mut all_channels_str = String::new();
    for i in 0..OUTPUT_CHANNELS {
        all_channels_str
            .push(((i as u8 + 1) + 0x30) as char)
            .unwrap();
    }

    // setup clocks
    let clocks = [Clock::new(pin_10, now), Clock::new(pin_11, now)];

    // setup input channel
    let mut input_channel = InputChannel::new(pin_6, pin_8, pin_9, pin_7, now);

    // setup buttons
    let buttons: [Button; 2] = [Button::new(pin_12), Button::new(pin_13)];

    /////////////////////////////////////
    // Setup Watchdog LED
    /////////////////////////////////////

    let blink_interval = 1_000_000u64;
    let mut blink_last = 0u64;

    /////////////////////////////////////
    // Setup Programs
    /////////////////////////////////////

    // Program 0 is special, will be caught by the main loop
    let mut prog = ProgramControl::new(TICKS_SECOND, clocks, buttons);

    prog.add_program(Program::new(
        String::from_str("Sync      ").unwrap(),
        2,
        [0b01, 0b01],
    ));
    prog.add_program(Program::new(
        String::from_str("Opp Sync  ").unwrap(),
        2,
        [0b10, 0b01],
    ));
    prog.add_program(Program::new(
        String::from_str("Inner     ").unwrap(),
        4,
        [0b0111, 0b0010],
    ));
    prog.add_program(Program::new(
        String::from_str("Overlap   ").unwrap(),
        4,
        [0b0110, 0b0011],
    ));
    prog.add_program(Program::new(
        String::from_str("Sequential").unwrap(),
        4,
        [0b0100, 0b0001],
    ));

    /////////////////////////////////////
    // Setup Output Text
    /////////////////////////////////////

    // setup channel text format
    let channel_format_decimal: String<PAGE_STR_WIDTH> = String::from_str("{:>6}").unwrap();
    let channel_format_hex: String<PAGE_STR_WIDTH> = String::from_str("{:#06X}").unwrap();
    let channel_format_bin: String<PAGE_STR_WIDTH> = String::from_str("{:#018b}").unwrap();
    let channel_format_inverted: String<PAGE_STR_WIDTH> = String::from_str("{}").unwrap();

    struct ChannelDataText {
        data_text: [DataText; 4],
    }

    // setup output channel text
    let mut output_channel_data_text: [ChannelDataText; OUTPUT_CHANNELS] = [
        ChannelDataText {
            data_text: [
                DataText::new(channel_format_decimal.clone(), 9, 3, true),
                DataText::new(channel_format_hex.clone(), 17, 3, true),
                DataText::new(channel_format_bin.clone(), 25, 3, true),
                DataText::new(channel_format_inverted.clone(), 7, 3, true),
            ],
        },
        ChannelDataText {
            data_text: [
                DataText::new(channel_format_decimal.clone(), 9, 4, true),
                DataText::new(channel_format_hex.clone(), 17, 4, true),
                DataText::new(channel_format_bin.clone(), 25, 4, true),
                DataText::new(channel_format_inverted.clone(), 7, 4, true),
            ],
        },
        ChannelDataText {
            data_text: [
                DataText::new(channel_format_decimal.clone(), 9, 5, true),
                DataText::new(channel_format_hex.clone(), 17, 5, true),
                DataText::new(channel_format_bin.clone(), 25, 5, true),
                DataText::new(channel_format_inverted.clone(), 7, 5, true),
            ],
        },
        ChannelDataText {
            data_text: [
                DataText::new(channel_format_decimal.clone(), 9, 6, true),
                DataText::new(channel_format_hex.clone(), 17, 6, true),
                DataText::new(channel_format_bin.clone(), 25, 6, true),
                DataText::new(channel_format_inverted.clone(), 7, 6, true),
            ],
        },
    ];

    // setup input channel text
    let mut input_channel_data_text = ChannelDataText {
        data_text: [
            DataText::new(channel_format_decimal.clone(), 9, 8, true),
            DataText::new(channel_format_hex.clone(), 17, 8, true),
            DataText::new(channel_format_bin.clone(), 25, 8, true),
            DataText::new(channel_format_inverted.clone(), 7, 8, true),
        ],
    };

    // setup text input (command line)
    let mut input_buffer: TextInput = TextInput::new();
    let input_format = String::from_str("{}                ").unwrap();
    let mut input_data_text = DataText::new(input_format, 11, 17, false);

    // setup scroll text
    let mut scroll_text: ScrollText = ScrollText::new(1, 11);

    // setup static text
    let screen_str = [
        "_____________________________________________________________________________",
        "                                                                             ",
        " OUT 1  -12345  0xFFFF  0b0000111100001111  |                     Mode  Freq ",
        " OUT 2  -12345  0xFFFF  0b0000111100001111  |                                ",
        " OUT 3  -12345  0xFFFF  0b0000111100001111  |  PROG 0 NAME______  AUTO    10 ",
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
        "Command:                                                                     ",
        "                                                                             ",
    ];

    let mut screen: StaticPageText = StaticPageText::new(
        from_fn(|i| {
            let s: String<PAGE_STR_WIDTH> = String::from_str(screen_str[i]).unwrap();
            s
        }),
        0,
        1,
    );

    //setup program text
    let mut prog_number_data_text = DataText::new(String::from_str("{}").unwrap(), 53, 5, false);
    let mut prog_name_data_text = DataText::new(String::from_str("{}").unwrap(), 55, 5, false);
    let mut prog_mode_data_text = DataText::new(String::from_str("{}").unwrap(), 67, 5, false);
    let mut prog_freq_data_text = DataText::new(String::from_str("{:>4}").unwrap(), 73, 5, false);

    prog_number_data_text.set(&(prog.get_current_program() as i16), now);
    prog_name_data_text.set(prog.get_current_program_name(), now);
    prog_mode_data_text.set(&"    ", now);
    prog_freq_data_text.set(&(prog.prog_freq as i16), now);

    // setup clock text
    let mut clock_mode_data_text: [DataText; 2] = [
        DataText::new(String::from_str("{}").unwrap(), 67, 7, false),
        DataText::new(String::from_str("{}").unwrap(), 67, 8, false),
    ];
    let mut clock_freq_data_text: [DataText; 2] = [
        DataText::new(String::from_str("{:>4}").unwrap(), 73, 7, false),
        DataText::new(String::from_str("{:>4}").unwrap(), 73, 8, false),
    ];
    for i in 0..2 {
        clock_mode_data_text[i].set(&"    ", now);
        clock_freq_data_text[i].set(&(prog.clocks[i].freq as i16), now);
    }

    /////////////////////////////////////
    // Prepare Screen
    /////////////////////////////////////

    // clear screen
    write_all("\x1B[2J\x1B[H".as_bytes()).unwrap();

    // print background
    for l in screen.get_lines() {
        let _ = write_all(l.as_bytes());
    }

    // print initial values
    for i in 0..OUTPUT_CHANNELS {
        output_channel_data_text[i].data_text[0].set(&0, now);
        output_channel_data_text[i].data_text[1].set(&0, now);
        output_channel_data_text[i].data_text[2].set(&0, now);
        output_channel_data_text[i].data_text[3].set(&" ", now);
    }

    let _ = serial.write(prog_number_data_text.get_text().as_bytes());
    let _ = serial.write(prog_name_data_text.get_text().as_bytes());
    let _ = serial.write(prog_mode_data_text.get_text().as_bytes());
    let _ = serial.write(prog_freq_data_text.get_text().as_bytes());

    for i in 0..2 {
        let _ = serial.write(clock_mode_data_text[i].get_text().as_bytes());
        let _ = serial.write(clock_freq_data_text[i].get_text().as_bytes());
    }

    /////////////////////////////////////
    // Main Loop
    /////////////////////////////////////

    loop {
        now = timer.get_counter().ticks();

        ////////////////////////////////////////////
        // update State
        ////////////////////////////////////////////

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

        ////////////////////////////////////////////
        // update Screen
        ////////////////////////////////////////////

        // print output channels
        for i in 0..OUTPUT_CHANNELS {
            for format in 0..4 {
                output_channel_data_text[i].data_text[format].update(now);
                if output_channel_data_text[i].data_text[format].is_changed {
                    let _ = serial.write(
                        output_channel_data_text[i].data_text[format]
                            .get_text()
                            .as_str()
                            .as_bytes(),
                    );
                }
            }
        }

        // print input channel
        for format in 0..3 {
            if input_channel.data_changed {
                input_channel_data_text.data_text[format].set(&input_channel.data, now);
            }
            input_channel_data_text.data_text[format].update(now);
            if input_channel_data_text.data_text[format].is_changed {
                let _ = serial.write(
                    input_channel_data_text.data_text[format]
                        .get_text()
                        .as_str()
                        .as_bytes(),
                );
            }
        }
        input_channel.data_changed = false;

        // print scroll text
        if scroll_text.is_changed {
            for l in scroll_text.get_lines() {
                let _ = serial.write(l.as_str().as_bytes());
            }
            scroll_text.is_changed = false;
        }

        // set cursor position to input cursor
        let (x, y) = input_data_text.get_cursor();
        let mut out_str: String<PAGE_STR_WIDTH> = String::from_str("\x1B[").unwrap();
        write!(out_str, "{}", y).unwrap();
        out_str.push_str(";").unwrap();
        write!(out_str, "{}", x).unwrap();
        out_str.push_str("H").unwrap();

        let _ = serial.write(out_str.as_str().as_bytes());

        //////////////////////////////////////////////
        // handle command input
        //////////////////////////////////////////////

        // handle USB communication
        if !usb_dev.poll(&mut [&mut serial]) {
            continue;
        }

        // FROM HERE ONLY IF INPUT FROM USB DETECTED

        // get keyboard input
        let mut buf = [b' '; 64];

        // any input found?
        let mut input_str = String::new();
        if let Ok(count) = serial.read(&mut buf) {
            let mut complete = false;
            for i in 0..count {
                match input_buffer.add_char(buf[i]) {
                    TextInputState::Unchanged => {}
                    TextInputState::Changed => {
                        input_data_text.set(input_buffer.get_text(), now);
                        let _ = serial.write(input_data_text.get_text().as_str().as_bytes());
                    }
                    TextInputState::Done => {
                        complete = true;
                        input_str = input_buffer.get_text().clone();
                        let clear_string = input_buffer.clear();
                        input_data_text.set(&clear_string, now);
                        let _ = serial.write(input_data_text.get_text().as_bytes());
                        input_data_text.set(input_buffer.get_text(), now);
                        break;
                    }
                }
            }
            if complete {
                let mut tokens = tokenize(input_str); // split input into tokens
                let num_tokens = tokens.len();
                // no tokens?
                if num_tokens == 0 {
                    scroll_text.add_line("no command found");
                    continue;
                }

                // check if tokens are for all channels
                match tokens[0].as_str() {
                    // for all channels
                    "z" | "0" | "r" => {
                        if num_tokens == 4 {
                            let mut log_str: String<PAGE_STR_WIDTH> =
                                String::from_str("Too many tokens for '").unwrap();
                            write!(log_str, "{}", tokens[0]).unwrap();
                            log_str.push_str("'").unwrap();
                            scroll_text.add_line(&log_str);
                            continue;
                        } else {
                            tokens.insert(0, all_channels_str.clone()).ok();
                        }
                    }
                    _ => {}
                }
                match tokens[0].as_str() {
                    "a" => {
                        // a
                        if num_tokens > 1 {
                            scroll_text.add_line("Err: too many tokens for 'a'");
                            continue;
                        }
                        if prog.get_current_program() > 0 {
                            prog.mode = match prog.mode {
                                ProgramMode::Manual => ProgramMode::Auto,
                                ProgramMode::Auto => ProgramMode::Manual,
                                ProgramMode::OneShot => ProgramMode::Auto,
                            };
                            prog.reset_state();
                            prog_mode_data_text.set(
                                &match prog.mode {
                                    ProgramMode::Manual | ProgramMode::OneShot => "    ",
                                    ProgramMode::Auto => "AUTO",
                                },
                                now,
                            );
                            if prog.mode == ProgramMode::Auto {
                                prog.clock_set_auto(0, false);
                                prog.clock_set_auto(1, false);
                                for i in 0..2 {
                                    clock_mode_data_text[i].set(&"    ", now);
                                    let _ = serial.write(
                                        clock_mode_data_text[i].get_text().as_str().as_bytes(),
                                    );
                                }
                            }
                            let _ =
                                serial.write(prog_mode_data_text.get_text().as_str().as_bytes());
                        } else {
                            scroll_text.add_line("Err: to automate Prog 0 use c/c1/c2 a");
                        }
                    }

                    // f x
                    "f" => {
                        if num_tokens != 2 {
                            scroll_text.add_line("Err: 'f' command needs exactly 1 parameter");
                            continue;
                        }
                        if prog.get_current_program() > 0 {
                            // is other a number?
                            if let Ok(num) = &tokens[1].trim_end().parse::<u64>() {
                                prog.set_freq(*num as u32);
                                let mut log_str: String<PAGE_STR_WIDTH> =
                                    String::from_str("Clock frequencies set to ").unwrap();
                                write!(log_str, "{}", num).unwrap();
                                log_str.push_str(" dHz").unwrap();
                                scroll_text.add_line(&log_str);
                                prog_freq_data_text.set(&(*num as i16), now);
                                let _ = serial
                                    .write(prog_freq_data_text.get_text().as_str().as_bytes());
                            } else {
                                scroll_text.add_line("Err: no valid frequency found");
                            }
                        } else {
                            scroll_text.add_line("Err: to set Frequency in Prog 0 use c/c1/c2 f");
                        }
                    }

                    "c" => {
                        if num_tokens == 1 {
                            scroll_text.add_line("Err: 'c' command needs minimum 1 parameter");
                            continue;
                        }

                        if prog.get_current_program() == 0 {
                            match tokens[1].as_str() {
                                "s" | "so" | "a" => {
                                    if num_tokens != 2 {
                                        scroll_text.add_line(
                                            "Err: 'c s/s0/a' commands do not accept parameters",
                                        );
                                        continue;
                                    }
                                }
                                "f" => {
                                    if num_tokens != 3 {
                                        scroll_text
                                            .add_line("Err: 'c f' command needs 1 parameters");
                                        continue;
                                    }
                                }
                                _ => {}
                            }

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
                                        clock_mode_data_text[i].set(
                                            &match prog.clocks[i].mode {
                                                ClockMode::Manual => "    ",
                                                ClockMode::Auto => "AUTO",
                                            },
                                            now,
                                        );
                                        let _ = serial.write(
                                            clock_mode_data_text[i].get_text().as_str().as_bytes(),
                                        );
                                    }
                                }
                                "f" => {
                                    // is other a number?
                                    if let Ok(num) = &tokens[2].trim_end().parse::<u64>() {
                                        for i in 0..2 {
                                            prog.clock_set_freq(i, num);
                                        }
                                    } else {
                                        scroll_text.add_line("Err: no valid frequency found");
                                    }
                                }
                                _ => {
                                    scroll_text.add_line("Err: 'c' command needs s/s0/a/f");
                                    continue;
                                }
                            }
                        } else {
                            scroll_text.add_line("Err: c command only valid in Prog 0");
                        }
                    }

                    "c1" | "c2" => {
                        if prog.get_current_program() == 0 {
                            match tokens[1].as_str() {
                                "a" => {
                                    if num_tokens != 2 {
                                        scroll_text.add_line(
                                            "Err: 'c1/2 a' command does not accept parameters",
                                        );
                                        continue;
                                    }
                                }
                                "f" => {
                                    if num_tokens != 3 {
                                        scroll_text
                                            .add_line("Err: 'c1/2 f' command needs 1 parameter");
                                        continue;
                                    }
                                }
                                _ => {}
                            }

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
                                    } else {
                                        scroll_text.add_line("Err: no valid frequency found");
                                    }
                                }
                                _ => {
                                    scroll_text.add_line("Err: 'c1/2' command needs a/f");
                                    continue;
                                }
                            }
                        }
                    }
                    "p" => {
                        // p x
                        if num_tokens != 2 {
                            scroll_text.add_line("Err: 'p' command needs exactly 1 parameter");
                            continue;
                        }
                        if let Ok(num) = &tokens[1].trim_end().parse::<u8>() {
                            if *num > prog.number_of_programs() as u8 {
                                scroll_text.add_line("Err: no valid program number found");
                            } else {
                                prog.set_program(*num as usize);
                                prog.reset_state();
                                prog.mode = ProgramMode::Manual;
                                prog_mode_data_text.set(&"    ", now);
                                prog_number_data_text.set(&(*num as i16), now);
                                let _ = serial
                                    .write(prog_number_data_text.get_text().as_str().as_bytes());
                                prog_name_data_text.set(prog.get_current_program_name(), now);
                                let _ = serial
                                    .write(prog_name_data_text.get_text().as_str().as_bytes());
                                let _ = serial
                                    .write(prog_mode_data_text.get_text().as_str().as_bytes());
                            }
                        } else {
                            scroll_text.add_line("Err: no valid program number found");
                        }
                    }

                    // starts with channel numbers
                    _ => {
                        let mut active_channels = get_channels_from_text(&tokens[0]);
                        let mut active_channel_found = false;
                        for i in 0..OUTPUT_CHANNELS {
                            if active_channels[i] {
                                active_channel_found = true;
                                break;
                            }
                        }
                        if !active_channel_found {
                            scroll_text.add_line("Err: no valid channel number found");
                            continue;
                        }

                        for (i, channel) in active_channels.iter_mut().enumerate() {
                            if *channel {
                                if num_tokens == 1 {
                                    scroll_text.add_line("Err: 'channel' command needs parameters");
                                    continue;
                                }
                                match tokens[1].as_str() {
                                    // reverse bit order
                                    // channel r
                                    "r" => {
                                        if num_tokens != 2 {
                                            scroll_text.add_line(
                                                "Err: 'r' command does not accept parameters",
                                            );
                                            continue;
                                        }
                                        output_channels[i].reverse();
                                        let mut rev_str: String<PAGE_STR_WIDTH> = String::new();
                                        match output_channels[i].is_reversed() {
                                            true => {
                                                rev_str.push_str("R").unwrap();
                                            }
                                            false => {
                                                rev_str.push_str(" ").unwrap();
                                            }
                                        }
                                        output_channel_data_text[i].data_text[3].set(&rev_str, now);
                                        let _ = serial.write(
                                            output_channel_data_text[i].data_text[3]
                                                .get_text()
                                                .as_str()
                                                .as_bytes(),
                                        );
                                    }
                                    // channel value
                                    _ => {
                                        if num_tokens != 2 {
                                            scroll_text.add_line(
                                                "Err: 'channel' command needs exactly 1 parameter",
                                            );
                                            continue;
                                        }

                                        // is other a number?
                                        if let Ok(num) = &tokens[1].trim_end().parse::<i16>() {
                                            let mut log_str: String<PAGE_STR_WIDTH> =
                                                String::from_str("Channel ").unwrap();
                                            log_str.push(((i as u8 + 1) + 0x30) as char).unwrap();
                                            log_str.push_str(" set to ").unwrap();
                                            write!(log_str, "{}    ", num).unwrap();
                                            scroll_text.add_line(&log_str);

                                            output_channels[i].set(*num);
                                            for format in 0..3 {
                                                output_channel_data_text[i].data_text[format]
                                                    .set(num, now);
                                            }
                                        } else {
                                            scroll_text.add_line("no valid 16bit number found");
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
