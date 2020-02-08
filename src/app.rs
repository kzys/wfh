use ignore::gitignore;
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::time;
use std::time::Duration;

pub struct App {
    pub host: String,
    pub dirs: Vec<String>,
}

fn get_path(event: &DebouncedEvent) -> Option<&std::path::Path> {
    match event {
        DebouncedEvent::Create(path)
        | DebouncedEvent::Remove(path)
        | DebouncedEvent::Chmod(path)
        | DebouncedEvent::Write(path) => Some(path),
        _ => None,
    }
}

impl App {
    fn find_dir_to_sync(&self, event: &DebouncedEvent) -> Option<PathBuf> {
        let path = get_path(&event);
        if path.is_none() {
            return None;
        }

        let edited = path.unwrap();

        for dir in self.dirs.iter() {
            let dir = std::path::PathBuf::from(dir);

            let mut ib = gitignore::GitignoreBuilder::new(dir.clone());
            ib.add(dir.join(".gitignore"));

            let ignore = ib.build().unwrap();

            let m = ignore.matched_path_or_any_parents(&edited, false);
            if m.is_ignore() {
                continue;
            }

            if edited.starts_with(dir.canonicalize().unwrap()) {
                return Some(dir);
            }
        }

        None
    }

    pub fn run(&self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut dirs_to_sync: Vec<std::path::PathBuf> = vec![];

        let (tx, rx) = channel();
        let mut watcher = watcher(tx, time::Duration::from_secs(1)).expect("error");

        for parent in &self.dirs {
            for dir in fs::read_dir(&parent)? {
                dirs_to_sync.push(dir?.path())
            }
            watcher.watch(parent, RecursiveMode::Recursive)?;
        }

        let mut dirs_set = std::collections::HashSet::new();
        loop {
            match rx.recv_timeout(Duration::from_millis(1000)) {
                Ok(event) => {
                    self.find_dir_to_sync(&event).map(|x| dirs_set.insert(x));
                },
                Err(e) => {
                    if e == std::sync::mpsc::RecvTimeoutError::Timeout {
                        if !dirs_set.is_empty() {
                            println!("rsync: {:?}", dirs_set);
                            dirs_set.clear();
                        }
                    } else {
                        println!("watch error: {:?}", e)
                    }
                }
            }
        }
        Ok(())
    }
}
