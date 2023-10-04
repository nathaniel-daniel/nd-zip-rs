# nd-zip-rs
A zip tool in Rust, specifically designed to be resilient against files which have non-utf8 paths.
Where 7-Zip produces [mojibake](https://en.wikipedia.org/wiki/Mojibake), this produces valid unicode filenames.
This program will use Firefox's [`chardetng`](https://github.com/hsivonen/chardetng) library to guess the file encoding before extracting.

## Usage
Currently only extraction is supported.
It is used as follows:
```bash
nd-zip extract input.zip -o output-directory
```

## License
Licensed under either of
 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.