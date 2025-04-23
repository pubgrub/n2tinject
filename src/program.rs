pub mod program {

    use crate::button::button::{Button, ButtonState};
    use crate::clock::clock::Clock;
    use crate::format_str;
    use core::str::FromStr;
    use heapless::{String, Vec};

    pub const MAX_PROGRAMS: usize = 20; // Maximum number of programs
    const PAGE_STR_WIDTH: usize = format_str::format_str::PAGE_STR_WIDTH;
    pub struct Program {
        name: String<PAGE_STR_WIDTH>,
        sequence_length: u8,
        sequence: [usize; 2],
    }
    impl Program {
        pub fn new(
            name: String<PAGE_STR_WIDTH>,
            sequence_length: u8,
            sequence: [usize; 2],
        ) -> Self {
            Program {
                name,
                sequence_length,
                sequence,
            }
        }
        pub fn get_name(&self) -> &String<PAGE_STR_WIDTH> {
            &self.name
        }
        pub fn get_sequence_length(&self) -> u8 {
            self.sequence_length
        }
        pub fn get_sequence(&self) -> &[usize; 2] {
            &self.sequence
        }

        pub fn get_signals(&self, state: u8) -> [bool; 2] {
            [
                self.sequence[0] >> (self.sequence_length - state - 1) & 1 != 0,
                self.sequence[1] >> (self.sequence_length - state - 1) & 1 != 0,
            ]
        }
    }

    #[derive(PartialEq)]
    pub enum ProgramMode {
        Manual,
        OneShot,
        Auto,
    }

    pub struct ProgramControl {
        program_list: Vec<Program, MAX_PROGRAMS>,
        pub clocks: [Clock; 2],
        buttons: [Button; 2],
        current_program: usize,
        pub mode: ProgramMode,
        state: u8,
        tick_freq: u32,
        ticks_per_step: u32,
        next_tick: u64,
    }

    impl ProgramControl {
        pub fn new(tick_freq: u32, clocks: [Clock; 2], buttons: [Button; 2]) -> Self {
            let mut p_control = ProgramControl {
                tick_freq,
                clocks,
                buttons,
                program_list: Vec::new(),
                current_program: 0,
                state: 0,
                mode: ProgramMode::Manual,
                ticks_per_step: tick_freq, // 1s tick length as default
                next_tick: 0,
            };
            p_control.add_program(Program::new(
                String::from_str("Manual").unwrap(),
                0,
                [0b0, 0b0],
            ));
            p_control
        }

        pub fn add_program(&mut self, program: Program) {
            if self.program_list.len() < MAX_PROGRAMS {
                self.program_list.push(program);
            } else {
                panic!("Program list is full");
            }
        }
        pub fn number_of_programs(&self) -> usize {
            self.program_list.len()
        }

        pub fn update(&mut self, now: u64) {
            // Update buttons
            let button_states = [self.buttons[0].update(now), self.buttons[1].update(now)];

            // Update clocks
            match self.current_program {
                0 => {
                    for (i, c) in self.clocks.iter_mut().enumerate() {
                        c.update(now, &button_states[i]);
                    }
                }
                _ => match self.mode {
                    ProgramMode::Manual => {
                        if button_states[0].state_changed && button_states[0].state {
                            self.mode = ProgramMode::OneShot; // Switch to oneshot mode
                        } else if button_states[1].state_changed && button_states[1].state {
                            self.state += 1;
                            if self.state
                                >= self.program_list[self.current_program].get_sequence_length()
                            {
                                self.state = 0;
                            }
                            let signals = self.get_signals();
                            for (i, c) in self.clocks.iter_mut().enumerate() {
                                c.set_pin(signals[i]);
                            }
                        }
                    }
                    ProgramMode::Auto | ProgramMode::OneShot => {
                        if now > self.next_tick {
                            self.next_tick = now + self.ticks_per_step as u64;
                            self.state += 1;
                            if self.state
                                >= self.program_list[self.current_program].get_sequence_length()
                            {
                                self.state = 0;
                                if self.mode == ProgramMode::OneShot {
                                    self.mode = ProgramMode::Manual; // Reset to manual mode
                                }
                            }
                            let signals = self.get_signals();
                            for (i, c) in self.clocks.iter_mut().enumerate() {
                                c.set_pin(signals[i]);
                            }
                        }
                    }
                },
            }
        }

        fn get_signals(&self) -> [bool; 2] {
            if self.current_program > 0 {
                self.program_list[self.current_program].get_signals(self.state)
            } else {
                [false, false]
            }
        }

        pub fn set_program(&mut self, program: usize) -> bool {
            if program < self.program_list.len() {
                self.current_program = program;
                self.state = 0;
                self.next_tick = 0;
                true
            } else {
                false
            }
        }

        pub fn get_current_program(&mut self) -> usize {
            self.current_program
        }

        pub fn clocks_sync(&mut self) {
            let (clock0, clock1) = self.clocks.split_at_mut(1);
            clock0[0].sync(&mut clock1[0]);
        }
        pub fn clocks_sync_opposite(&mut self) {
            let (clock0, clock1) = self.clocks.split_at_mut(1);
            clock0[0].sync_opposite(&mut clock1[0]);
        }
        pub fn clock_toggle_auto(&mut self, clock: usize) {
            if clock < self.clocks.len() {
                self.clocks[clock].auto = !self.clocks[clock].auto;
            }
        }
        pub fn clock_set_freq(&mut self, clock: usize, interval: &u64) {
            if clock < self.clocks.len() {
                self.clocks[clock].set_freq(interval);
            }
        }
        pub fn reset_state(&mut self) {
            self.state = 0;
            self.next_tick = 0;
        }

        pub fn set_freq(&mut self, freq: u32) {
            self.ticks_per_step = self.tick_freq / freq * 10;
        }
    }
}
