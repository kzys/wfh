use ignore::gitignore::GitignoreBuilder;
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::time;
use std::time::Duration;

pub struct App {
    host: String,
    dirs: Vec<PathBuf>,
}

fn find_path(event: &DebouncedEvent) -> Option<&Path> {
    match event {
        DebouncedEvent::Create(path)
        | DebouncedEvent::Remove(path)
        | DebouncedEvent::Chmod(path)
        | DebouncedEvent::Write(path) => Some(path),
        _ => None,
    }
}

impl App {
    pub fn new(host: String, dirs: Vec<String>) -> Result<App, Box<dyn std::error::Error>> {
        let mut dirs_to_sync: Vec<std::path::PathBuf> = vec![];

        for parent in dirs {
            for dir in fs::read_dir(&parent)? {
                dirs_to_sync.push(dir?.path())
            }
        }

        Ok(App {
            host,
            dirs: dirs_to_sync,
        })
    }
    fn find_dir_to_sync(&self, event: &DebouncedEvent) -> Option<PathBuf> {
        find_path(&event).and_then(|edited| {
            let dir_to_sync = self
                .dirs
                .iter()
                .map(PathBuf::from)
                .find(|dir| edited.starts_with(dir.canonicalize().unwrap()));

            dir_to_sync.filter(|dir| {
                // TODO: handle multiple .gitignore files
                let ignore_path = dir.join(".gitignore");
                if !ignore_path.exists() {
                    return true;
                }

                let mut ib = GitignoreBuilder::new(dir.clone());
                ib.add(ignore_path);

                let ignore = ib.build().unwrap();
                let m = ignore.matched_path_or_any_parents(&edited, false);
                !m.is_ignore()
            })
        })
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

        let mut dirs_set = HashSet::new();
        loop {
            match rx.recv_timeout(Duration::from_millis(1000)) {
                Ok(event) => {
                    self.find_dir_to_sync(&event).map(|x| dirs_set.insert(x));
                }
                Err(e) => {
                    if e == std::sync::mpsc::RecvTimeoutError::Timeout {
                        if !dirs_set.is_empty() {
                            self.sync_dirs(&dirs_set);
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

    fn sync_dirs(&self, dirs: &HashSet<PathBuf>) {
        println!("rsync: {:?} {:?}", self.host, dirs);
    }
}
