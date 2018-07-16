nvptx toolchain
======

[![Crate](http://meritbadge.herokuapp.com/nvptx)](https://crates.io/crates/nvptx)
[![docs.rs](https://docs.rs/nvptx/badge.svg)](https://docs.rs/nvptx)
[![CircleCI](https://circleci.com/gh/rust-accel/nvptx.svg?style=shield)](https://circleci.com/gh/rust-accel/nvptx)

Compile Rust into PTX

Install
--------

nvptx command for manging nvptx-toolchain for Rust can be installed from crate.io

```
cargo install nvptx
```

And then, you can install nvptx-toolchain (including `rustc`, `rust-std` for `nvptx64-nvidia-cuda` target):

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
- Compile LLVM bitcode into PTX using `llc`
