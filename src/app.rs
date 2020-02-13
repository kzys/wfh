extern crate tempfile;

use ignore::gitignore::GitignoreBuilder;
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::channel;
use std::time;
use std::time::Duration;

pub struct App {
    host: String,
    dirs: Vec<PathBuf>,
    remote_home: String,
    local_home: String,
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

fn capture_stdin(command: &str, args: Vec<&str>) -> Result<String, std::string::FromUtf8Error> {
    info!("{} {:?}", command, args.clone());

    let out = Command::new(command)
        .args(args)
        .output()
        .expect("failed to execute process");

    String::from_utf8(out.stdout)
}

impl App {
    pub fn new(host: String, dirs: Vec<String>) -> Result<App, Box<dyn std::error::Error>> {
        let mut dirs_to_sync: Vec<std::path::PathBuf> = vec![];

        for parent in dirs {
            for dir in fs::read_dir(&parent)? {
                dirs_to_sync.push(dir?.path())
            }
        }

        let remote_home = capture_stdin("ssh", vec![&host, "echo", "-n", "$HOME"])?;
        let local_home = env::var("HOME")?;

        Ok(App {
            host,
            dirs: dirs_to_sync,
            remote_home,
            local_home,
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
        let mut dirs_set = HashSet::new();
        for dir in &self.dirs {
            dirs_set.insert(dir.to_owned());
        }
        self.sync_dirs(&dirs_set);

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
                    self.find_dir_to_sync(&event)
                        .map(|x| dirs_set.insert(x.clone()));
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
        let remote_dir = self.remote_dir(dir);

        let mut git_dir = dir.clone();
        git_dir.push(".git");

        let exclude_file = if git_dir.is_dir() {
            let mut file = tempfile::NamedTempFile::new().unwrap();
            let output = Command::new("git")
                .arg("-C")
                .arg(dir)
                .args(vec!["ls-files", "--exclude-standard", "-oi", "--directory"])
                .output()
                .expect("failed to execute process");
            file.write_all(&output.stdout);
            Some(file.into_temp_path())
        } else {
            None
        };

        let mkdir = Command::new("ssh")
            .arg(&self.host)
            .args(vec!["mkdir", "-p", &remote_dir])
            .output()
            .expect("failed to execute process");

        let mut rsync = Command::new("rsync");
        rsync.args(vec!["--archive", "--verbose"]);

        exclude_file.as_ref().map(|path| {
            rsync.arg("--exclude-from");
            rsync.arg(path.as_os_str());
        });

        let mut src = dir.clone();
        src.push("/");
        rsync.arg(src);

        rsync.arg(format!("{}:{}/", self.host, remote_dir));

        self.spawn_and_wait(&mut rsync);

        if git_dir.is_dir() {
            self.sync_git_dir(dir);
        }
    }

    fn sync_git_dir(&self, dir: &PathBuf) {
        let mut git_dir = dir.clone();
        git_dir.push(".git/");

        let mut rsync = Command::new("rsync");
        rsync.args(vec!["--archive", "--verbose", "--delete"]);
        rsync.arg(git_dir.as_os_str());

        let remote_dir = self.remote_dir(dir);
        rsync.arg(format!("{}:{}/.git/", self.host, remote_dir));

        self.spawn_and_wait(&mut rsync);
    }

    fn remote_dir(&self, path: &PathBuf) -> String {
        let mut s = path.to_string_lossy().to_string();
        if let Some(begin) = s.find(&self.local_home) {
            if begin == 0 {
                s.replace_range(..self.local_home.len(), &self.remote_home)
            }
        }
        s
    }

    fn spawn_and_wait(&self, command: &mut Command) -> bool {
        debug!("spwan {:?}", command);

        let mut child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to execute process");

        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            reader.lines().for_each(|line| trace!("out: {:?}", line));
        }

        if let Some(stderr) = child.stderr.take() {
            let reader = BufReader::new(stderr);
            reader.lines().for_each(|line| error!("err: {:?}", line));
        }

        if let Ok(exit) = child.wait() {
            exit.success()
        } else {
            false
        }
    }
}
