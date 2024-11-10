use std::sync::{Arc, Mutex};
use twobit::TwoBitPhysicalFile;

#[derive(Clone)]
pub struct SequenceData {
    pub chm13_2bit: Arc<Mutex<TwoBitPhysicalFile>>,
    pub hg38_2bit: Arc<Mutex<TwoBitPhysicalFile>>,
}