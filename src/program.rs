pub mod program {
    use crate::format_str;
    use heapless::String;

    const PAGE_STR_WIDTH: usize = format_str::format_str::PAGE_STR_WIDTH;
    pub struct Program {
        name: String<PAGE_STR_WIDTH>,
        sequence_length: u8,
        sequence: [usize; 2],
        status: u8,
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
                status: 0,
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
        pub fn get_status(&self) -> u8 {
            self.status
        }

        pub fn start(&mut self) {
            self.status = 0; // Running
        }

        pub fn advance(&mut self) -> bool {
            self.status += 1;
            if self.status >= self.sequence_length {
                self.status = 0; // Loop back to the start
                return false;
            }
            true
        }
        pub fn get_signals(&self) -> [bool; 2] {
            [
                self.sequence[0] >> (self.sequence_length - self.status - 1) & 1 != 0,
                self.sequence[1] >> (self.sequence_length - self.status - 1) & 1 != 0,
            ]
        }
    }

    struct ProgramList {
        programs: [Program; 2],
        current_program: usize,
    }
}
