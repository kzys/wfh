extern crate libc;

use std::fmt::Display;

const CSI: &str = "\x1b[2";

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

pub fn reset_pbar(n: usize) {
    for _ in 0..n {
        println!("{}F", CSI);
    }
}

pub fn print_pbar<I, X>(xs: I)
where
    I: IntoIterator<Item = X>,
    X: Display,
{
    for x in xs {
        println!("{}K{}", CSI, x);
    }
}
