use serde::{Deserialize, Serialize};
use dashmap::DashMap;
use crossbeam::channel;
use crossbeam::queue::SegQueue;
use rayon::prelude::*;
use std::sync::Arc;

/// Represents a single XMAP record from parsed files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XmapRecord {
    pub xmap_entry_id: u32,
    pub qry_contig_id: u32,
    pub ref_contig_id: u8,
    pub qry_start_pos: f64,
    pub qry_end_pos: f64,
    pub ref_start_pos: f64,
    pub ref_end_pos: f64,
    pub orientation: char,
    pub confidence: f64,
    pub ref_len: f64,
}

/// Represents chromosome information with length
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChromosomeInfo {
    pub ref_contig_id: u8,
    pub ref_len: f64,
}

/// Represents a match between multiple XMAP records
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XmapMatch {
    pub qry_contig_id: u32,
    pub file_indices: Box<[usize]>,
    pub records: Box<[MatchedRecord]>,
}

/// Represents an individual record within a match
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedRecord {
    pub file_index: usize,
    pub ref_contig_id: u8,
    pub qry_start_pos: f64,
    pub qry_end_pos: f64,
    pub ref_start_pos: f64,
    pub ref_end_pos: f64,
    pub orientation: char,
    pub confidence: f64,
    pub ref_len: f64,
}

/// Parses XMAP file content into structured records
///
/// # Arguments
/// * `content` - Raw XMAP file content as string
///
/// # Returns
/// * `Result<Arc<DashMap<u32, Arc<XmapRecord>>>, String>` - Parsed records or error
///
/// # Format
/// Expects tab-separated values with specific column ordering
pub fn parse_xmap_file(content: &str) -> Result<(Arc<DashMap<u32, Arc<XmapRecord>>>, Arc<DashMap<u8, f64>>), String> {
    let records = DashMap::new();
    let chromosome_lengths = DashMap::new();

    content
        .par_lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .try_for_each(|line| -> Result<(), String> {
            let fields: Box<[&str]> = line.split('\t').collect();
            if fields.len() < 12 {
                return Ok(());
            }

            let ref_contig_id: u8 = fields[2].parse().map_err(|e| format!("Parse RefContigID: {}", e))?;
            let ref_len: f64 = fields[11].parse().map_err(|e| format!("Parse RefLen: {}", e))?;

            chromosome_lengths.insert(ref_contig_id, ref_len);

            let record = Arc::new(XmapRecord {
                xmap_entry_id: fields[0].parse().map_err(|e| format!("Parse XmapEntryID: {}", e))?,
                qry_contig_id: fields[1].parse().map_err(|e| format!("Parse QryContigID: {}", e))?,
                ref_contig_id,
                qry_start_pos: fields[3].parse().map_err(|e| format!("Parse QryStartPos: {}", e))?,
                qry_end_pos: fields[4].parse().map_err(|e| format!("Parse QryEndPos: {}", e))?,
                ref_start_pos: fields[5].parse().map_err(|e| format!("Parse RefStartPos: {}", e))?,
                ref_end_pos: fields[6].parse().map_err(|e| format!("Parse RefEndPos: {}", e))?,
                orientation: fields[7].chars().next().unwrap_or('+'),
                confidence: fields[8].parse().map_err(|e| format!("Parse Confidence: {}", e))?,
                ref_len,
            });

            records.insert(record.xmap_entry_id, Arc::clone(&record));
            Ok(())
        })?;

    Ok((Arc::new(records), Arc::new(chromosome_lengths)))
}

/// Builds index mapping query contig IDs to their records
///
/// # Arguments
/// * `records` - Parsed XMAP records
///
/// # Returns
/// * `Arc<DashMap<u32, Arc<DashMap<u32, Arc<XmapRecord>>>>>` - Indexed records
pub fn build_index(
    records: Arc<DashMap<u32, Arc<XmapRecord>>>
) -> Arc<DashMap<u32, Arc<DashMap<u32, Arc<XmapRecord>>>>> {
    let index = Arc::new(DashMap::new());

    for entry in records.iter() {
        let record = entry.value();
        let qry_id = record.qry_contig_id;

        let qry_map = index
            .entry(qry_id)
            .or_insert_with(|| Arc::new(DashMap::new()))
            .value()
            .clone();

        qry_map.insert(record.xmap_entry_id, Arc::clone(record));
    }

    index
}

/// Container for multiple XMAP files and their indices
pub struct XmapFileSet {
    pub files: Box<[Arc<DashMap<u32, Arc<XmapRecord>>>]>,
    pub indices: Box<[Arc<DashMap<u32, Arc<DashMap<u32, Arc<XmapRecord>>>>>]>,
}

impl XmapFileSet {
    /// Creates new XmapFileSet with built indices
    ///
    /// # Arguments
    /// * `files` - Collection of parsed XMAP files
    pub fn new(files: Box<[Arc<DashMap<u32, Arc<XmapRecord>>>]>) -> Self {
        let indices: Box<[_]> = files
            .iter()
            .map(|f| build_index(Arc::clone(f)))
            .collect();

        Self { files, indices }
    }

    /// Returns number of files in the set
    pub fn len(&self) -> usize {
        self.files.len()
    }
}

/// Streams matches between all file pairs in the fileset
///
/// # Arguments
/// * `fileset` - Set of XMAP files to compare
///
/// # Returns
/// * `channel::Receiver<XmapMatch>` - Channel receiver for match results
///
/// # Process
/// * Generates all file pair combinations
/// * Groups records by query contig ID
/// * Processes matches in parallel using rayon
/// * Streams results via channel
pub fn stream_matches_multi(
    fileset: Arc<XmapFileSet>,
) -> channel::Receiver<XmapMatch> {
    let (tx, rx) = channel::unbounded();

    if fileset.len() < 2 {
        return rx;
    }

    let mut file_pairs = Vec::new();
    for i in 0..fileset.len() {
        for j in (i + 1)..fileset.len() {
            file_pairs.push((i, j));
        }
    }

    for (file_i, file_j) in file_pairs {
        let qry_groups_i: Arc<DashMap<u32, Vec<Arc<XmapRecord>>>> = Arc::new(DashMap::new());
        let qry_groups_j: Arc<DashMap<u32, Vec<Arc<XmapRecord>>>> = Arc::new(DashMap::new());

        for entry in fileset.files[file_i].iter() {
            let record = entry.value();
            qry_groups_i
                .entry(record.qry_contig_id)
                .or_insert_with(Vec::new)
                .push(Arc::clone(record));
        }

        for entry in fileset.files[file_j].iter() {
            let record = entry.value();
            qry_groups_j
                .entry(record.qry_contig_id)
                .or_insert_with(Vec::new)
                .push(Arc::clone(record));
        }

        let queue: Arc<SegQueue<Arc<Box<[(u32, Vec<Arc<XmapRecord>>)]>>>> = Arc::new(SegQueue::new());
        let chunk_size = 100;
        let mut temp_chunk = Vec::with_capacity(chunk_size);

        for entry in qry_groups_i.iter() {
            let qry_id = *entry.key();
            let records = entry.value().clone();
            temp_chunk.push((qry_id, records));

            if temp_chunk.len() == chunk_size {
                queue.push(Arc::new(temp_chunk.into_boxed_slice()));
                temp_chunk = Vec::with_capacity(chunk_size);
            }
        }

        if !temp_chunk.is_empty() {
            queue.push(Arc::new(temp_chunk.into_boxed_slice()));
        }

        let n_threads = num_cpus::get();

        rayon::scope(|s| {
            for _ in 0..n_threads {
                let tx = tx.clone();
                let queue = Arc::clone(&queue);
                let qry_groups_j = Arc::clone(&qry_groups_j);

                s.spawn(move |_| {
                    while let Some(chunk) = queue.pop() {
                        for (qry_id, records_i) in chunk.iter() {
                            if let Some(records_j) = qry_groups_j.get(qry_id) {
                                let mut matched_indices = Vec::new();
                                let mut matched_records = Vec::new();

                                for record_i in records_i {
                                    matched_indices.push(file_i);
                                    matched_records.push(MatchedRecord {
                                        file_index: file_i,
                                        ref_contig_id: record_i.ref_contig_id,
                                        qry_start_pos: record_i.qry_start_pos,
                                        qry_end_pos: record_i.qry_end_pos,
                                        ref_start_pos: record_i.ref_start_pos,
                                        ref_end_pos: record_i.ref_end_pos,
                                        orientation: record_i.orientation,
                                        confidence: record_i.confidence,
                                        ref_len: record_i.ref_len,
                                    });
                                }

                                for record_j in records_j.value() {
                                    matched_indices.push(file_j);
                                    matched_records.push(MatchedRecord {
                                        file_index: file_j,
                                        ref_contig_id: record_j.ref_contig_id,
                                        qry_start_pos: record_j.qry_start_pos,
                                        qry_end_pos: record_j.qry_end_pos,
                                        ref_start_pos: record_j.ref_start_pos,
                                        ref_end_pos: record_j.ref_end_pos,
                                        orientation: record_j.orientation,
                                        confidence: record_j.confidence,
                                        ref_len: record_j.ref_len,
                                    });
                                }

                                let match_data = XmapMatch {
                                    qry_contig_id: *qry_id,
                                    file_indices: matched_indices.into_boxed_slice(),
                                    records: matched_records.into_boxed_slice(),
                                };
                                let _ = tx.send(match_data);
                            }
                        }
                    }
                });
            }
        });
    }

    rx
}

/// Cache manager for XMAP parsing and matching results
pub struct XmapCache {
    pub parsed_files: Arc<DashMap<u64, Arc<DashMap<u32, Arc<XmapRecord>>>>>,
    pub chromosome_lengths: Arc<DashMap<u64, Arc<DashMap<u8, f64>>>>,
    pub indices: Arc<DashMap<u64, Arc<DashMap<u32, Arc<DashMap<u32, Arc<XmapRecord>>>>>>>,
    pub match_cache: Arc<DashMap<Box<[u64]>, Arc<DashMap<u64, Arc<XmapMatch>>>>>,
}

impl XmapCache {
    /// Creates new empty cache
    pub fn new() -> Self {
        Self {
            parsed_files: Arc::new(DashMap::new()),
            chromosome_lengths: Arc::new(DashMap::new()),
            indices: Arc::new(DashMap::new()),
            match_cache: Arc::new(DashMap::new()),
        }
    }

    /// Gets parsed records from cache or parses new content
    ///
    /// # Arguments
    /// * `hash` - Content hash for caching
    /// * `content` - XMAP file content to parse
    pub fn get_or_parse(&self, hash: u64, content: &str) -> Result<(Arc<DashMap<u32, Arc<XmapRecord>>>, Arc<DashMap<u8, f64>>), String> {
        if let Some(cached_records) = self.parsed_files.get(&hash) {
            if let Some(cached_lengths) = self.chromosome_lengths.get(&hash) {
                return Ok((Arc::clone(cached_records.value()), Arc::clone(cached_lengths.value())));
            }
        }

        let (records, chr_lengths) = parse_xmap_file(content)?;
        self.parsed_files.insert(hash, Arc::clone(&records));
        self.chromosome_lengths.insert(hash, Arc::clone(&chr_lengths));
        Ok((records, chr_lengths))
    }

    /// Gets index from cache or builds new index
    ///
    /// # Arguments
    /// * `hash` - Records hash for caching
    /// * `records` - Records to index
    pub fn get_or_build_index(
        &self,
        hash: u64,
        records: Arc<DashMap<u32, Arc<XmapRecord>>>,
    ) -> Arc<DashMap<u32, Arc<DashMap<u32, Arc<XmapRecord>>>>> {
        if let Some(cached) = self.indices.get(&hash) {
            return Arc::clone(cached.value());
        }

        let index = build_index(records);
        self.indices.insert(hash, Arc::clone(&index));
        index
    }

    /// Caches match result for future requests
    ///
    /// # Arguments
    /// * `key` - Cache key (file hashes)
    /// * `match_data` - Match result to cache
    pub fn cache_match(&self, key: Box<[u64]>, match_data: Arc<XmapMatch>) {
        let matches = self.match_cache
            .entry(key)
            .or_insert_with(|| Arc::new(DashMap::new()))
            .value()
            .clone();

        let match_id = (match_data.qry_contig_id as u64) << 32
            | (match_data.records[0].qry_start_pos as u64);
        matches.insert(match_id, match_data);
    }
}

/// Generates hash for file content caching
///
/// # Arguments
/// * `content` - String content to hash
///
/// # Returns
/// * `u64` - Content hash
pub fn hash_content(content: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_xmap_content() -> &'static str {
        r#"# hostname=imuno5p-compute
#h XmapEntryID	QryContigID	RefContigID	QryStartPos	QryEndPos	RefStartPos	RefEndPos	Orientation	Confidence	HitEnum	QryLen	RefLen
1	4881976	1	103833.0	2059.6	4561.0	111073.0	-	15.11	1M1D4M1D1M1D9M	103833.0	117599.0
2	1269991	1	107882.8	229.3	4561.0	117599.0	-	16.87	1M1D6M1D7M1I3M	107882.8	117599.0
3	4881976	2	10214.4	118509.6	4561.0	117599.0	+	17.81	1M1D6M1D10M	118509.6	117599.0"#
    }

    #[test]
    fn test_parse_xmap_file() {
        let (records, chr_lengths) = parse_xmap_file(sample_xmap_content()).unwrap();
        assert_eq!(records.len(), 3);
        assert_eq!(chr_lengths.len(), 2);

        let rec1 = records.get(&1).unwrap();
        assert_eq!(rec1.qry_contig_id, 4881976);
        assert_eq!(rec1.ref_contig_id, 1);
        assert_eq!(rec1.orientation, '-');
        assert_eq!(rec1.confidence, 15.11);
        assert_eq!(rec1.ref_len, 117599.0);
    }

    #[test]
    fn test_build_index() {
        let (records, _) = parse_xmap_file(sample_xmap_content()).unwrap();
        let index = build_index(records);
        assert_eq!(index.len(), 2);

        let qry_map = index.get(&4881976).unwrap();
        assert_eq!(qry_map.len(), 2);
    }

    #[test]
    fn test_stream_matches_multi_two_files() {
        let file1_content = r#"#h XmapEntryID	QryContigID	RefContigID	QryStartPos	QryEndPos	RefStartPos	RefEndPos	Orientation	Confidence	HitEnum	QryLen	RefLen
1	100	1	1000.0	2000.0	5000.0	6000.0	+	15.0	1M	2000.0	250000.0
2	200	2	3000.0	4000.0	7000.0	8000.0	-	14.5	1M	4000.0	250000.0"#;

        let file2_content = r#"#h XmapEntryID	QryContigID	RefContigID	QryStartPos	QryEndPos	RefStartPos	RefEndPos	Orientation	Confidence	HitEnum	QryLen	RefLen
10	100	3	1500.0	2500.0	9000.0	10000.0	+	16.0	1M	2500.0	250000.0
11	200	4	3500.0	4500.0	11000.0	12000.0	-	15.5	1M	4500.0	250000.0"#;

        let (file1_records, _) = parse_xmap_file(file1_content).unwrap();
        let (file2_records, _) = parse_xmap_file(file2_content).unwrap();

        let fileset = Arc::new(XmapFileSet::new(
            vec![file1_records, file2_records].into_boxed_slice()
        ));

        let rx = stream_matches_multi(fileset);

        let mut match_count = 0;
        while let Ok(match_data) = rx.recv() {
            match_count += 1;
            assert!(match_data.qry_contig_id == 100 || match_data.qry_contig_id == 200);
            assert_eq!(match_data.file_indices.len(), 2);
        }

        assert_eq!(match_count, 2);
    }

    #[test]
    fn test_stream_matches_multi_three_files() {
        let file1_content = r#"#h XmapEntryID	QryContigID	RefContigID	QryStartPos	QryEndPos	RefStartPos	RefEndPos	Orientation	Confidence	HitEnum	QryLen	RefLen
1	100	1	1000.0	2000.0	5000.0	6000.0	+	15.0	1M	2000.0	250000.0"#;

        let file2_content = r#"#h XmapEntryID	QryContigID	RefContigID	QryStartPos	QryEndPos	RefStartPos	RefEndPos	Orientation	Confidence	HitEnum	QryLen	RefLen
10	100	2	1500.0	2500.0	7000.0	8000.0	+	16.0	1M	2500.0	250000.0"#;

        let file3_content = r#"#h XmapEntryID	QryContigID	RefContigID	QryStartPos	QryEndPos	RefStartPos	RefEndPos	Orientation	Confidence	HitEnum	QryLen	RefLen
20	100	3	2000.0	3000.0	9000.0	10000.0	-	17.0	1M	3000.0	250000.0"#;

        let (file1_records, _) = parse_xmap_file(file1_content).unwrap();
        let (file2_records, _) = parse_xmap_file(file2_content).unwrap();
        let (file3_records, _) = parse_xmap_file(file3_content).unwrap();

        let fileset = Arc::new(XmapFileSet::new(
            vec![file1_records, file2_records, file3_records].into_boxed_slice()
        ));

        let rx = stream_matches_multi(fileset);

        let mut match_count = 0;
        while let Ok(match_data) = rx.recv() {
            match_count += 1;
            assert_eq!(match_data.qry_contig_id, 100);
            assert_eq!(match_data.file_indices.len(), 2);
            assert_eq!(match_data.records.len(), 2);
        }

        assert_eq!(match_count, 3);
    }
}