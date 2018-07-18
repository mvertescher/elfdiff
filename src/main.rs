#[macro_use]
extern crate clap;
extern crate difference;
extern crate hexdump;
extern crate term;
extern crate xmas_elf;

use std::io;
use std::io::prelude::*;
use std::fs::File;

use clap::Arg;
use difference::{Difference, Changeset};
use xmas_elf::ElfFile;
use xmas_elf::sections::{SectionHeader, ShType, SectionData};

fn get_data(path: &str) -> io::Result<Vec<u8>> {
    let mut f = File::open(path)?;
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer)?;
    Ok(buffer)
}

fn get_section_with_name<'a>(name: &str, elf:  &'a ElfFile) -> Option<SectionHeader<'a>> {
    for section in elf.section_iter() {
        let n = match section.get_name(elf) {
            Ok(n) => n,
            Err(_) => continue,
        };

        if name != n {
            continue;
        }

        return Some(section);
    };

    None
}

fn diff_lines(lines_one: &str, lines_two: &str) {
    let mut t = term::stdout().unwrap();

    let Changeset { diffs, .. } = Changeset::new(lines_one, lines_two, "\n");
    for i in 0..diffs.len() {
        match diffs[i] {
            Difference::Add(ref x) => {
                t.fg(term::color::GREEN).unwrap();
                writeln!(t, "+{}", x).unwrap();
            }
            Difference::Rem(ref x) => {
                t.fg(term::color::RED).unwrap();
                writeln!(t, "-{}", x).unwrap();
            }
            _ => (),
        }
    }

    t.reset().unwrap();
    t.flush().unwrap();
}

fn diff_section_data(data_one: &[u8], data_two: &[u8]) {
    let hex_one = hexdump::hexdump_iter(data_one);
    let mut hex_two = hexdump::hexdump_iter(data_two);

    for line in hex_one {
        let l = match hex_two.next() {
            Some(l) => l,
            None => continue,
        };

        diff_lines(&line.to_string(), &l.to_string());
    }

    for line in hex_two {
        diff_lines("", &line.to_string());
    }
}

fn diff_sections(header_one: SectionHeader, elf_one: &ElfFile,
                 header_two: SectionHeader, elf_two: &ElfFile) {
    let data_one = match header_one.get_data(elf_one) {
        Ok(data) => data,
        Err(_) => return,
    };

    let data_two = match header_two.get_data(elf_two) {
        Ok(data) => data,
        Err(_) => return,
    };

    let data_one = match data_one {
        SectionData::Undefined(a) => a,
        _ => return,
    };

    let data_two = match data_two {
        SectionData::Undefined(a) => a,
        _ => return,
    };

    if data_one == data_two {
        return;
    }

    println!("Section {} differs", header_one.get_name(&elf_one).unwrap());
    diff_section_data(data_one, data_two);
}

fn main() {
    let matches = app_from_crate!()
        .arg(Arg::with_name("ELF-1")
             .help("First ELF")
             .required(true)
             .index(1))
        .arg(Arg::with_name("ELF-2")
             .help("Second ELF")
             .required(true)
             .index(2))
        .get_matches();

    let path_one = matches.value_of("ELF-1").unwrap();
    let path_two = matches.value_of("ELF-2").unwrap();

    let file_one = get_data(path_one).unwrap();
    let elf_one = ElfFile::new(&file_one).unwrap();

    let file_two = get_data(path_two).unwrap();
    let elf_two = ElfFile::new(&file_two).unwrap();

    // TODO: Diff section headers?

    for section_one in elf_one.section_iter() {
        let name_one = match section_one.get_name(&elf_one) {
            Ok(name) => name,
            Err(_) => continue,
        };

        let section_two = match get_section_with_name(name_one, &elf_two) {
            Some(s) => s,
            None => continue,
        };

        if section_one.get_type().unwrap() != section_one.get_type().unwrap() {
            println!("Different section types for '{}'", name_one);
            continue;
        }

        if section_one.get_type().unwrap() == ShType::Note {
            println!("Skipping over Note section type for '{}'", name_one);
            continue;
        }

        diff_sections(section_one, &elf_one, section_two, &elf_two);
    }
}
