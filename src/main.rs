extern crate ignore;
extern crate notify;

#[macro_use]
extern crate structopt;

use ignore::gitignore;
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use std::fs;
use std::io;
use std::sync::mpsc::channel;
use std::thread;
use std::time;

mod pbar;

use std::path::PathBuf;
use structopt::StructOpt;

/// A basic example
#[derive(StructOpt, Debug)]
#[structopt(name = "wfh", about = "synchronize files as you edit")]
struct Options {
    host: String,
    dirs: Vec<String>,
}

fn get_path(event: &DebouncedEvent) -> Option<&std::path::Path> {
    match event {
        DebouncedEvent::Create(path)
        | DebouncedEvent::Remove(path)
        | DebouncedEvent::Chmod(path)
        | DebouncedEvent::Write(path) => Some(path),
        _ => None, //
    }
} //

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let options = Options::from_args();
    println!("{:#?}", options);

    let mut dirs_to_sync: Vec<std::path::PathBuf> = vec![];

    let (_, col) = pbar::window_size();
    println!("col = {}", col);

    let (tx, rx) = channel();
    let mut watcher = watcher(tx, time::Duration::from_secs(1)).expect("error");

    for parent in options.dirs {
        for dir in fs::read_dir(&parent)? {
            dirs_to_sync.push(dir?.path())
        }
        watcher.watch(parent, RecursiveMode::Recursive)?;
    }
    loop {
        match rx.recv() {
            Ok(event) => {
                let path = get_path(&event);
                if path.is_none() {
                    continue;
                }

                let edited = path.unwrap();

                for dir in dirs_to_sync.iter() {
                    let mut ib = gitignore::GitignoreBuilder::new(dir.canonicalize().unwrap());
                    ib.add(dir.join(".gitignore"));

                    let ignore = ib.build().unwrap();

                    let m = ignore.matched_path_or_any_parents(&edited, false);
                    if m.is_ignore() {
                        println!("ignore edited file {:?}", edited);
                        continue;
                    }

                    if edited.starts_with(dir.canonicalize().unwrap()) {
                        println!("rsync {:?} due to {:?}", dir, event);
                    }
                }
            }
            Err(e) => println!("watch error: {:?}", e),
        }
    }

    Ok(())
}
