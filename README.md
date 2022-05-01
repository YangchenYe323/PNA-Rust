# Practical Networked Application in Rust [![ci](https://github.com/YangchenYe323/PNA-Rust/actions/workflows/ci.yml/badge.svg)](https://github.com/YangchenYe323/PNA-Rust/actions/workflows/ci.yml)


This is my repo for implementing projects offered by PingCAP's Talent-Plan course [PNA in Rust](https://github.com/pingcap/talent-plan/tree/master/courses/rust)

## Usage

If by any chance you want to run this project:

```Bash
git clone https://github.com/YangchenYe323/PNA-Rust.git
cd PNA-Rust
cargo test
```

This will automatically build all the five projects in the workspace and run all the testcases. The project also comes with binary crates for running. The latest version is `kvs-client5` and `kvs-server5`, which can be run by `cargo run --bin kvs-server5 -- <addr> <engine> [subcommand]` and `cargo run --bin kvs-client5 -- <addr> [subcommand]`.
