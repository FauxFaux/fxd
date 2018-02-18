extern crate argparse;
extern crate regex;
extern crate rustc_serialize;

use std::io;
use std::vec::Vec;

use argparse::{StoreFalse, StoreTrue};

// magic:
use std::io::Read;
use std::io::BufRead;
use std::io::Write;

use rustc_serialize::hex::FromHex;

fn undo_line(
    numbers: bool,
    re: &regex::Regex,
    next_offset: u64,
    line: &str,
) -> Result<Vec<u8>, String> {
    let option_cap = re.captures(line);
    if option_cap.is_none() {
        return Err("invalid line".to_string());
    }
    let cap = option_cap.unwrap();

    if numbers {
        let offset = try!(u64::from_str_radix(&cap[1], 16).map_err(|e| format!(
            "offset looked like hex, but was rejected: '{}': {}",
            &cap[1], e
        )));

        if offset != next_offset {
            return Err(format!(
                "invalid offset, expected {} but was {}",
                next_offset, offset
            ));
        }
    }

    let data = if numbers { &cap[2] } else { &cap[1] };

    let bytes: Vec<u8> = try!(
        data.from_hex()
            .map_err(|e| format!("data ({}) looked like hex, but was rejected: {}", data, e))
    );

    return Ok(bytes);
}

fn undo(numbers: bool) -> Result<(), String> {
    let stdin = io::stdin();
    let mut dest = io::stdout();

    let re = if numbers {
        regex::Regex::new(
            "^(?:0x)?([0-9a-fA-F]+):\
             ((?:\\s*[0-9a-fA-F]+)+)",
        )
    } else {
        regex::Regex::new(r"^((?:\s*[0-9a-fA-F]+)+)\s*$")
    }.unwrap();

    let mut next_offset: u64 = 0;

    for (line_no, line_dumb) in stdin.lock().lines().enumerate() {
        let line = line_dumb.unwrap();
        match undo_line(numbers, &re, next_offset, line.as_str()) {
            Ok(bytes) => {
                next_offset += bytes.len() as u64;
                try!(
                    dest.write(bytes.as_slice())
                        .map_err(|e| format!("writing output failed: {}", e))
                );
            }
            Err(msg) => return Err(format!("error: {} on line {}: {}", msg, line_no, line)),
        };
    }
    return Ok(());
}

fn main() {
    let mut reverse = false;
    let mut numbers = true;
    {
        let mut ap = argparse::ArgumentParser::new();
        ap.set_description("a less rage inducing xxd");
        ap.refer(&mut reverse).add_option(
            &["-r", "--reverse"],
            StoreTrue,
            "reverse operation: convert hexdump into binary",
        );
        ap.refer(&mut numbers).add_option(
            &["-n", "--no-addresses"],
            StoreFalse,
            "don't produce (or require) addresses",
        );
        ap.parse_args_or_exit();
    }

    if reverse {
        undo(numbers).unwrap();
    }

    unimplemented!();
}
