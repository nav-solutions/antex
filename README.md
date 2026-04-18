ANTEX
=====

[![Rust](https://github.com/nav-solutions/antex/actions/workflows/rust.yml/badge.svg)](https://github.com/nav-solutions/antex/actions/workflows/rust.yml)
[![Rust](https://github.com/nav-solutions/antex/actions/workflows/daily.yml/badge.svg)](https://github.com/nav-solutions/antex/actions/workflows/daily.yml)
[![crates.io](https://docs.rs/antex-rs/badge.svg)](https://docs.rs/antex-rs/)
[![crates.io](https://img.shields.io/crates/d/antex-rs.svg)](https://crates.io/crates/antex-rs)
[![discord server](https://img.shields.io/discord/1342922474110586910?logo=discord)](https://discord.gg/EqhEBXBmJh)

[![MRSV](https://img.shields.io/badge/MSRV-1.88.0-orange?style=for-the-badge)](https://github.com/rust-lang/rust/releases/tag/1.89.0)
[![License](https://img.shields.io/badge/license-MPL_2.0-orange?style=for-the-badge&logo=mozilla)](https://github.com/nav-solutions/antex/blob/main/LICENSE)

ANTEX is a file format derived from the [RINEX (Receiver Independent EXchange)](https://en.wikipedia.org/wiki/RINEX) file format, to contain a database of calibration parameters for
ground and satellite antennas. These parameters are required for precise navigation.

This library proposes a dedicated parser for this format, to easily deploy in an application.
It will try to propose a formatting ecosystem as well, to create custom or updated
databases using your own specifications.

To contribute to either of our project or join our community, you way
- open an [Issue on Github.com](https://github.com/nav-solutions/antex/issues) 
- follow our [Discussions on Github.com](https://github.com/nav-solutions/discussions)
- join our [Discord channel](https://discord.gg/EqhEBXBmJh)

## Advantages

- Fast
- Open sources: read and access all the code
- Seamless Gzip decompression (on `flate2` feature)

## Other RINEX formats

Our framework proposes a parser for other RINEX formats, in particular:
- [RINEX (Receiver INdependent EXchange)](https://github.com/nav-solutions/rinex)
  - provides support for Observations, Meteo observations, Navigation message and
  Clock data
  - supports file formatting
  - integrates a modern rewrite of the CRX2RNX (decompression) and RNX2CRX
  (compression) algorithm specified by Y. Hatanaka.
- [IONEX (Ionosphere Maps)](https://github.com/nav-solutions/ionex)
- [DORIS (special observations)](https://github.com/nav-solutions/doris)
- [SINEX (daily corrections) is work in progress](https://github.com/nav-solutions/sinex)

## Citation and referencing

If you need to reference this work, please use the following model:

`Nav-solutions (2026), ANTEX (MPLv2), https://github.com/nav-solutions`
