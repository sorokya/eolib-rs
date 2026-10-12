#!/usr/bin/env python3
"""Regenerate tests/packet_roundtrip.rs from the eo-captured-packets submodule.

The captured-packet JSON files (the same ones used by ../eolib-c to generate its
packet_tests.c) map one-to-one onto generated packet types: a file whose top-level
"family" and "action" are `Family` and `Action` models the packet type
`{Family}{Action}{Client|Server}Packet`. This script scans the submodule and emits the
registry table used by tests/packet_roundtrip.rs, then writes the complete test file.

Run from the repository root:

    python3 tools/generate_packet_roundtrip.py
"""

import json
import os

ROOT = os.path.join(os.path.dirname(__file__), "..")
CAPTURED = os.path.join(ROOT, "eo-captured-packets")
OUTPUT = os.path.join(ROOT, "tests", "packet_roundtrip.rs")

TEMPLATE = '''//! Round-trip tests for every packet captured in `eo-captured-packets`.
//!
//! Each JSON file in the [`eo-captured-packets`] submodule (the same data used by
//! `../eolib-c` to generate its `packet_tests.c`) contains the raw packet body as
//! produced by the original v28 client/server. These tests feed those bytes through
//! the matching generated packet type and assert that serializing the deserialized
//! packet reproduces the original bytes exactly, covering both directions at once.
//!
//! [`eo-captured-packets`]: https://github.com/ethanmoffat/eo-captured-packets

use std::{{fs, path::Path}};

use bytes::Bytes;
use eolib::data::{{EoReader, EoSerialize, EoWriter}};

/// Signature of a round-trip function for a single packet type.
type Roundtrip = fn(&[u8]) -> Result<Vec<u8>, String>;

/// Table mapping a captured packet's direction, family, and action to the generated
/// packet type that models it. Kept in sync with the captured packets by
/// `tools/generate_packet_roundtrip.py`.
#[rustfmt::skip]
const PACKETS: &[(&str, &str, &str, Roundtrip)] = &[
{registry}
];

/// Deserializes `data` as `T` and re-serializes it, returning the resulting bytes.
fn roundtrip<T: EoSerialize>(data: &[u8]) -> Result<Vec<u8>, String> {{
    let reader = EoReader::new(Bytes::copy_from_slice(data));
    let packet = T::deserialize(&reader).map_err(|e| e.to_string())?;

    let mut writer = EoWriter::new();
    packet.serialize(&mut writer).map_err(|e| e.to_string())?;

    Ok(writer.to_byte_array().to_vec())
}}

fn decode_base64(input: &str) -> Vec<u8> {{
    fn value(byte: u8) -> Option<u8> {{
        match byte {{
            b'A'..=b'Z' => Some(byte - b'A'),
            b'a'..=b'z' => Some(byte - b'a' + 26),
            b'0'..=b'9' => Some(byte - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }}
    }}

    let mut output = Vec::with_capacity(input.len() / 4 * 3);
    let bytes: Vec<u8> = input
        .bytes()
        .filter(|b| *b != b'\\n' && *b != b'\\r')
        .collect();

    for chunk in bytes.chunks(4) {{
        let a = value(chunk[0]).expect("invalid base64 input");
        let b = value(chunk[1]).expect("invalid base64 input");
        output.push((a << 2) | (b >> 4));

        if chunk.len() > 2 && chunk[2] != b'=' {{
            let c = value(chunk[2]).expect("invalid base64 input");
            output.push((b << 4) | (c >> 2));

            if chunk.len() > 3 && chunk[3] != b'=' {{
                let d = value(chunk[3]).expect("invalid base64 input");
                output.push((c << 6) | d);
            }}
        }}
    }}

    output
}}

#[test]
fn captured_packets_roundtrip() {{
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("eo-captured-packets");
    assert!(
        root.is_dir(),
        "captured packets are missing at {{}}. Initialize submodules with `git submodule update --init --recursive`.",
        root.display()
    );

    let mut tested = 0;
    let mut failures: Vec<String> = Vec::new();

    for source in ["client", "server"] {{
        let dir = root.join(source);
        let mut paths: Vec<_> = fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("failed to read {{}}: {{e}}", dir.display()))
            .map(|entry| entry.unwrap().path())
            .filter(|path| {{
                path.extension().and_then(|extension| extension.to_str()) == Some("json")
            }})
            .collect();
        paths.sort();

        for path in paths {{
            let text = fs::read_to_string(&path).unwrap();
            let value: serde_json::Value = serde_json::from_str(&text).unwrap();

            let family = value["family"].as_str().unwrap();
            let action = value["action"].as_str().unwrap();
            let expected = decode_base64(value["expected"].as_str().unwrap_or(""));

            let Some((_, _, _, roundtrip)) = PACKETS
                .iter()
                .find(|(s, f, a, _)| *s == source && *f == family && *a == action)
            else {{
                failures.push(format!(
                    "{{}}: no generated packet registered for {{source}}/{{family}}/{{action}}",
                    path.display()
                ));
                continue;
            }};

            match roundtrip(&expected) {{
                Ok(actual) if actual == expected => {{}}
                Ok(actual) => failures.push(format!(
                    "{{}}: round-trip mismatch\\n    expected: {{expected:02x?}}\\n    actual:   {{actual:02x?}}",
                    path.display()
                )),
                Err(error) => failures.push(format!("{{}}: {{error}}", path.display())),
            }}

            tested += 1;
        }}
    }}

    assert!(tested > 0, "no captured packets were tested");
    assert!(
        failures.is_empty(),
        "{{}} of {{tested}} captured packets failed to round-trip:\\n{{}}",
        failures.len(),
        failures.join("\\n")
    );
}}
'''


def collect_registry():
    entries = {}
    for direction, suffix in (("client", "Client"), ("server", "Server")):
        directory = os.path.join(CAPTURED, direction)
        for name in sorted(os.listdir(directory)):
            if not name.endswith(".json"):
                continue
            path = os.path.join(directory, name)
            if os.path.isdir(path):
                continue
            with open(path) as handle:
                data = json.load(handle)
            family, action = data["family"], data["action"]
            packet = f"{family}{action}{suffix}Packet"
            key = (direction, family, action)
            if key in entries and entries[key] != packet:
                raise SystemExit(f"conflicting packet types for {key}: {entries[key]} vs {packet}")
            entries[key] = packet
    return entries


def main():
    entries = collect_registry()
    registry = "\n".join(
        f'    ("{direction}", "{family}", "{action}", '
        f"roundtrip::<eolib::protocol::net::{direction}::{packet}>),"
        for (direction, family, action), packet in entries.items()
    )
    with open(OUTPUT, "w") as handle:
        handle.write(TEMPLATE.format(registry=registry))
    print(f"wrote {OUTPUT} with {len(entries)} packet types")


if __name__ == "__main__":
    main()
