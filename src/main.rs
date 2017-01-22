extern crate byteorder;

use byteorder::{LittleEndian, ByteOrder};
use std::convert::From;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::str;

const GCNO_MAGIC: u32 = 0x67636e6f;
const TAG_FUNCTION: u32 = 0x01000000;
const TAG_LINE_COUNT: u32 = 0x01450000;

fn main() {
    let args: Vec<String> = env::args().collect();

    if let Some(gcno_path) = args.get(1) {
        println!("Opening gcno file: {}", &gcno_path);
        let path = Path::new(&gcno_path);
        let mut file = match File::open(&path) {
            Err(e) => {
                writeln!(std::io::stderr(), "Failed to open {}:{}", &gcno_path, e.description()).unwrap();
                std::process::exit(1);
            }
            Ok(file) => file
        };

        let mut buffer = Vec::<u8>::new();
        file.read_to_end(&mut buffer).unwrap();

        let mut offset = match parse_header(&buffer) {
            Ok(offset) => offset,
            Err(ParseError { code }) => std::process::exit(code),
        };

        let tag = LittleEndian::read_u32(&buffer[offset + 0..offset + 4]);
        let length = LittleEndian::read_u32(&buffer[offset + 4..offset + 8]) * 4; // file gives length in u32 words

        offset += 8;

        let record_offset = match tag {
            TAG_FUNCTION => {
                let function_record = match parse_function_record(&buffer[offset..offset+(length as usize)]) {
                    Ok(tuple) => tuple,
                    Err(ParseError { code }) => std::process::exit(code),
                };
                println!("{}|{}|{}", function_record.1.src_path, function_record.1.function_name, function_record.1.line_number);
                function_record.0
            },
            TAG_LINE_COUNT => 2,
            _ => length as usize, // skip record, it's not useful to us
        };

    } else {
        println!("Usage: lcov-rs PATH_TO_GCNO");
    }
}

fn parse_header(buffer: &[u8]) -> Result<usize, ParseError> {
    if GCNO_MAGIC != LittleEndian::read_u32(&buffer[0..4]) {
        writeln!(std::io::stderr(),
                 "Invalid magic bytes. Could be an endian issue if on non-Linux").unwrap();
        return Err(ParseError::new(2));
    };

    println!("version: {}", str::from_utf8(&buffer[4..8])?);
    return Ok(12); // Read magic, version, skip stamp
}

fn parse_function_record(buffer: &[u8]) -> Result<(usize, FunctionRecord), ParseError> {
    // Skip identifier, lineno_checksum, cfg_checksum, 12 bytes
    let name_length = (LittleEndian::read_u32(&buffer[12..16]) * 4) as usize;
    let name = str::from_utf8(&buffer[16..16 + name_length])?;
    let src_path_length = (LittleEndian::read_u32(&buffer[16 + name_length..20 + name_length]) * 4) as usize;
    let src_path = str::from_utf8(&buffer[20 + name_length..20 + name_length + src_path_length])?;
    let line_number = LittleEndian::read_u32(&buffer[20 + name_length + src_path_length..24 + name_length + src_path_length]) * 4;

    return Ok((24 + name_length + src_path_length, FunctionRecord {
        src_path: src_path.to_owned(),
        function_name: name.to_owned(),
        line_number: line_number,
    }));
}

struct FunctionRecord {
    src_path: String,
    function_name: String,
    line_number: u32,
}

struct ParseError {
    code: i32,
}

impl ParseError {
    fn new(code: i32) -> ParseError {
        return ParseError { code: code };
    }
}

impl From<str::Utf8Error> for ParseError {
    fn from(_: str::Utf8Error) -> ParseError {
        return ParseError { code: 3 };
    }
}