use super::*;

// Helper function to create a temp file for testing
fn create_test_fasta(contents: &str) -> String {
    let temp_path = "test.fasta";
    let mut file = File::create(temp_path).unwrap();
    file.write_all(contents.as_bytes()).unwrap();

    temp_path.to_string()
}

#[tokio::test]
async fn test_read_sequence() {
    let fasta_data = ">seq1\nATGC\n>seq2\nCGTA\n";
    let fasta_path = create_test_fasta(fasta_data);

    let result = read_sequence(&fasta_path).await.unwrap();
    assert_eq!(result, "ATGCCGTA");
}

#[tokio::test]
async fn test_parse_alignment_file() {
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

    let result = parse_alignment_file(alignment_path).await.unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0], ("chr1".to_string(), 5618, 12693, "chr1".to_string(), 260873, 267915));
    assert_eq!(result[1], ("chr1".to_string(), 12693, 1556790, "chr1".to_string(), 586069, 2121513));
}

#[tokio::test]
async fn test_extract_sequence_2bit_in_chunks() {
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

#[tokio::test]
async fn test_find_mappings() {
    let fasta_path = "query.fasta";
    let query_sequence = read_sequence(&fasta_path).await.unwrap();

    let chm13_2bit_path = "chm13.2bit";
    let hg38_2bit_path = "hg38.2bit";

    let alignment_data = r#"
    chr1 0 1000 + chr1 0 1000 +
    "#;

    let alignment_data = alignment_data.trim();

    let alignment_path = "test2.txt";
    let mut file = File::create(alignment_path).unwrap();
    file.write_all(alignment_data.as_bytes()).unwrap();
    let mappings = parse_alignment_file(alignment_path).await.unwrap();

    let chm13_2bit = Arc::new(Mutex::new(TwoBitFile::open(chm13_2bit_path).unwrap()));
    let hg38_2bit = Arc::new(Mutex::new(TwoBitFile::open(hg38_2bit_path).unwrap()));

    let sequence_data = SequenceData {
        chm13_2bit,
        hg38_2bit,
    };

    let match_found = find_mappings(&query_sequence, &mappings, &sequence_data).await.unwrap();
    assert!(match_found, "No match was found!");
}