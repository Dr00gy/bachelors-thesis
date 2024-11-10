use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use anyhow::Context;
use rayon::iter::IntoParallelRefIterator;
use twobit::TwoBitPhysicalFile;
use crate::prelude::{SequenceData, CHUNK_SIZE};
use rayon::prelude::*;
// use tokio version instead
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};

pub async fn read_sequence(fasta_path: &str) -> anyhow::Result<String> {
    let file = File::open(fasta_path)
        .await
        .context(format!("Failed to open FASTA file: {}", fasta_path))?;

    let reader = BufReader::new(file);
    let mut seq = String::new();
    let mut in_sequence = false;
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        if line.starts_with('>') {
            in_sequence = true; // Skip header
        } else if in_sequence {
            seq.push_str(&line.trim());
        }
    }

    Ok(seq)
}

pub async fn parse_alignment_file(alignment_file_path: &str) -> anyhow::Result<Vec<(String, i64, i64, String, i64, i64)>> {
    let file = File::open(alignment_file_path)
        .await
        .context(format!("Failed to open alignment file: {}", alignment_file_path))?;
    let reader = BufReader::new(file);
    let mut mappings = Vec::new();
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.len() >= 8 {
            let chr1 = parts[0].to_string();
            let start1: anyhow::Result<i64, _> = parts[1].parse();
            let end1: anyhow::Result<i64, _> = parts[2].parse();
            let chr2 = parts[4].to_string();
            let start2: anyhow::Result<i64, _> = parts[5].parse();
            let end2: anyhow::Result<i64, _> = parts[6].parse();

            if let (Ok(start1), Ok(end1), Ok(start2), Ok(end2)) = (start1, end1, start2, end2) {
                mappings.push((chr1, start1, end1, chr2, start2, end2));
            } else {
                eprintln!("Skipping invalid line: {}", line);
            }
        } else {
            eprintln!("Skipping malformed line: {}", line);
        }
    }

    Ok(mappings)
}

pub fn extract_sequence_2bit_in_chunks(
    genome: &Arc<Mutex<TwoBitPhysicalFile>>, // One thread can read
    chr: &str,
    start: usize,
    end: usize,
) -> anyhow::Result<String> {
    // Use map_err to convert PoisonError to an anyhow error
    let mut genome_lock = genome
        .lock()
        .map_err(|e| anyhow::anyhow!("Failed to lock genome file for reading: {}", e))?;;


    let sequence_length = end - start;
    let mut sequence = String::with_capacity(sequence_length); // For now
    let mut chunk_start = start;

    while chunk_start < end {
        let chunk_end = (chunk_start + CHUNK_SIZE).min(end);
        match genome_lock
            .read_sequence(chr, chunk_start..chunk_end)
            .context("Failed to read sequence chunk")
        {
            Ok(chunk) => sequence.push_str(&chunk),
            Err(e) => return Err(e),
        }
        chunk_start = chunk_end;
    }

    Ok(sequence)
}

pub async fn find_mappings(
    query_seq: &str,
    mappings: &[(String, i64, i64, String, i64, i64)],
    sequence_data: &SequenceData,
) -> anyhow::Result<bool> {
    let count = Arc::new(AtomicUsize::new(0)); // Thread safe
    let match_found = Arc::new(AtomicBool::new(false));

    mappings.par_iter().for_each(|mapping| { // Par here
        let match_found = Arc::clone(&match_found);
        let count = Arc::clone(&count);
        let query_seq = query_seq.to_string();
        let sequence_data = sequence_data.clone();

        let chm13_seq_result = extract_sequence_2bit_in_chunks(
            &sequence_data.chm13_2bit,
            &mapping.0,
            mapping.1 as usize,
            mapping.2 as usize
        )
            .context(format!(
                "Error extracting CHM13 sequence for {}:{}-{}",
                mapping.0, mapping.1, mapping.2
            ));

        let hg38_seq_result = extract_sequence_2bit_in_chunks(
            &sequence_data.hg38_2bit,
            &mapping.3,
            mapping.4 as usize,
            mapping.5 as usize
        )
            .context(format!(
                "Error extracting HG38 sequence for {}:{}-{}",
                mapping.3, mapping.4, mapping.5
            ));

        match (chm13_seq_result, hg38_seq_result) {
            (Ok(chm13_seq), Ok(hg38_seq)) => {
                if chm13_seq.contains(&query_seq) || hg38_seq.contains(&query_seq) {
                    match_found.store(true, Ordering::SeqCst);
                    let current_count = count.fetch_add(1, Ordering::SeqCst);
                    if current_count < 3 {
                        println!("Match found:");
                        println!(
                            "CHM13: {}:{}-{} -> HG38: {}:{}-{}",
                            mapping.0, mapping.1, mapping.2, mapping.3, mapping.4, mapping.5
                        );
                        println!("CHM13 Seq: {}", &chm13_seq);
                        println!("HG38 Seq: {}", &hg38_seq);
                    }
                }
            }
            (Err(e1), Ok(_)) => {
                eprintln!("{}", e1);
            }
            (Ok(_), Err(e2)) => {
                eprintln!("{}", e2);
            }
            (Err(e1), Err(e2)) => {
                eprintln!("Both sequence extractions failed: {} | {}", e1, e2);
            }
        }
    });

    Ok(match_found.load(Ordering::SeqCst))
}