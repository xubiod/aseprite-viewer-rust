# Aseprite file loading

This module should be able to be used in any scenario for reading out Aseprite files
as of writing.

## Usage

Call `aseprite::read` with any type that implements the [`std::io::Read`](https://doc.rust-lang.org/std/io/trait.Read.html)
and [`std::io::Seek`](https://doc.rust-lang.org/std/io/trait.Seek.html) traits from
the Rust standard library.

The structs used by `Aseprite` are tightly tied to the file contents can may contain
unneeded and redundant data.
