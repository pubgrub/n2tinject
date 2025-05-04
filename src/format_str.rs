pub mod format_str {
    use core::fmt::Write;
    use core::{array::from_fn, str::FromStr};
    use heapless::String;

    const SCROLL_LINES: usize = 5;
    pub const PAGE_LINES: usize = 18;
    pub const PAGE_WIDTH: usize = 80;
    pub const PAGE_STR_WIDTH: usize = PAGE_WIDTH + 20;
    const NEW_INTERVAL: u64 = 5_000;
    pub struct ScrollText {
        lines: [String<PAGE_STR_WIDTH>; SCROLL_LINES],
        print_lines: [String<PAGE_STR_WIDTH>; SCROLL_LINES],
        pub is_changed: bool,
        x: u8,
        first_y: u8,
    }

    impl ScrollText {
        pub fn new(x: u8, first_y: u8) -> ScrollText {
            let mut s: ScrollText = ScrollText {
                lines: from_fn(|_| String::new()),
                print_lines: from_fn(|_| String::new()),
                is_changed: true,
                x,
                first_y,
            };
            for y in 0..SCROLL_LINES {
                s.print_lines[y] = add_position(&s.lines[y], x, first_y + (y as u8));
            }
            s
        }
        pub fn add_line(&mut self, new_line: &str) {
            for i in (0..SCROLL_LINES - 1).rev() {
                self.lines[i + 1] = core::mem::take(&mut self.lines[i]);
                self.print_lines[i + 1] =
                    add_position(&self.lines[i + 1], self.x, self.first_y + (i as u8 + 1));
            }
            self.lines[0] = String::from_str(new_line).unwrap();
            self.print_lines[0] = bold(&add_position(&self.lines[0], self.x, self.first_y));
            self.is_changed = true;
        }
        pub fn get_lines(&mut self) -> &[String<PAGE_STR_WIDTH>; SCROLL_LINES] {
            self.is_changed = false;
            &self.print_lines
        }
    }

    #[derive(Debug)]
    pub struct DataText {
        format_str: String<PAGE_STR_WIDTH>,
        text: String<PAGE_STR_WIDTH>,
        print_text: String<PAGE_STR_WIDTH>,
        x: u8,
        y: u8,
        mark_as_new: bool,
        is_new: bool,
        new_until: u64,
        pub is_changed: bool,
    }

    impl DataText {
        pub fn new(
            format_str: String<PAGE_STR_WIDTH>,
            x: u8,
            y: u8,
            mark_as_new: bool,
        ) -> DataText {
            DataText {
                format_str,
                text: String::new(),
                print_text: String::new(),
                x,
                y,
                mark_as_new,
                is_new: false,
                new_until: 0,
                is_changed: false,
            }
        }
        pub fn set<T: core::fmt::Display + DynamicFormatArg>(&mut self, val: &T, now: u64) {
            self.text.clear();
            let args: &[&dyn DynamicFormatArg] = &[val];
            for arg in args.iter() {
                arg.format(&mut self.text, self.format_str.as_str())
                    .unwrap();
            }
            self.print_text = self.text.clone();
            if self.mark_as_new {
                self.print_text = invert(&self.text);

                self.is_new = true;
                self.new_until = now + NEW_INTERVAL;
            }
            self.print_text = add_position(&self.print_text, self.x, self.y);
            self.is_changed = true;
        }

        pub fn update(&mut self, now: u64) {
            if self.mark_as_new && self.is_new && now > self.new_until {
                self.is_new = false;
                self.print_text = add_position(&self.text, self.x, self.y);
                self.is_changed = true;
            }
        }

        pub fn get_text(&mut self) -> &String<PAGE_STR_WIDTH> {
            self.is_changed = false;
            &self.print_text
        }

        pub fn get_cursor(&self) -> (u8, u8) {
            (self.x + self.text.len() as u8, self.y)
        }
    }

    #[derive(Debug)]
    pub struct StaticPageText {
        lines: [String<PAGE_STR_WIDTH>; PAGE_LINES],
        print_lines: [String<PAGE_STR_WIDTH>; PAGE_LINES],
        x: u8,
        first_y: u8,
    }

    impl StaticPageText {
        pub fn new(
            lines: [String<PAGE_STR_WIDTH>; PAGE_LINES],
            x: u8,
            first_y: u8,
        ) -> StaticPageText {
            let mut s: StaticPageText = StaticPageText {
                lines,
                print_lines: from_fn(|_| String::new()),
                x,
                first_y,
            };
            for l in 0..PAGE_LINES {
                s.print_lines[l] = add_position(&s.lines[l], s.x, s.first_y + (l as u8));
            }
            s
        }
        pub fn get_lines(&mut self) -> &[String<PAGE_STR_WIDTH>; PAGE_LINES] {
            &self.print_lines
        }
    }

    static INVERTED_ON: &str = "\x1B[7m";
    static INVERTED_OFF: &str = "\x1B[27m";

    static BOLD_ON: &str = "\x1B[1m";
    static BOLD_OFF: &str = "\x1B[0m";

    /// This function adds ANSI escape codes to invert the text color.
    ///
    /// # Arguments
    ///
    /// * `str` - The string to which the inverted effect will be added.
    ///
    /// # Returns
    ///
    /// A new string with the ANSI escape codes prepended and appended.
    pub fn invert(str: &String<PAGE_STR_WIDTH>) -> String<PAGE_STR_WIDTH> {
        let mut result = String::new();
        result.push_str(INVERTED_ON).unwrap();
        result.push_str(&str).unwrap();
        result.push_str(INVERTED_OFF).unwrap();
        result
    }

    /// pub fn bold(str: &String<PAGE_STR_WIDTH>) -> String<PAGE_STR_WIDTH>
    /// /// This function adds ANSI escape codes to make the text bold.
    ///
    /// # Arguments
    ///
    /// * `str` - The string to which the bold effect will be added.
    ///
    /// # Returns
    ///
    /// A new string with the ANSI escape codes prepended and appended.
    pub fn bold(str: &String<PAGE_STR_WIDTH>) -> String<PAGE_STR_WIDTH> {
        let mut result = String::new();
        result.push_str(BOLD_ON).unwrap();
        result.push_str(&str).unwrap();
        result.push_str(BOLD_OFF).unwrap();
        result
    }

    /// This function adds ANSI escape codes to position the cursor.
    ///
    /// # Arguments
    ///
    /// * `str` - The string to which the position will be added.
    /// * `x` - The horizontal position (column).
    /// * `y` - The vertical position (row).
    ///
    /// # Returns
    ///
    /// A new string with the ANSI escape codes prepended.
    fn add_position(str: &String<PAGE_STR_WIDTH>, x: u8, y: u8) -> String<PAGE_STR_WIDTH> {
        let mut result: String<PAGE_STR_WIDTH> = String::new();
        write!(result, "\x1B[{};{}H", y, x).unwrap();
        result.push_str(str).unwrap();
        result
    }

    pub trait DynamicFormatArg {
        fn format(&self, f: &mut dyn core::fmt::Write, fmt: &str) -> core::fmt::Result;
    }
    impl DynamicFormatArg for i16 {
        fn format(&self, f: &mut dyn core::fmt::Write, fmt: &str) -> core::fmt::Result {
            match fmt {
                "{:>4}" => write!(f, "{:>4}", self),
                "{:>6}" => write!(f, "{:>6}", self),
                "{:#06X}" => write!(f, "{:#06X}", self),
                "{:#018b}" => write!(f, "{:#018b}", self),
                _ => write!(f, "{}", self),
            }
        }
    }
    impl DynamicFormatArg for String<PAGE_STR_WIDTH> {
        fn format(&self, f: &mut dyn core::fmt::Write, fmt: &str) -> core::fmt::Result {
            match fmt {
                _ => write!(f, "{}", self),
            }
        }
    }
    impl DynamicFormatArg for &str {
        fn format(&self, f: &mut dyn core::fmt::Write, fmt: &str) -> core::fmt::Result {
            match fmt {
                _ => write!(f, "{}", self),
            }
        }
    }
}
