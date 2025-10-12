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
    pub chm13_ref_contig_id: u8,
    pub chm13_qry_start_pos: f64,
    pub chm13_qry_end_pos: f64,
    pub chm13_ref_start_pos: f64,
    pub chm13_ref_end_pos: f64,
    pub chm13_orientation: char,
    pub hg38_ref_contig_id: u8,
    pub hg38_qry_start_pos: f64,
    pub hg38_qry_end_pos: f64,
    pub hg38_ref_start_pos: f64,
    pub hg38_ref_end_pos: f64,
    pub hg38_orientation: char,
    pub avg_confidence: f64,
}

/// return arc, avoid clones
pub fn parse_xmap_file(content: &str) -> Result<Arc<DashMap<u32, Arc<XmapRecord>>>, String> {
    let records = DashMap::new();

    content
        .par_lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .try_for_each(|line| -> Result<(), String> {
            let fields: Vec<&str> = line.split('\t').collect();
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

/// QryContigID -> DashMap of XmapRecords
/// avoids double lookup and double locks
pub fn build_index(
    records: Arc<DashMap<u32, Arc<XmapRecord>>>
) -> Arc<DashMap<u32, Arc<DashMap<u32, Arc<XmapRecord>>>>> {
    let index = Arc::new(DashMap::new());

    for entry in records.iter() {
        let record = entry.value();
        let qry_id = record.qry_contig_id;

        // insert DM only once per key
        let qry_map = index
            .entry(qry_id)
            .or_insert_with(|| Arc::new(DashMap::new()))
            .value()
            .clone();

        qry_map.insert(record.xmap_entry_id, Arc::clone(record));
    }

    index
}

/// fully parallel and chunked, no inter
pub fn stream_matches(
    chm13_records: Arc<DashMap<u32, Arc<XmapRecord>>>,
    hg38_index: Arc<DashMap<u32, Arc<DashMap<u32, Arc<XmapRecord>>>>>,
) -> channel::Receiver<XmapMatch> {
    use crossbeam::queue::SegQueue;
    let (tx, rx) = channel::unbounded();

    let queue: Arc<SegQueue<Arc<Vec<Arc<XmapRecord>>>>> = Arc::new(SegQueue::new());

    // chunk size for batching records
    let chunk_size = 1000;
    let mut temp_chunk = Vec::with_capacity(chunk_size);

    for entry in chm13_records.iter() {
        temp_chunk.push(Arc::clone(entry.value()));
        if temp_chunk.len() == chunk_size {
            queue.push(Arc::new(temp_chunk));
            temp_chunk = Vec::with_capacity(chunk_size);
        }
    }

    if !temp_chunk.is_empty() {
        queue.push(Arc::new(temp_chunk));
    }

    let n_threads = num_cpus::get();

    rayon::scope(|s| {
        for _ in 0..n_threads {
            let tx = tx.clone();
            let queue = Arc::clone(&queue);
            let hg38_index = Arc::clone(&hg38_index);

            s.spawn(move |_| {
                while let Some(chunk) = queue.pop() {
                    for chm13_rec in chunk.iter() {
                        if let Some(hg38_candidates) = hg38_index.get(&chm13_rec.qry_contig_id) {
                            for hg38_entry in hg38_candidates.iter() {
                                let hg38_rec = hg38_entry.value();
                                let match_data = XmapMatch {
                                    qry_contig_id: chm13_rec.qry_contig_id,
                                    chm13_ref_contig_id: chm13_rec.ref_contig_id,
                                    chm13_qry_start_pos: chm13_rec.qry_start_pos,
                                    chm13_qry_end_pos: chm13_rec.qry_end_pos,
                                    chm13_ref_start_pos: chm13_rec.ref_start_pos,
                                    chm13_ref_end_pos: chm13_rec.ref_end_pos,
                                    chm13_orientation: chm13_rec.orientation,
                                    hg38_ref_contig_id: hg38_rec.ref_contig_id,
                                    hg38_qry_start_pos: hg38_rec.qry_start_pos,
                                    hg38_qry_end_pos: hg38_rec.qry_end_pos,
                                    hg38_ref_start_pos: hg38_rec.ref_start_pos,
                                    hg38_ref_end_pos: hg38_rec.ref_end_pos,
                                    hg38_orientation: hg38_rec.orientation,
                                    avg_confidence: (chm13_rec.confidence + hg38_rec.confidence) / 2.0,
                                };
                                let _ = tx.send(match_data);
                            }
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
    pub match_cache: Arc<DashMap<(u64, u64), Arc<DashMap<u64, Arc<XmapMatch>>>>>,
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

    /// cache match with unique id generator for duplicates
    pub fn cache_match(&self, key: (u64, u64), match_data: Arc<XmapMatch>) {
        let matches = self.match_cache
            .entry(key)
            .or_insert_with(|| Arc::new(DashMap::new()))
            .value()
            .clone();

        // ensure uniqueness for same qry contig
        let match_id = (match_data.qry_contig_id as u64) << 32
            | (match_data.chm13_qry_start_pos as u64);
        matches.insert(match_id, match_data);
    }
}

/// simple hash function
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
        // 2 unique id and 2 records
        assert_eq!(index.len(), 2);
        
        let qry_map = index.get(&4881976).unwrap();
        assert_eq!(qry_map.len(), 2);
    }

    #[test]
    fn test_stream_matches() {
        let chm13_content = r#"#h XmapEntryID	QryContigID	RefContigID	QryStartPos	QryEndPos	RefStartPos	RefEndPos	Orientation	Confidence	HitEnum
1	100	1	1000.0	2000.0	5000.0	6000.0	+	15.0	1M
2	200	2	3000.0	4000.0	7000.0	8000.0	-	14.5	1M"#;

        let hg38_content = r#"#h XmapEntryID	QryContigID	RefContigID	QryStartPos	QryEndPos	RefStartPos	RefEndPos	Orientation	Confidence	HitEnum
10	100	3	1500.0	2500.0	9000.0	10000.0	+	16.0	1M
11	200	4	3500.0	4500.0	11000.0	12000.0	-	15.5	1M"#;

        let chm13_records = parse_xmap_file(chm13_content).unwrap();
        let hg38_records = parse_xmap_file(hg38_content).unwrap();
        let hg38_index = build_index(hg38_records);

        let rx = stream_matches(chm13_records, hg38_index);

        let mut match_count = 0;
        while let Ok(match_data) = rx.recv() {
            match_count += 1;

            // verify match structure
            assert!(match_data.qry_contig_id == 100 || match_data.qry_contig_id == 200);
            assert!(match_data.avg_confidence > 14.0);
        }

        assert_eq!(match_count, 2);
    }

    #[test]
    fn test_parallel_matching() {
        use std::time::Instant;

        // sim larger dataset
        let mut chm13_lines = vec!["#h XmapEntryID	QryContigID	RefContigID	QryStartPos	QryEndPos	RefStartPos	RefEndPos	Orientation	Confidence	HitEnum".to_string()];
        let mut hg38_lines = vec!["#h XmapEntryID	QryContigID	RefContigID	QryStartPos	QryEndPos	RefStartPos	RefEndPos	Orientation	Confidence	HitEnum".to_string()];

        for i in 0..10000 {
            chm13_lines.push(format!("{}	{}	1	1000.0	2000.0	5000.0	6000.0	+	15.0	1M", i, i % 1000));
            hg38_lines.push(format!("{}	{}	2	1500.0	2500.0	7000.0	8000.0	-	14.0	1M", i + 10000, i % 1000));
        }

        let chm13_content = chm13_lines.join("\n");
        let hg38_content = hg38_lines.join("\n");

        let start = Instant::now();
        let chm13_records = parse_xmap_file(&chm13_content).unwrap();
        let hg38_records = parse_xmap_file(&hg38_content).unwrap();
        let hg38_index = build_index(hg38_records);
        let rx = stream_matches(chm13_records, hg38_index);

        let mut count = 0;
        while rx.recv().is_ok() {
            count += 1;
        }

        let duration = start.elapsed();
        println!("Matched {} records in {:?}", count, duration);

        assert!(count > 0);
        assert!(duration.as_secs() < 5); // should be fast ahhh hell
    }
}