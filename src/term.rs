extern crate libc;

const CSI: &str = "\x1b[";

pub fn cursor_previous_line(n: u8) {
    print!("{}{}F", CSI, n);
}
