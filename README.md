# EOLib

[![Build Status][actions-badge]][actions-url]
[![Crate][crates-badge]][crates-url]
[![Docs][docs-badge]][docs-url]
[![License][mit-badge]][mit-url]

[crates-badge]: https://img.shields.io/crates/v/eolib.svg
[crates-url]: https://crates.io/crates/eolib
[docs-badge]: https://img.shields.io/docsrs/eolib.svg
[docs-url]: https://docs.rs/eolib
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/sorokya/eolib-rs/blob/master/LICENSE
[actions-badge]: https://github.com/sorokya/eolib-rs/actions/workflows/rust.yml/badge.svg
[actions-url]: https://github.com/sorokya/eolib-rs/actions/workflows/rust.yml

A core rust library for writing applications related to Endless Online

## Features

Read and write the following EO data structures:

- Client packets
- Server packets
- Endless Map Files (EMF)
- Endless Item Files (EIF)
- Endless NPC Files (ENF)
- Endless Spell Files (ESF)
- Endless Class Files (ECF)

Utilities:

- Data reader
- Data writer
- Number encoding
- String encoding
- Data encryption
- Packet sequencer
