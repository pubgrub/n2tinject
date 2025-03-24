pub mod string {
    use core::u8;

    pub struct String {
        buf: [u8; 64],
        size: usize,
    }
    impl String {
        pub fn new(data: [u8; 64]) -> Self {
            String {
                buf: data,
                size: String::calc_size(data),
            }
        }

        pub fn get(&self) -> [u8; 64] {
            self.buf
        }
        pub fn get_size(&self) -> usize {
            self.size
        }

        pub fn set(&mut self, data: [u8; 64]) -> () {
            self.buf = data;
            self.size = String::calc_size(data);
        }

        fn calc_size(data: [u8; 64]) -> usize {
            let mut size = 0;
            for i in 0..64 {
                if data[i] == 0 {
                    break;
                }
                size += 1;
            }
            size
        }
    }
}
