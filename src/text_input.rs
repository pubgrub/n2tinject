pub mod text_input {
    use crate::format_str::format_str::PAGE_STR_WIDTH;
    use core::str::FromStr;
    use heapless::String;

    const VALID_CHARS: &str = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ ";

    pub struct TextInputState {
        pub text: String<PAGE_STR_WIDTH>,
        pub is_changed: bool,
        pub is_done: bool,
    }
    impl TextInputState {
        pub fn new() -> Self {
            TextInputState {
                text: String::new(),
                is_changed: false,
                is_done: false,
            }
        }
    }
    pub struct TextInput {
        pub text: String<PAGE_STR_WIDTH>,
        pub max_len: usize,
    }
    impl TextInput {
        pub fn new() -> Self {
            TextInput {
                text: String::new(),
                max_len: PAGE_STR_WIDTH,
            }
        }

        pub fn add_char(&mut self, c: char) -> TextInputState {
            let mut state = TextInputState::new();
            if c == '\n' {
                state.is_done = true;
                state.text = self.text.clone();
                self.text.clear();
                return state;
            }
            if c == '\x08' {
                self.remove_char();
                state.text = self.text.clone();
                state.is_changed = true;
                return state;
            }
            if is_valid_char(c) {
                if self.text.len() < self.max_len {
                    self.text.push(c).unwrap();
                    state.text = self.text.clone();
                    state.is_changed = true;
                    return state;
                } else {
                    // Handle max length exceeded case
                    return state;
                }
            } else {
                // Handle invalid character case
                return state;
            }
        }
        pub fn remove_char(&mut self) {
            if self.text.len() > 0 {
                self.text.pop().unwrap();
            }
        }

        fn get_text(&self) -> &String<PAGE_STR_WIDTH> {
            &self.text
        }
    }
    pub fn is_valid_char(c: char) -> bool {
        VALID_CHARS.contains(c)
    }
}
