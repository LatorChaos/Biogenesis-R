// SPDX-License-Identifier: GPL-2.0-or-later

use biogenesis_core::object_stream::{Limits, Node, Record, StreamReader, Value, class_schema, encode_document};
use std::error::Error;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;

const MAX_BYTES: usize = 1_048_576;
const ROOTS: [(&str, &str); 4] = [
    ("gene.ser", "biogenesis.Gene"),
    ("genome.ser", "biogenesis.GeneticCode"),
    ("statistics.ser", "biogenesis.WorldStatistics"),
    ("world.ser", "biogenesis.World"),
];

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn main() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<_> = std::env::args_os().collect();
    if arguments.len() != 3 {
        return Err(invalid("Usage: object_stream_write INPUT_DIRECTORY NEW_OUTPUT_DIRECTORY").into());
    }
    let input = Path::new(&arguments[1]);
    let output = Path::new(&arguments[2]);
    fs::create_dir(output)?;
    let limits = Limits {
        input_bytes: MAX_BYTES,
        output_bytes: MAX_BYTES,
        handles: 10_000,
        array_elements: 20_000,
        total_array_elements: 20_000,
        allocated_bytes: 16 * 1024 * 1024,
        work: 2_000_000,
        ..Limits::default()
    };
    for (name, expected_class) in ROOTS {
        let path = input.join(name);
        let metadata = fs::symlink_metadata(&path)?;
        if !metadata.is_file() || metadata.len() > MAX_BYTES as u64 {
            return Err(invalid("A bounded regular input file is required").into());
        }
        let mut bytes = Vec::new();
        File::open(path)?.take(MAX_BYTES as u64 + 1).read_to_end(&mut bytes)?;
        if bytes.len() > MAX_BYTES {
            return Err(invalid("Input grew beyond the declared byte budget").into());
        }
        let document = StreamReader::new(&bytes, limits)?.read_document()?;
        let [Record::Value(Value::Handle(root))] = document.records.as_slice() else {
            return Err(invalid("Exactly one object root is required").into());
        };
        let Node::Object(object) = document.arena.node(*root)? else {
            return Err(invalid("The root is not an object").into());
        };
        if class_schema(document.arena.descriptor(object.descriptor)?.class).name != expected_class {
            return Err(invalid("The input root class differs from its declared filename").into());
        }
        let encoded = encode_document(&document, limits)?;
        let mut file = OpenOptions::new().write(true).create_new(true).open(output.join(name))?;
        file.write_all(&encoded)?;
        file.sync_all()?;
        println!("OBJECT_STREAM_WRITE root={name} consumed={} handles={} bytes={}",
            document.consumed, document.arena.nodes.len(), encoded.len());
    }
    println!("OBJECT_STREAM_WRITE PASS roots=4");
    Ok(())
}
