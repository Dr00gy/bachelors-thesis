use serde::{Deserialize, Serialize};
use dashmap::DashMap;
use crossbeam::channel;
use crossbeam::queue::SegQueue;
use rayon::prelude::*;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XmapRecord {
    pub xmap_entry_id: u32,
    pub qry_contig_id: u32,
    pub ref_contig_id: u8,  // chromosome 1-23
    pub qry_start_pos: f64,
    pub qry_end_pos: f64,
    pub ref_start_pos: f64,
    pub ref_end_pos: f64,
    pub orientation: char,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XmapMatch {
    pub qry_contig_id: u32,
    pub file_indices: Box<[usize]>,
    pub records: Box<[MatchedRecord]>,
}

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
}

pub fn parse_xmap_file(content: &str) -> Result<Arc<DashMap<u32, Arc<XmapRecord>>>, String> {
    let records = DashMap::new();

    content
        .par_lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .try_for_each(|line| -> Result<(), String> {
            let fields: Box<[&str]> = line.split('\t').collect();
            if fields.len() < 9 {
                return Ok(());
            }

            let record = Arc::new(XmapRecord {
                xmap_entry_id: fields[0].parse().map_err(|e| format!("Parse XmapEntryID: {}", e))?,
                qry_contig_id: fields[1].parse().map_err(|e| format!("Parse QryContigID: {}", e))?,
                ref_contig_id: fields[2].parse().map_err(|e| format!("Parse RefContigID: {}", e))?,
                qry_start_pos: fields[3].parse().map_err(|e| format!("Parse QryStartPos: {}", e))?,
                qry_end_pos: fields[4].parse().map_err(|e| format!("Parse QryEndPos: {}", e))?,
                ref_start_pos: fields[5].parse().map_err(|e| format!("Parse RefStartPos: {}", e))?,
                ref_end_pos: fields[6].parse().map_err(|e| format!("Parse RefEndPos: {}", e))?,
                orientation: fields[7].chars().next().unwrap_or('+'),
                confidence: fields[8].parse().map_err(|e| format!("Parse Confidence: {}", e))?,
            });

            records.insert(record.xmap_entry_id, record);
            Ok(())
        })?;

    Ok(Arc::new(records))
}

/// build index: QryContigID -> DashMap of XmapRecords
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

pub struct XmapFileSet {
    pub files: Box<[Arc<DashMap<u32, Arc<XmapRecord>>>]>,
    pub indices: Box<[Arc<DashMap<u32, Arc<DashMap<u32, Arc<XmapRecord>>>>>]>,
}

impl XmapFileSet {
    pub fn new(files: Box<[Arc<DashMap<u32, Arc<XmapRecord>>>]>) -> Self {
        let indices: Box<[_]> = files
            .iter()
            .map(|f| build_index(Arc::clone(f)))
            .collect();

        Self { files, indices }
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }
}

/// compares the first file rn against all others by QryContigID
pub fn stream_matches_multi(
    fileset: Arc<XmapFileSet>,
) -> channel::Receiver<XmapMatch> {
    let (tx, rx) = channel::unbounded();

    if fileset.len() < 2 {
        return rx;
    }

    let queue: Arc<SegQueue<Arc<Box<[Arc<XmapRecord>]>>>> = Arc::new(SegQueue::new());
    let chunk_size = 1000;
    let mut temp_chunk = Vec::with_capacity(chunk_size);
    
    for entry in fileset.files[0].iter() {
        temp_chunk.push(Arc::clone(entry.value()));
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
            let fileset = Arc::clone(&fileset);

            s.spawn(move |_| {
                while let Some(chunk) = queue.pop() {
                    for ref_record in chunk.iter() {
                        let mut matched_indices = Vec::new();
                        let mut matched_records = Vec::new();

                        // add reference record (from file 0)
                        matched_indices.push(0);
                        matched_records.push(MatchedRecord {
                            file_index: 0,
                            ref_contig_id: ref_record.ref_contig_id,
                            qry_start_pos: ref_record.qry_start_pos,
                            qry_end_pos: ref_record.qry_end_pos,
                            ref_start_pos: ref_record.ref_start_pos,
                            ref_end_pos: ref_record.ref_end_pos,
                            orientation: ref_record.orientation,
                            confidence: ref_record.confidence,
                        });
                        
                        for (file_idx, index) in fileset.indices.iter().enumerate().skip(1) {
                            if let Some(candidates) = index.get(&ref_record.qry_contig_id) {
                                for candidate_entry in candidates.iter() {
                                    let candidate = candidate_entry.value();

                                    matched_indices.push(file_idx);
                                    matched_records.push(MatchedRecord {
                                        file_index: file_idx,
                                        ref_contig_id: candidate.ref_contig_id,
                                        qry_start_pos: candidate.qry_start_pos,
                                        qry_end_pos: candidate.qry_end_pos,
                                        ref_start_pos: candidate.ref_start_pos,
                                        ref_end_pos: candidate.ref_end_pos,
                                        orientation: candidate.orientation,
                                        confidence: candidate.confidence,
                                    });
                                }
                            }
                        }

                        // only emit if we found matches in at least one other file
                        if matched_indices.len() > 1 {
                            let match_data = XmapMatch {
                                qry_contig_id: ref_record.qry_contig_id,
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

    rx
}

/// cache manager
pub struct XmapCache {
    pub parsed_files: Arc<DashMap<u64, Arc<DashMap<u32, Arc<XmapRecord>>>>>,
    pub indices: Arc<DashMap<u64, Arc<DashMap<u32, Arc<DashMap<u32, Arc<XmapRecord>>>>>>>,
    pub match_cache: Arc<DashMap<Box<[u64]>, Arc<DashMap<u64, Arc<XmapMatch>>>>>,
}

impl XmapCache {
    pub fn new() -> Self {
        Self {
            parsed_files: Arc::new(DashMap::new()),
            indices: Arc::new(DashMap::new()),
            match_cache: Arc::new(DashMap::new()),
        }
    }

    pub fn get_or_parse(&self, hash: u64, content: &str) -> Result<Arc<DashMap<u32, Arc<XmapRecord>>>, String> {
        if let Some(cached) = self.parsed_files.get(&hash) {
            return Ok(Arc::clone(cached.value()));
        }

        let records = parse_xmap_file(content)?;
        self.parsed_files.insert(hash, Arc::clone(&records));
        Ok(records)
    }

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
#h XmapEntryID	QryContigID	RefContigID	QryStartPos	QryEndPos	RefStartPos	RefEndPos	Orientation	Confidence	HitEnum
1	4881976	1	103833.0	2059.6	4561.0	111073.0	-	15.11	1M1D4M1D1M1D9M
2	1269991	1	107882.8	229.3	4561.0	117599.0	-	16.87	1M1D6M1D7M1I3M
3	4881976	2	10214.4	118509.6	4561.0	117599.0	+	17.81	1M1D6M1D10M"#
    }

    #[test]
    fn test_parse_xmap_file() {
        let records = parse_xmap_file(sample_xmap_content()).unwrap();
        assert_eq!(records.len(), 3);

        let rec1 = records.get(&1).unwrap();
        assert_eq!(rec1.qry_contig_id, 4881976);
        assert_eq!(rec1.ref_contig_id, 1);
        assert_eq!(rec1.orientation, '-');
        assert_eq!(rec1.confidence, 15.11);
    }

    #[test]
    fn test_build_index() {
        let records = parse_xmap_file(sample_xmap_content()).unwrap();
        let index = build_index(records);
        assert_eq!(index.len(), 2);

        let qry_map = index.get(&4881976).unwrap();
        assert_eq!(qry_map.len(), 2);
    }

    #[test]
    fn test_stream_matches_multi_two_files() {
        let file1_content = r#"#h XmapEntryID	QryContigID	RefContigID	QryStartPos	QryEndPos	RefStartPos	RefEndPos	Orientation	Confidence	HitEnum
1	100	1	1000.0	2000.0	5000.0	6000.0	+	15.0	1M
2	200	2	3000.0	4000.0	7000.0	8000.0	-	14.5	1M"#;

        let file2_content = r#"#h XmapEntryID	QryContigID	RefContigID	QryStartPos	QryEndPos	RefStartPos	RefEndPos	Orientation	Confidence	HitEnum
10	100	3	1500.0	2500.0	9000.0	10000.0	+	16.0	1M
11	200	4	3500.0	4500.0	11000.0	12000.0	-	15.5	1M"#;

        let file1_records = parse_xmap_file(file1_content).unwrap();
        let file2_records = parse_xmap_file(file2_content).unwrap();

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
        let file1_content = r#"#h XmapEntryID	QryContigID	RefContigID	QryStartPos	QryEndPos	RefStartPos	RefEndPos	Orientation	Confidence	HitEnum
1	100	1	1000.0	2000.0	5000.0	6000.0	+	15.0	1M"#;

        let file2_content = r#"#h XmapEntryID	QryContigID	RefContigID	QryStartPos	QryEndPos	RefStartPos	RefEndPos	Orientation	Confidence	HitEnum
10	100	2	1500.0	2500.0	7000.0	8000.0	+	16.0	1M"#;

        let file3_content = r#"#h XmapEntryID	QryContigID	RefContigID	QryStartPos	QryEndPos	RefStartPos	RefEndPos	Orientation	Confidence	HitEnum
20	100	3	2000.0	3000.0	9000.0	10000.0	-	17.0	1M"#;

        let file1_records = parse_xmap_file(file1_content).unwrap();
        let file2_records = parse_xmap_file(file2_content).unwrap();
        let file3_records = parse_xmap_file(file3_content).unwrap();

        let fileset = Arc::new(XmapFileSet::new(
            vec![file1_records, file2_records, file3_records].into_boxed_slice()
        ));

        let rx = stream_matches_multi(fileset);

        let mut match_count = 0;
        while let Ok(match_data) = rx.recv() {
            match_count += 1;
            assert_eq!(match_data.qry_contig_id, 100);
            assert_eq!(match_data.file_indices.len(), 3); // All 3 files match
            assert_eq!(match_data.records.len(), 3);
        }

        assert_eq!(match_count, 1);
    }
}