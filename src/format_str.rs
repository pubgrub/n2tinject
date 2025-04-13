pub mod format_str {
    use core::array::from_fn;
    use core::fmt::Write;
    use dyn_fmt::dyn_write;
    use heapless::String;

    const SCROLL_LINES: usize = 5;
    pub const PAGE_LINES: usize = 15;
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
        pub fn add_line(&mut self, new_line: String<PAGE_STR_WIDTH>) {
            for i in (0..SCROLL_LINES - 1).rev() {
                self.lines[i + 1] = core::mem::take(&mut self.lines[i]);
                self.print_lines[i + 1] =
                    add_position(&self.lines[i + 1], self.x, self.first_y + (i as u8 + 1));
            }
            self.lines[0] = new_line;
            self.print_lines[0] = bold(&add_position(&self.lines[0], self.x, self.first_y));
            self.is_changed = true;
        }
        pub fn get_lines(&mut self) -> &[String<PAGE_STR_WIDTH>; SCROLL_LINES] {
            self.is_changed = false;
            &self.print_lines
        }
    }

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
                format_str: format_str,
                text: String::new(),
                print_text: String::new(),
                x: x,
                y: y,
                mark_as_new: mark_as_new,
                is_new: false,
                new_until: 0,
                is_changed: false,
            }
        }
        pub fn set<T: core::fmt::Display>(&mut self, val: &T, now: u64) {
            self.text.clear();
            dyn_write!(self.text, self.format_str.as_str(), &[val]).unwrap();
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
    }

    #[derive(Debug)]
    pub struct StaticPageText {
        lines: [String<PAGE_STR_WIDTH>; PAGE_LINES],
        print_lines: [String<PAGE_STR_WIDTH>; PAGE_LINES],
        x: u8,
        first_y: u8,
    }

    impl StaticPageText {
        pub fn new(lines_str: [&str; PAGE_LINES], x: u8, first_y: u8) -> StaticPageText {
            let mut s: StaticPageText = StaticPageText {
                lines: from_fn(|_| String::new()),
                print_lines: from_fn(|_| String::new()),
                x,
                first_y,
            };
            for l in 0..PAGE_LINES {
                //                println!("<--1 {}: {}: {} -->",l,lines_str[l],&s.lines[l].as_str());
                s.lines[l].push_str(lines_str[l].as_ref()).unwrap();
                //                println!("<--2 {}: {}: {} -->", l, lines_str[l], s.lines[l].as_str());
                s.print_lines[l] = add_position(&s.lines[l], s.x, s.first_y + (l as u8));
                //                println!("<--3 {}: {}: {} -->",l,lines_str[l],&s.print_lines[l].as_str());
            }
            s
        }
        pub fn get_lines(&mut self) -> &[String<PAGE_STR_WIDTH>; PAGE_LINES] {
            &self.print_lines
        }
        pub fn edit_line(&mut self, line: usize, start: u8, str: &str) {
            let line_index = line - 1;
            if line_index < PAGE_LINES {
                let result: String<PAGE_STR_WIDTH> = edit_str(&self.lines[line_index], start, str);
                self.lines[line_index] = result;
                self.print_lines[line_index] = add_position(
                    &self.lines[line_index],
                    self.x,
                    self.first_y + (line_index as u8),
                );
            }
        }
    }

    static INVERTED_ON: &str = "\x1B[7m";
    static INVERTED_OFF: &str = "\x1B[27m";

    static BOLD_ON: &str = "\x1B[1m";
    static BOLD_OFF: &str = "\x1B[0m";

    pub fn edit_str(
        str: &String<PAGE_STR_WIDTH>,
        start: u8,
        str_to_add: &str,
    ) -> String<PAGE_STR_WIDTH> {
        let end = start + str_to_add.len() as u8 - 1;
        if end > PAGE_WIDTH as u8 {
            panic!("Line is too long");
        }
        if start > PAGE_WIDTH as u8 {
            panic!("Line is too short");
        }
        let pre_str = &str.clone()[0..start as usize];
        let post_str = &str.clone()[end as usize + 1..];
        let mut result: String<PAGE_STR_WIDTH> = String::new();
        result.push_str(pre_str).unwrap();
        result.push_str(str_to_add).unwrap();
        result.push_str(post_str).unwrap();
        result
    }

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
        let mut result = String::new();
        write!(result, "\x1B[{};{}H", y, x).unwrap();
        //        println!("before: {}", result);
        result.push_str(str).unwrap();
        //        println!("after: {}", result);
        result
    }

    fn str_len(str: &[u8; PAGE_STR_WIDTH]) -> u8 {
        let mut str_len = 0;
        for s in str.iter() {
            if *s == 0 {
                break;
            } else {
                str_len += 1;
            }
        }
        str_len
    }
}
