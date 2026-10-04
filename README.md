# combinefiles

A Rust library that allows you to combine separate files into one, contiguously.

The multithreaded version uses seeked writes to create a sparse file that multiple threads can write to at the same time at different offsets.

## feature flags

- `tracing` -- enables logging with a tracing subscriber
- `cli` -- is required for compiling the example cli app

## tool

This repository also includes an example cli app using the library at [src/bin/cli.rs](https://github.com/Trevrosa/combine/blob/main/src/bin/cli.rs), and can be installed with:

```sh
cargo install combinefiles -F cli
```
or from the repo with:
```sh
cargo install --git https://github.com/Trevrosa/combinefiles -F cli
```
or, with [cargo-binstall](https://github.com/cargo-bins/cargo-binstall):
```sh
cargo binstall combinefiles
```

The installed binary will be called `combine[EXE]`.

```sh
Usage: combine[EXE] [OPTIONS] <OUTPUT> <FILES>...

Arguments:
  <OUTPUT>
  <FILES>...

Options:
  -F, --force-threaded     Force multithreaded mode
  -t, --threads <THREADS>  The max number of threads to use, if multithreaded
  -h, --help               Print help
  -V, --version            Print version
```
