use rayon::prelude::*;
use std::fs::File;
use std::io::Write;
use std::io::BufRead;
use std::sync::{Arc, Mutex};
use std::error::Error;
use twobit::TwoBitFile;
use rayon::ThreadPoolBuilder;
use anyhow::Context;

#[cfg(test)]
mod tests;

mod constants;
mod resources;
mod functions;
mod prelude;

use crate::prelude::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let fasta_path = "query.fasta";
    let alignment_file_path = "chm13_to_hg38.txt";
    let chm13_2bit_path = "chm13.2bit";
    let hg38_2bit_path = "hg38.2bit";

    let query_sequence = read_sequence(fasta_path).await?;
    let alignment_mappings = parse_alignment_file(alignment_file_path).await?;

    let chm13_2bit = Arc::new(Mutex::new(TwoBitFile::open(chm13_2bit_path)?));
    let hg38_2bit = Arc::new(Mutex::new(TwoBitFile::open(hg38_2bit_path)?));

    let sequence_data = SequenceData {
        chm13_2bit,
        hg38_2bit,
    };

    let num_threads = num_cpus::get();
    println!("Using {} threads.", num_threads);

    ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()?;

    find_mappings(&query_sequence, &alignment_mappings, &sequence_data).await?;

    Ok(())
}