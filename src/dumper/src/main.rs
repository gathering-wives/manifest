use std::process::Command;

use base64::prelude::*;
use clap::Parser;
use pelite::{
    FileMap,
    image::IMAGE_SCN_MEM_EXECUTE,
    pattern,
    pe::{Pe, PeFile},
};
use regex::Regex;
use tracing::{error, info};

mod types;
use types::*;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Parser)]
struct Args {
    /// AES Dumpster path
    #[arg(short, long)]
    aes_dumpster_path: String,

    /// Game binary path
    #[arg(short, long)]
    binary_path: String,

    /// Game version
    #[arg(short, long)]
    version: String,

    /// Output file path
    #[arg(short, long)]
    output_path: String,
}

fn dump_pak_key(args: &Args) -> Result<Vec<PakKey>> {
    info!("Dumping PAK AES keys...");
    let mut result = vec![];

    let output = Command::new(&args.aes_dumpster_path)
        .arg(&args.binary_path)
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    let re = Regex::new(r"Key:\s*(?P<key>[A-Fa-f0-9x]+)\s*\|\s*Key Entropy:\s*(?P<entropy>[\d.]+)")
        .unwrap();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() || !line.starts_with("Key:") {
            continue;
        }

        if let Some(captures) = re.captures(line) {
            let key = &captures["key"];
            let entropy = &captures["entropy"];

            result.push(PakKey {
                key: key.to_string(),
                entropy: entropy.parse().unwrap(),
            });
        } else {
            error!("Failed to parse line: {line}");
        }
    }

    Ok(result)
}

fn dump_net_key(args: &Args) -> Result<Option<NetKey>> {
    info!("Dumping NET encryption key...");

    let file = FileMap::open(&args.binary_path)?;
    let pe = PeFile::from_bytes(&file)?;
    let scanner = pe.scanner();

    for section in pe.section_headers() {
        if (section.Characteristics & IMAGE_SCN_MEM_EXECUTE) != 0 {
            let range = section.file_range();
            let mut save = [0; 8];
            if scanner.finds(
                pattern!("C7 44 24 38 10 00 00 00 48 8D 05 ? ? ? ? 48 F7 D1"),
                range,
                &mut save,
            ) {
                let lea = save[0] + 8;
                let rva = lea
                    .overflowing_add((pe.derva_copy::<i32>(lea + 3)? + 3 + 4) as u32)
                    .0;

                let key_slice = pe.slice(rva - 32, 32, 1)?;
                let iv_slice = pe.slice(rva, 16, 1)?;

                return Ok(Some(NetKey {
                    key: BASE64_STANDARD.encode(&key_slice[..32]),
                    iv: BASE64_STANDARD.encode(&iv_slice[..16]),
                }));
            }
        }
    }

    Ok(None)
}

fn main() -> Result<()> {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let output = OutputFile {
        version: args.version.clone(),
        pak_keys: dump_pak_key(&args)?,
        net_keys: dump_net_key(&args)?,
    };

    info!("Writing output...");
    let output = serde_json::to_string_pretty(&output)?;
    std::fs::write(args.output_path.clone(), output)?;

    info!("Done!");
    Ok(())
}
