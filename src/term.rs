extern crate libc;

const CSI: &str = "\x1b[";

pub fn window_size() -> (u16, u16) {
    let size = libc::winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    unsafe {
        libc::ioctl(0, libc::TIOCGWINSZ, &size);
    }

    (size.ws_row, size.ws_col)
}

pub fn cursor_previous_line(n: u8) {
    print!("{}{}F", CSI, n);
}
