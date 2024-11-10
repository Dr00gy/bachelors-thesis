use rayon::prelude::*;
use std::fs::File;
use std::io::{Write};
use std::io::{BufReader, BufRead};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering, AtomicBool};
use std::error::Error;
use twobit::{TwoBitFile, TwoBitPhysicalFile};
use rayon::ThreadPoolBuilder;
use anyhow::{Context, Result};

const CHUNK_SIZE: usize = 100000; // 100 000 bases at a time (memory management)

#[derive(Clone)]
struct SequenceData {
    chm13_2bit: Arc<Mutex<TwoBitPhysicalFile>>,
    hg38_2bit: Arc<Mutex<TwoBitPhysicalFile>>,
}

fn read_sequence(fasta_path: &str) -> Result<String> {
    let file = File::open(fasta_path)
        .context(format!("Failed to open FASTA file: {}", fasta_path))?;
    let reader = BufReader::new(file);
    let mut seq = String::new();
    let mut in_sequence = false;

    for line in reader.lines() { // Note: concats EVERYTHING, query is small atm
        let line = line.context("Error reading line from FASTA file")?;
        if line.starts_with('>') {
            in_sequence = true; // Skip header
        } else if in_sequence {
            seq.push_str(&line.trim());
        }
    }

    Ok(seq)
}

fn parse_alignment_file(alignment_file_path: &str) -> Result<Vec<(String, i64, i64, String, i64, i64)>> {
    let file = File::open(alignment_file_path)
        .context(format!("Failed to open alignment file: {}", alignment_file_path))?;
    let reader = BufReader::new(file);
    let mut mappings = Vec::new();

    for line in reader.lines() {
        let line = line.context("Error reading line from alignment file")?;
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.len() >= 8 {
            let chr1 = parts[0].to_string();
            let start1: Result<i64, _> = parts[1].parse();
            let end1: Result<i64, _> = parts[2].parse();
            let chr2 = parts[4].to_string();
            let start2: Result<i64, _> = parts[5].parse();
            let end2: Result<i64, _> = parts[6].parse();

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

fn extract_sequence_2bit_in_chunks(
    genome: &Arc<Mutex<TwoBitPhysicalFile>>, // One thread can read
    chr: &str,
    start: usize,
    end: usize,
) -> anyhow::Result<String> {
    // Use map_err to convert PoisonError to an anyhow error
    let mut genome_lock = genome
        .lock()
        .map_err(|e| anyhow::anyhow!("Failed to lock genome file for reading: {}", e))?;

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

fn find_mappings(
    query_seq: &str,
    mappings: &[(String, i64, i64, String, i64, i64)],
    sequence_data: &SequenceData,
) -> Result<bool> {
    let count = Arc::new(AtomicUsize::new(0)); // THread safe
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

fn main() -> anyhow::Result<()> {
    let fasta_path = "query.fasta";
    let alignment_file_path = "chm13_to_hg38.txt";
    let chm13_2bit_path = "chm13.2bit";
    let hg38_2bit_path = "hg38.2bit";

    let query_sequence = read_sequence(fasta_path)?;
    let alignment_mappings = parse_alignment_file(alignment_file_path)?;

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

    find_mappings(&query_sequence, &alignment_mappings, &sequence_data)?;

    Ok(())
}


// Test module
#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create a temp file for testing
    fn create_test_fasta(contents: &str) -> String {
        let temp_path = "test.fasta";
        let mut file = File::create(temp_path).unwrap();
        file.write_all(contents.as_bytes()).unwrap();

        temp_path.to_string()
    }

    #[test]
    fn test_read_sequence() {
        let fasta_data = ">seq1\nATGC\n>seq2\nCGTA\n";
        let fasta_path = create_test_fasta(fasta_data);

        let result = read_sequence(&fasta_path).unwrap();
        assert_eq!(result, "ATGCCGTA");
    }

    #[test]
    fn test_parse_alignment_file() {
        let alignment_data = r#"
        chr1 5618 12693 + chr1 260873 267915 +
        chr1 12693 1556790 + chr1 586069 2121513 +
    "#;

        let alignment_data = alignment_data.trim();

        let alignment_data = alignment_data
            .lines()
            .map(|line| line.trim())
            .collect::<Vec<&str>>()
            .join("\n");

        let alignment_path = "test1.txt";
        let mut file = File::create(alignment_path).unwrap();
        file.write_all(alignment_data.as_bytes()).unwrap();

        let result = parse_alignment_file(alignment_path).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], ("chr1".to_string(), 5618, 12693, "chr1".to_string(), 260873, 267915));
        assert_eq!(result[1], ("chr1".to_string(), 12693, 1556790, "chr1".to_string(), 586069, 2121513));
    }

    #[test]
    fn test_extract_sequence_2bit_in_chunks() {
        let chm13_2bit_path = "chm13.2bit";
        let hg38_2bit_path = "hg38.2bit";

        let chm13_2bit = Arc::new(Mutex::new(TwoBitFile::open(chm13_2bit_path).unwrap()));
        let hg38_2bit = Arc::new(Mutex::new(TwoBitFile::open(hg38_2bit_path).unwrap()));

        let sequence_data = SequenceData {
            chm13_2bit,
            hg38_2bit,
        };

        let chr = "chr1";
        let start = 0;
        let end = 1000;

        let result = extract_sequence_2bit_in_chunks(&sequence_data.chm13_2bit, chr, start, end);

        assert!(result.is_ok());
        let seq = result.unwrap();

        assert!(seq.len() > 0);
    }

    #[test]
    fn test_find_mappings() {
        let fasta_path = "query.fasta";
        let query_sequence = read_sequence(&fasta_path).unwrap();

        let chm13_2bit_path = "chm13.2bit";
        let hg38_2bit_path = "hg38.2bit";

        let alignment_data = r#"
    chr1 0 1000 + chr1 0 1000 +
    "#;

        let alignment_data = alignment_data.trim();

        let alignment_path = "test2.txt";
        let mut file = File::create(alignment_path).unwrap();
        file.write_all(alignment_data.as_bytes()).unwrap();
        let mappings = parse_alignment_file(alignment_path).unwrap();

        let chm13_2bit = Arc::new(Mutex::new(TwoBitFile::open(chm13_2bit_path).unwrap()));
        let hg38_2bit = Arc::new(Mutex::new(TwoBitFile::open(hg38_2bit_path).unwrap()));

        let sequence_data = SequenceData {
            chm13_2bit,
            hg38_2bit,
        };

        let match_found = find_mappings(&query_sequence, &mappings, &sequence_data).unwrap();
        assert!(match_found, "No match was found!");
    }
}

//TODO: use timer / tracer, figure out chunk size, figure out thread control (number) and tokio for I/O