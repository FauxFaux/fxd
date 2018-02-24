extern crate clap;

#[macro_use]
extern crate error_chain;
extern crate iowrap;
extern crate regex;

use std::fs;
use std::io;
use std::io::BufRead;
use std::io::Read;
use std::io::Write;

use clap::Arg;
use iowrap::ReadMany;

mod errors;
use errors::*;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn undo_line(
    numbers: bool,
    re: &regex::Regex,
    next_offset: u64,
    line: &str,
    mut nibble: Option<u8>,
) -> Result<(Vec<u8>, Option<u8>)> {
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

    let mut bytes = Vec::with_capacity(data.len() / 2);

    for byte in data.bytes() {
        let part = match byte {
            c @ b'0'...b'9' => c - b'0',
            c @ b'a'...b'f' => c + 10 - b'a',
            c @ b'A'...b'F' => c + 10 - b'A',
            other if other.is_ascii_whitespace() => continue,
            other => bail!("invalid character in stream: {:?}", other),
        };

        match nibble {
            Some(first) => {
                bytes.push(first * 0x10 + part);
                nibble = None;
            }
            None => nibble = Some(part),
        };
    }

    Ok((bytes, nibble))
}

fn undo<R: BufRead>(input: R, numbers: bool) -> Result<()> {
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

    let mut nibble = None;
    for (line_no, line) in input.lines().enumerate() {
        let line = line?;
        match undo_line(numbers, &re, next_offset, line.as_str(), nibble) {
            Ok((bytes, new_nibble)) => {
                next_offset += bytes.len() as u64;
                dest.write(bytes.as_slice())
                    .map_err(|e| format!("writing output failed: {}", e))?;
                nibble = new_nibble;
            }
            Err(msg) => bail!("error: {} on line {}: {}", msg, line_no, line),
        };
    }
    Ok(())
}

fn encode_xxd<R: Read>(mut input: R, numbers: bool, width: usize) -> Result<()> {
    let mut off = 0;
    let mut line = vec![0u8; width].into_boxed_slice();
    loop {
        let read = input.read_many(&mut line)?;
        if 0 == read {
            break;
        }

        if numbers {
            print!("{:08x}: ", off);
        }

        for i in 0..width {
            if i < read {
                print!("{:02x}", line[i]);
            } else {
                print!("  ");
            }

            if i % 2 == 1 {
                print!(" ");
            }
        }

        print!(" ");

        if width % 2 == 1 {
            print!(" ");
        }

        for i in 0..width {
            if i < read {
                let c = line[i];
                if c.is_ascii_graphic() {
                    print!("{}", c as char);
                } else {
                    print!(".");
                }
            }
        }

        println!();
        off += read;
    }

    Ok(())
}

fn encode_code<R: Read>(mut input: R, numbers: bool, width: usize) -> Result<()> {
    let mut off = 0;
    let mut line = vec![0u8; width].into_boxed_slice();
    loop {
        let read = input.read_many(&mut line)?;
        if 0 == read {
            break;
        }

        if numbers {
            print!("/* {:04x} */ ", off);
        }

        for i in 0..width {
            if i < read {
                print!("0x{:02x}, ", line[i]);
            } else {
                print!("      ");
            }
        }

        print!("// ");

        for i in 0..width {
            if i < read {
                let c = line[i];
                if c.is_ascii_graphic() {
                    print!("{}", c as char);
                } else {
                    print!(".");
                }
            }
        }

        println!();
        off += read;
    }

    Ok(())
}

fn run() -> Result<()> {
    let matches = clap::App::new("fxd")
        .version(VERSION)
        .about("a less rage inducing xxd")
        .arg(
            Arg::with_name("reverse")
                .long("reverse")
                .short("r")
                .help("reverse operation: convert hexdump into binary"),
        )
        .arg(
            Arg::with_name("no-addresses")
                .long("no-addresses")
                .short("n")
                .help("don't produce (or require) addresses"),
        )
        .arg(
            Arg::with_name("width")
                .long("width")
                .short("w")
                .default_value("16")
                .help("output this many bytes per line"),
        )
        .arg(
            Arg::with_name("code")
                .long("code")
                .help("output commented code"),
        )
        .arg(
            Arg::with_name("INPUT")
                .required(false)
                .index(1)
                .help("file to read"),
        )
        .get_matches();

    let reverse = matches.is_present("reverse");
    let numbers = !matches.is_present("no-addresses");
    let code = matches.is_present("code");
    let width: usize = matches.value_of("width").expect("default").parse()?;

    let stdin = io::stdin();
    let input: Box<BufRead> = match matches.value_of("INPUT") {
        Some(path) => Box::new(io::BufReader::new(fs::File::open(path)?)),
        None => Box::new(stdin.lock()),
    };

    if reverse {
        undo(input, numbers)
    } else if code {
        encode_code(input, numbers, width)
    } else {
        encode_xxd(input, numbers, width)
    }
}

quick_main!(run);
