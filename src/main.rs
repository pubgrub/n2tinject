#![no_std]
#![no_main]

mod button;
mod clock;
mod input_channel;
mod string;

use core::fmt::Write;
use embedded_hal::digital::{OutputPin, StatefulOutputPin as _};
use panic_halt as _;
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
use string::string::String;

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

fn tokenize(input: [u8; 64]) -> [String; 4] {
    let mut tokens: [String; 4] = [
        String::new([0; 64]),
        String::new([0; 64]),
        String::new([0; 64]),
        String::new([0; 64]),
    ];
    let mut token: [u8; 64] = [0u8; 64];
    let mut token_index = 0;
    let mut token_count = 0;
    for i in 0..64 {
        if input[i] == 0 {
            break;
        }
        if input[i] == 0x20 {
            if token_index == 0 {
                continue;
            }
            tokens[token_count].set(token);
            token = [0; 64];
            token_index = 0;
            token_count += 1;
        } else {
            token[token_index] = input[i];
            token_index += 1;
        }
    }
    tokens[token_count].set(token);
    tokens
}

fn get_channels_from_text(text: [u8; 64]) -> [bool; OUTPUT_CHANNELS] {
    let mut channels: [bool; OUTPUT_CHANNELS] = [false; OUTPUT_CHANNELS];
    for &t in text.iter() {
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

    // Initialisiere Systemtakt
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

    enum OutputChannelState {
        Idle,
        DataSet,
        EnableSet,
        Pause,
    }

    struct OutputChannel {
        data_pin: Pin<DynPinId, FunctionSioOutput, PullDown>, // data pin
        enable_pin: Pin<DynPinId, FunctionSioOutput, PullDown>, // enable pin
        state: OutputChannelState,                            // state of channel
        data: u16,                                            // number to output
        bit: u8,                                              // current bit
        next_tick: u64,                                       // next tick to output
        reverse: bool,                                        // reverse output bits
        last: bool, // double output last bit to trigger last shift
    }

    let mut output_channel_1 = OutputChannel {
        data_pin: pin_14,
        enable_pin: pin_15,
        state: OutputChannelState::Idle,
        data: 0,
        bit: 0,
        next_tick: 0,
        reverse: false,
        last: false,
    };
    let mut output_channel_2 = OutputChannel {
        data_pin: pin_16,
        enable_pin: pin_17,
        state: OutputChannelState::Idle,
        data: 0,
        bit: 0,
        next_tick: 0,
        reverse: false,
        last: false,
    };
    let mut output_channel_3 = OutputChannel {
        data_pin: pin_18,
        enable_pin: pin_19,
        state: OutputChannelState::Idle,
        data: 0,
        bit: 0,
        next_tick: 0,
        reverse: false,
        last: false,
    };
    let mut output_channel_4 = OutputChannel {
        data_pin: pin_20,
        enable_pin: pin_21,
        state: OutputChannelState::Idle,
        data: 0,
        bit: 0,
        next_tick: 0,
        reverse: false,
        last: false,
    };

    let mut output_channels: [&mut OutputChannel; OUTPUT_CHANNELS] = [
        &mut output_channel_1,
        &mut output_channel_2,
        &mut output_channel_3,
        &mut output_channel_4,
    ];

    let blink_interval = 1_000_000u64;
    let mut blink_last = 0u64;

    let mut data_output_data_slot = 10_000u64; // time from data out to enable on and from enable off to end of cycle
    let mut data_output_enable_slot = 10_000u64; // time from enable on to enable off

    let mut now;

    let answer_str = "Answer:".as_bytes();
    let empty_command = [0u8; 64];
    let mut all_channels_str = empty_command;
    for i in 0..OUTPUT_CHANNELS {
        all_channels_str[i] = (i as u8 + 1) + 0x30;
    }

    let button1 = Button::new(pin_12);
    let button2 = Button::new(pin_13);

    let mut clock1 = Clock::new(pin_10, button1);
    let mut clock2 = Clock::new(pin_11, button2);

    let mut clocks = [&mut clock1, &mut clock2];

    let mut input_channel = InputChannel::new(pin_6, pin_8, pin_9, pin_7);

    //test data
    output_channels[0].data = 0b1111011101101;
    output_channels[0].state = OutputChannelState::Pause;
    output_channels[0].next_tick = 0;
    output_channels[0].bit = 0;
    output_channels[0].reverse = true;

    loop {
        now = timer.get_counter().ticks();

        // handle blinking LED
        if now > blink_last + blink_interval {
            blink_pin.toggle().unwrap();
            blink_last = now;
        }

        // handle output channels
        for channel in output_channels.iter_mut() {
            match channel.state {
                OutputChannelState::Idle => {
                    // do nothing
                }
                OutputChannelState::Pause => {
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
                        channel.next_tick = now + data_output_data_slot;
                        channel.state = OutputChannelState::DataSet;
                    }
                }
                OutputChannelState::DataSet => {
                    if now > channel.next_tick {
                        channel.enable_pin.set_high().unwrap();
                        channel.next_tick = now + data_output_enable_slot;
                        channel.state = OutputChannelState::EnableSet;
                    }
                }
                OutputChannelState::EnableSet => {
                    if now > channel.next_tick {
                        channel.enable_pin.set_low().unwrap();
                        channel.next_tick = now + data_output_enable_slot;
                        if channel.bit < 16 {
                            channel.state = OutputChannelState::Pause;
                        } else {
                            if channel.last == false {
                                channel.last = true;
                                channel.bit = 15;
                                channel.next_tick = now + data_output_data_slot;
                                channel.state = OutputChannelState::Pause;
                            } else {
                                channel.state = OutputChannelState::Idle;
                                channel.last = false;
                            }
                        }
                    }
                }
            }
        }

        // handle clocks
        for clock in clocks.iter_mut() {
            clock.update(now);
        }

        // handle USB communication
        if !usb_dev.poll(&mut [&mut serial]) {
            //            debug!("waiting...");
            continue;
        }

        // handle input channel
        input_channel.update(now);
        let _ = serial.write(&u16_to_str(input_channel.data).buf);
        // match input_channel.state {
        //     input_channel::input_channel::InputChannelState::Idle => {
        //         let _ = serial.write("Idle ".as_bytes());
        //     }
        //     input_channel::input_channel::InputChannelState::LoadingData => {
        //         let _ = serial.write("Loading Data ".as_bytes());
        //     }
        //     input_channel::input_channel::InputChannelState::SerialShiftOff => {
        //         let _ = serial.write("SerialShiftOff ".as_bytes());
        //     }
        //     input_channel::input_channel::InputChannelState::ShiftClockOn => {
        //         let _ = serial.write("SerialShift On ".as_bytes());
        //     }
        // }
        let _ = serial.write(&u16_to_str(input_channel.data).buf);
        if input_channel.data_changed {
            let data = input_channel.data;
            let data_str = u16_to_str(data);
            let _ = serial.write(&data_str.buf);
            input_channel.data_changed = false;
        }

        let mut buf = [b' '; 64];

        if let Ok(count) = serial.read(&mut buf) {
            // Echo zurücksenden

            let mut tokens = tokenize(buf); // split input into tokens
            let mut changed = true;
            while changed {
                changed = false;

                match core::str::from_utf8(&tokens[0].get()[..tokens[0].get_size()]).unwrap() {
                    "z" | "0" | "r" => {
                        tokens[1].set(tokens[0].get());
                        tokens[0].set(all_channels_str);
                        changed = true;
                    }
                    "+" => {
                        data_output_data_slot = 100.max(data_output_data_slot / 10);
                        data_output_enable_slot = 100.max(data_output_enable_slot / 10);
                    }
                    "-" => {
                        data_output_data_slot = 1_000_000.min(data_output_data_slot * 10);
                        data_output_enable_slot = 1_000_000.min(data_output_enable_slot * 10);
                    }
                    "c" => {
                        match core::str::from_utf8(&tokens[1].get()[0..tokens[1].get_size()])
                            .unwrap()
                        {
                            "sync" => {
                                match core::str::from_utf8(
                                    &tokens[2].get()[0..tokens[2].get_size()],
                                )
                                .unwrap()
                                {
                                    "opp" => {
                                        let (clock1, clock2) = clocks.split_at_mut(1);
                                        clock1[0].sync_opposite(&mut clock2[0]);
                                    }
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
                        let clock_index = match tokens[0].get()[1] {
                            b'1' => 0,
                            b'2' => 1,
                            _ => 0,
                        };
                        match core::str::from_utf8(&tokens[1].get()[0..tokens[1].get_size()])
                            .unwrap()
                        {
                            "auto" => {
                                clocks[clock_index].auto = true;
                                clocks[clock_index].next_tick = now;
                                clocks[clock_index].state = false;
                            }

                            "on" => {
                                clocks[clock_index].auto = false;
                                clocks[clock_index].state = true;
                            }
                            "off" => {
                                clocks[clock_index].auto = false;
                                clocks[clock_index].state = false;
                            }
                            "f" => {
                                // is other a number?
                                if let Ok(num) =
                                    core::str::from_utf8(&tokens[2].get()[0..tokens[2].get_size()])
                                        .unwrap()
                                        .trim_end()
                                        .parse::<u64>()
                                {
                                    clocks[clock_index].set_freq(num);
                                }
                            }
                            _ => {}
                        }
                    }

                    _ => {
                        let mut active_channels = get_channels_from_text(tokens[0].get());
                        for (i, channel) in active_channels.iter_mut().enumerate() {
                            if *channel {
                                match tokens[1].get()[0] {
                                    b'0' | b'z' => {
                                        output_channels[i].state = OutputChannelState::Pause;
                                        output_channels[i].next_tick = 0;
                                        output_channels[i].bit = 0;
                                        output_channels[i].data = 0;
                                    }
                                    b'r' => {
                                        output_channels[i].state = OutputChannelState::Pause;
                                        output_channels[i].next_tick = 0;
                                        output_channels[i].bit = 0;
                                        output_channels[i].reverse = !output_channels[i].reverse;
                                    }
                                    _ => {
                                        // is other a number?
                                        if let Ok(num) = core::str::from_utf8(
                                            &tokens[1].get()[0..tokens[1].get_size()],
                                        )
                                        .unwrap()
                                        .trim_end()
                                        .parse::<u16>()
                                        {
                                            let _ = serial.write("got a number".as_bytes());

                                            output_channels[i].data = num;
                                            output_channels[i].state = OutputChannelState::Pause;
                                            output_channels[i].next_tick = 0;
                                            output_channels[i].bit = 0;
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
