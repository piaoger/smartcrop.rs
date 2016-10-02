# smartcrop.rs

smartcrop implementation in Rust.

smartcrop finds good crops for arbitrary images and crop sizes, based on Jonas Wagner's [smartcrop.js](https://github.com/jwagner/smartcrop.js).

## Installation

Cargo.toml
```
[dependencies.smartcrop]
git = "https://github.com/hhatto/smartcrop.rs.git"
```

## Usage
see [examples directory](https://github.com/hhatto/smartcrop.rs/tree/master/examples)

```console
$ git clone https://github.com/hhatto/smartcrop.rs.git
$ cd smartcrop.rs
$ cargo build --release --example smartcrop
$ ./target/release/examples/smartcrop INPUTFILE.jpg
```
