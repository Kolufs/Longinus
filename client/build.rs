use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::io::{self, BufRead};
use std::path::Path;

// There only other way to circunvent double-memory usage would be a sad proc-macro;
fn main() {
    println!("cargo:rerun-if-changed=./comptime/domainsquarter");

    let source_path = "./comptime/domains";
    let source_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(false)
        .open(source_path)
        .expect("Failed to open file");

    let output_path = Path::new("./src/comptime_prod").join("domains.rs");
    let mut output_file = OpenOptions::new()
        .read(true)
        .write(true)
        .truncate(true)
        .create(true)
        .open(output_path)
        .expect("Failed to open file");

    let lines: Vec<_> = io::BufReader::new(source_file)
        .lines()
        .map(|line| line.expect("Failed to read line"))
        .collect::<Vec<_>>();

    writeln!(
        &mut output_file,
        "pub const DOMAINS: &'static[&'static str] = &["
    )
    .unwrap();

    lines.into_iter().for_each(|line| {
        writeln!(&mut output_file, "    \"{}\",", line).unwrap();
    });

    writeln!(&mut output_file, "];").unwrap();
}
