use std::fs;
use std::io;
use std::thread;
use std::time;

mod pbar;

fn main() -> io::Result<()> {
    let entries = fs::read_dir(".")?
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, io::Error>>()?;

    let (_, col) = pbar::window_size();
    println!("col = {}", col);

    let mut x = false;
    loop {
        pbar::print_pbar(&entries, if x { "--" } else { "  " });

        let secs = time::Duration::from_secs(1);
        thread::sleep(secs);

        x = !x;

        pbar::reset_pbar(entries.len());
    }
}
