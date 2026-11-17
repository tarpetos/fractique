# Third-party notices

Fractique itself is licensed under the [PolyForm Noncommercial License 1.0.0](LICENSE.md). It is built on the Rust
crates below, each under its own license. The list is what `Cargo.lock` resolves to for every platform; the full
license texts are in each crate's repository and in its package on [crates.io](https://crates.io).

| Crate | Version | License |
| --- | --- | --- |
| adler2 | 2.0.1 | 0BSD OR MIT OR Apache-2.0 |
| anstream | 1.0.0 | MIT OR Apache-2.0 |
| anstyle | 1.0.14 | MIT OR Apache-2.0 |
| anstyle-parse | 1.0.0 | MIT OR Apache-2.0 |
| anstyle-query | 1.1.5 | MIT OR Apache-2.0 |
| anstyle-wincon | 3.0.11 | MIT OR Apache-2.0 |
| bitflags | 2.13.2 | MIT OR Apache-2.0 |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 |
| clap | 4.6.8 | MIT OR Apache-2.0 |
| clap_builder | 4.6.7 | MIT OR Apache-2.0 |
| clap_derive | 4.6.8 | MIT OR Apache-2.0 |
| clap_lex | 1.1.2 | MIT OR Apache-2.0 |
| colorchoice | 1.0.5 | MIT OR Apache-2.0 |
| crc32fast | 1.5.5 | MIT OR Apache-2.0 |
| crossbeam-deque | 0.8.8 | MIT OR Apache-2.0 |
| crossbeam-epoch | 0.9.21 | MIT OR Apache-2.0 |
| crossbeam-utils | 0.8.23 | MIT OR Apache-2.0 |
| either | 1.19.0 | MIT OR Apache-2.0 |
| fdeflate | 0.3.7 | MIT OR Apache-2.0 |
| flate2 | 1.1.11 | MIT OR Apache-2.0 |
| heck | 0.5.0 | MIT OR Apache-2.0 |
| is_terminal_polyfill | 1.70.2 | MIT OR Apache-2.0 |
| itoa | 1.0.19 | MIT OR Apache-2.0 |
| libm | 0.2.16 | MIT |
| memchr | 2.8.3 | Unlicense OR MIT |
| miniz_oxide | 0.8.9 | MIT OR Zlib OR Apache-2.0 |
| miniz_oxide | 0.9.1 | MIT OR Zlib OR Apache-2.0 |
| once_cell_polyfill | 1.70.2 | MIT OR Apache-2.0 |
| png | 0.18.1 | MIT OR Apache-2.0 |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 |
| quote | 1.0.47 | MIT OR Apache-2.0 |
| rayon | 1.12.0 | MIT OR Apache-2.0 |
| rayon-core | 1.13.0 | MIT OR Apache-2.0 |
| serde | 1.0.229 | MIT OR Apache-2.0 |
| serde_core | 1.0.229 | MIT OR Apache-2.0 |
| serde_derive | 1.0.229 | MIT OR Apache-2.0 |
| serde_json | 1.0.154 | MIT OR Apache-2.0 |
| simd-adler32 | 0.3.10 | MIT |
| strsim | 0.11.1 | MIT |
| syn | 3.0.10 | MIT OR Apache-2.0 |
| unicode-ident | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 |
| utf8parse | 0.2.2 | Apache-2.0 OR MIT |
| windows-link | 0.2.1 | MIT OR Apache-2.0 |
| windows-sys | 0.61.2 | MIT OR Apache-2.0 |
| zlib-rs | 0.6.8 | Zlib |
| zmij | 1.0.23 | MIT |

## The fractal flame algorithm

The renderer implements the fractal flame algorithm published by Scott Draves and Erik Reckase
(["The Fractal Flame Algorithm"](https://flam3.com/flame_draves.pdf)), including variation formulas listed in that
paper. The code is written from the published formulas: no code from flam3, Apophysis, Fractorium or any other
renderer is included.
