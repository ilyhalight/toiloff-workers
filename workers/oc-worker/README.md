# oc-worker

Client-side (local machine) worker that collects sessions usage stats from OpenCode and sends them to [backend](https://github.com/ilyhalight/toiloff-backend).

## Configuration

The worker requires the following environment variables to be set:

- `API_BASE_URL` - The base URL of the backend API (e.g., `http://localhost:3001`)
- `API_SERVICE_TOKEN` - The secret service token for authenticating with the backend API

## Install

You can download `oc-worker` for linux from [Releases](https://github.com/ilyhalight/toiloff-workers/releases).

## Build

1. Install [Rust 1.75+](https://www.rust-lang.org/learn/get-started)
2. Clone this repository

```bash
git clone https://github.com/ilyhalight/toiloff-workers
```

3. Navigate to the `workers/oc-worker` directory
4. Rename `.example.env` to `.env` and fill them with your own values
5. Run `cargo build --release` to build the worker

To periodicaly run the worker, you can use a cron utils

### Multi-platform build

1. Install [Zig](https://ziglang.org/download/)
2. Install `cargo-zigbuild`

```bash
cargo install --locked cargo-zigbuild
```

3. Add targets to build

```bash
rustup target add aarch64-apple-darwin x86_64-unknown-linux-gnu x86_64-pc-windows-gnu aarch64-unknown-linux-musl
```

4. Build for the target platform

```bash
cargo zigbuild --release --target <platform>
```
