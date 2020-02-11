use ignore::gitignore::GitignoreBuilder;
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader, Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
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
                    debug!("sync {:?}", edited);
                    return true;
                }

                let mut ib = GitignoreBuilder::new(dir.clone());
                ib.add(ignore_path.clone());
                let ignore = ib.build().unwrap();

                let m = ignore.matched_path_or_any_parents(&edited, false);
                if m.is_ignore() {
                    trace!("ignore {:?} due to {:?}", edited, ignore_path);
                    false
                } else {
                    debug!("sync {:?}", edited);
                    true
                }
            })
        })
    }

    pub fn run(&self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let (tx, rx) = channel();
        let mut watcher = watcher(tx, time::Duration::from_secs(1)).expect("error");

        for parent in &self.dirs {
            debug!("watch {:?}", parent);
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
                        error!("watch error: {:?}", e)
                    }
                }
            }
        }
        Ok(())
    }

    fn sync_dirs(&self, dirs: &HashSet<PathBuf>) {
        for dir in dirs {
            self.sync_dir(dir)
        }
    }

    fn sync_dir(&self, dir: &PathBuf) {
        let src = format!("{}/", dir.to_string_lossy());
        let dest = format!("{}:{}/", self.host, dir.to_string_lossy());

        let mut args = vec!["--archive", "--verbose"];
        args.push(&src);
        args.push(&dest);

        self.run_command("rsync", args)
    }

    fn run_command(&self, command: &str, args: Vec<&str>) {
        info!("{} {:?}", command, args.clone());

        let child = Command::new(command)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to execute process");

        let stdout = BufReader::new(child.stdout.unwrap());
        stdout.lines().for_each(|line| debug!("out: {:?}", line)); //

        let stderr = BufReader::new(child.stderr.unwrap());
        stderr.lines().for_each(|line| debug!("err: {:?}", line)); //
    }
}
