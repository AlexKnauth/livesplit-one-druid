# LiveSplit One Druid

A prototype Desktop version of LiveSplit One, using the Druid framework and the multiplatform
[livesplit-core][livesplit-core] library.

The Web Version is available at [one.livesplit.org](https://one.livesplit.org/).

## Installation Instructions

Download the latest release for your operating system and archictecture here:
https://github.com/AlexKnauth/livesplit-one-druid/releases/latest

When you run LiveSplit One Druid,
it needs to have permission to read memory of other processes.
- On Mac, that might require running it under `sudo`.
- On Linux, give it permission with one of:
  - setting the capabilities to include `CAP_SYS_PTRACE`, which can be done with
    `sudo setcap CAP_SYS_PTRACE=+eip LiveSplitOne` or some variation of that
  - setting `/proc/sys/kernel/yama/ptrace_scope` to 0, which can be done with
    `echo "0"|sudo tee /proc/sys/kernel/yama/ptrace_scope`
  - running it under `sudo`
- On Windows, it should just work. Windows allows memory reading by default.

## Build Instructions

In order to build LiveSplit One you need the [Rust
Compiler](https://www.rust-lang.org/). You can then build and run the project
with:

```bash
cargo run
```

In order to build and run a release build, use the following command:

```bash
cargo run --release
```

## Configuration

The config file and log file are located in the local data directory from [data_local_dir][data_local_dir]:
- Windows: `C:\Users\<name>\AppData\Local\LiveSplit\LiveSplit One\data\config.yml`
- Mac: `/Users/<name>/Library/Application Support/org.LiveSplit.LiveSplit-One/config.yml`
- Linux: `/home/<name>/.local/share/livesplitone/config.yml` or `/root/.local/share/livesplitone/config.yml` if running as `sudo`

  [data_local_dir]: https://docs.rs/directories/latest/directories/struct.ProjectDirs.html#method.data_local_dir
  [livesplit-core]: https://github.com/LiveSplit/livesplit-core
