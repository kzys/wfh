# wfh

![GitHub Actions](https://github.com/kzys/wfh/workflows/Rust/badge.svg)

wfh continuously watches your local directories and `rsync` them against
a remote host.

## Why?

If you

- Want to edit files on your local machine, because of IDE, latency, ...
- Need to build that and run tests on your remote machine, because of dependencies.
- Don't want to have a daemon such as `lsyncd`. Okay to run something on your terminal.
- Don't want to send `node_modules` or such to the remote machine. It may not work.
- Use Git and most of files you are editing are managed by Git.

Then `wfh` makes working from *home*, your local machine easier!

## License

The MIT License
