/**
 * Represents file metadata for visualization
 */
export interface FileData {
  name: string;
  rows: number;
  color: string;
}

/**
 * Represents a single matched record position
 */
export interface MatchedRecord {
  file_index: number;
  ref_contig_id: number;
  qry_start_pos: number;
  qry_end_pos: number;
  ref_start_pos: number;
  ref_end_pos: number;
  orientation: string;
  confidence: number;
  ref_len: number;
}

/**
 * Represents a complete backend match with multiple records
 */
export interface BackendMatch {
  qry_contig_id: number;
  file_indices: number[];
  records: MatchedRecord[];
}

/**
 * Represents a donut chart segment
 */
export interface DonutSegment {
  name: string;
  rows: number;
  color: string;
  index: number;
  genomeSize: number;
  dashArray: string;
  dashOffset: number;
  percentage: string;
  showLabel: boolean;
  showChromosomes: boolean;
  startAngle: number;
  endAngle: number;
  angleRange: number;
}

/**
 * Represents a flow path between two points
 */
export interface FlowPath {
  path: string;
  p1: { x: number; y: number };
  p2: { x: number; y: number };
  fromOrientation: string;
  toOrientation: string;
  color: string;
  opacity: number;
  width: number;
  fromChromosome: number;
  toChromosome: number;
  confidence: number;
  fromFileIndex: number;
  toFileIndex: number;
  isSameGenome: boolean;
  qryContigId: number;
  fromRecord: MatchedRecord;
  toRecord: MatchedRecord;
}

/**
 * Represents chromosome division markers
 */
export interface ChromosomeDivision {
  chromosome: number;
  startAngle: number;
  endAngle: number;
  midAngle: number;
}