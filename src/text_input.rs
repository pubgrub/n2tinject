pub mod text_input {
    use crate::format_str::format_str::PAGE_STR_WIDTH;
    use heapless::String;

    const VALID_CHARS: &str = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ -";

    pub enum TextInputState {
        Unchanged,
        Changed,
        Done,
    }

    pub struct TextInput {
        pub text: String<PAGE_STR_WIDTH>,
        state: TextInputState,
    }
    impl TextInput {
        pub fn new() -> Self {
            TextInput {
                text: String::new(),
                state: TextInputState::Unchanged,
            }
        }

        pub fn add_char(&mut self, c: u8) -> &TextInputState {
            if c == b'\n' || c == b'\r' {
                self.state = TextInputState::Done;
            } else if c == b'\x08' || c == b'\x7F' {
                self.remove_char();
                self.state = TextInputState::Changed;
            } else if is_valid_char(c as char) {
                self.text.push(c as char).unwrap();
                self.state = TextInputState::Changed;
            } else {
                // Handle invalid character case
            }
            &self.state
        }

        pub fn remove_char(&mut self) {
            if self.text.len() > 0 {
                self.text.pop().unwrap();
            }
        }

        pub fn clear(&mut self) -> String<PAGE_STR_WIDTH> {
            let len = self.text.len();
            self.text.clear();
            let mut clear_string: String<PAGE_STR_WIDTH> = String::new();
            for _ in 0..len {
                clear_string.push(' ').unwrap();
            }
            clear_string
        }

        pub fn get_text(&self) -> &String<PAGE_STR_WIDTH> {
            &self.text
        }
    }

    pub fn is_valid_char(c: char) -> bool {
        VALID_CHARS.contains(c)
    }
}
