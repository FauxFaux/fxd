extern crate clap;

#[macro_use]
extern crate error_chain;
extern crate hex;
extern crate regex;

use std::io;
use std::io::BufRead;
use std::io::Write;

use clap::Arg;

mod errors;
use errors::*;

fn undo_line(numbers: bool, re: &regex::Regex, next_offset: u64, line: &str) -> Result<Vec<u8>> {
    let cap = match re.captures(line) {
        Some(cap) => cap,
        None => bail!("invalid line"),
    };

    if numbers {
        let offset = u64::from_str_radix(&cap[1], 16).map_err(|e| {
            format!(
                "offset looked like hex, but was rejected: '{}': {}",
                &cap[1], e
            )
        })?;

        if offset != next_offset {
            bail!(
                "invalid offset, expected {} but was {}",
                next_offset,
                offset
            );
        }
    }

    let data = if numbers { &cap[2] } else { &cap[1] };

    let bytes = hex::decode(data)
        .map_err(|e| format!("data ({}) looked like hex, but was rejected: {}", data, e))?;

    return Ok(bytes);
}

fn undo(numbers: bool) -> Result<()> {
    let stdin = io::stdin();
    let mut dest = io::stdout();

    let re = if numbers {
        regex::Regex::new(
            "^(?:0x)?([0-9a-fA-F]+):\
             ((?:\\s*[0-9a-fA-F]+)+)",
        )
    } else {
        regex::Regex::new(r"^((?:\s*[0-9a-fA-F]+)+)\s*$")
    }.expect("compiling static string");

    let mut next_offset: u64 = 0;

    for (line_no, line) in stdin.lock().lines().enumerate() {
        let line = line?;
        match undo_line(numbers, &re, next_offset, line.as_str()) {
            Ok(bytes) => {
                next_offset += bytes.len() as u64;
                dest.write(bytes.as_slice())
                    .map_err(|e| format!("writing output failed: {}", e))?;
            }
            Err(msg) => bail!("error: {} on line {}: {}", msg, line_no, line),
        };
    }
    return Ok(());
}

fn run() -> Result<()> {
    let matches = clap::App::new("fxd")
        .about("a less rage inducing xxd")
        .arg(
            Arg::with_name("reverse")
                .short("r")
                .help("reverse operation: convert hexdump into binary"),
        )
        .arg(
            Arg::with_name("no-addresses")
                .short("n")
                .help("don't produce (or require) addresses"),
        )
        .get_matches();

    let reverse = matches.is_present("reverse");
    let numbers = !matches.is_present("no-addresses");

    if reverse {
        undo(numbers)?;
    }

    unimplemented!();
}

quick_main!(run);
