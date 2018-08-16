nvptx toolchain
======

[![Crate](http://meritbadge.herokuapp.com/nvptx)](https://crates.io/crates/nvptx)
[![docs.rs](https://docs.rs/nvptx/badge.svg)](https://docs.rs/nvptx)
[![CircleCI](https://circleci.com/gh/rust-accel/nvptx.svg?style=shield)](https://circleci.com/gh/rust-accel/nvptx)

Compile Rust into PTX/cubin

Install
-----------

### nvptx command

`nvptx` is a CLI tool to

- Build Rust crate into a PTX/cubin file
- install `accel-nvptx` toolchain (for rustup)

```
cargo install nvptx
```

### accel-nvptx toolchain

`accel-nvptx` is a toolchain name for rustup. It contains `rustc` and runtime libraries with `nvptx64-nvidia-cuda` target.

```
nvptx install
```

This installs `accel-nvptx` toolchain to `rustup` like:

```
$ rustup toolchain list
stable-x86_64-unknown-linux-gnu
nightly-x86_64-unknown-linux-gnu (default)
accel-nvptx
```

This toolchain is built from
  - [rust-accel/rust](https://github.com/rust-accel/rust)
  - [rust-accel/libc](https://github.com/rust-accel/libc)

Build
------

You can build your crate using `accel-nvptx` toolchain into a PTX file

```
nvptx build
```

This consists of following three steps:

- Compile Rust into LLVM bitcode. This step corresponds to the following command:

```
cargo +accel-nvptx build --target nvptx64-nvidia-cuda
```

- Link rlib into a LLVM bitcode using `llvm-link`
- Drop unused bitcode using `opt`
- Compile LLVM bitcode into PTX using `llc`
- (Optional) Convert PTX to cubin using `nvcc`
