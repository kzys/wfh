extern crate libc;

use std::path;

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
        println!("\x1b[2F");
    }
}

pub fn print_pbar(dirs: &Vec<path::PathBuf>, s: &str) {
    for dir in dirs {
        println!("\x1b[2K[{}] {:?}", s, dir);
    }
}
