/**
 * Represents chromosome information with length
 */
export interface ChromosomeInfo {
  ref_contig_id: number;
  ref_len: number;
}

/**
 * Represents a single matched record from XMAP data
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
 * Represents a complete match between multiple XMAP records
 */
export interface BackendMatch {
  qry_contig_id: number;
  file_indices: number[];
  records: MatchedRecord[];
}

/**
 * Represents the complete response from the backend
 */
export interface BackendResponse {
  chromosomeInfo: ChromosomeInfo[][];
  matches: BackendMatch[];
}

/**
 * Utility class for reading binary data with various type support
 */
class ByteReader {
  private data: Uint8Array;
  private view: DataView;
  private pos: number;

  constructor(data: Uint8Array) {
    this.data = data;
    this.view = new DataView(data.buffer, data.byteOffset, data.byteLength);
    this.pos = 0;
  }

  /** Reads a single unsigned 8-bit integer */
  readU8(): number {
    if (this.pos >= this.data.length) {
      throw new Error(`Read past end: pos=${this.pos}, len=${this.data.length}`);
    }
    const v = this.view.getUint8(this.pos);
    this.pos += 1;
    return v;
  }

  /** Reads a 32-bit unsigned integer in little-endian format */
  readU32(): number {
    if (this.pos + 4 > this.data.length) {
      throw new Error(`readU32 out of range at ${this.pos}`);
    }
    const v = this.view.getUint32(this.pos, true);
    this.pos += 4;
    return v;
  }

  /** Reads a 64-bit unsigned integer in little-endian format */
  readU64(): bigint {
    if (this.pos + 8 > this.data.length) {
      throw new Error(`readU64 out of range at ${this.pos}`);
    }
    const v = this.view.getBigUint64(this.pos, true);
    this.pos += 8;
    return v;
  }

  /** Reads a 64-bit floating point number in little-endian format */
  readF64(): number {
    if (this.pos + 8 > this.data.length) {
      throw new Error(`readF64 out of range at ${this.pos}`);
    }
    const v = this.view.getFloat64(this.pos, true);
    this.pos += 8;
    return v;
  }

  /** Reads exactly one UTF-8 character */
  private static decoder = new TextDecoder('utf-8');

  readChar(): string {
    const first = this.readU8();

    let length = 1;
    if ((first & 0x80) === 0) length = 1;
    else if ((first & 0xE0) === 0xC0) length = 2;
    else if ((first & 0xF0) === 0xE0) length = 3;
    else if ((first & 0xF8) === 0xF0) length = 4;
    else throw new Error(`Invalid UTF-8 start byte: ${first}`);

    const bytes = new Uint8Array(length);
    bytes[0] = first;
    for (let i = 1; i < length; i++) {
      bytes[i] = this.readU8();
    }

    return ByteReader.decoder.decode(bytes);
  }

  /** Returns number of bytes remaining to read */
  remaining(): number {
    return this.data.length - this.pos;
  }

  /** Returns current read position */
  getPos(): number {
    return this.pos;
  }
}

/**
 * Decodes chromosome information from binary data
 * @param reader - ByteReader instance positioned at chromosome info start
 * @returns Decoded ChromosomeInfo array
 */
function decodeChromosomeInfo(reader: ByteReader): ChromosomeInfo[] {
  const length = Number(reader.readU64());
  const chromosomes: ChromosomeInfo[] = [];

  for (let i = 0; i < length; i++) {
    const ref_contig_id = reader.readU8();
    const ref_len = reader.readF64();
    chromosomes.push({ ref_contig_id, ref_len });
  }

  return chromosomes;
}

/**
 * Decodes a single MatchedRecord from binary data
 * @param reader - ByteReader instance positioned at record start
 * @returns Decoded MatchedRecord
 */
function decodeMatchedRecord(reader: ByteReader): MatchedRecord {
  const file_index = Number(reader.readU64());
  const ref_contig_id = reader.readU8();
  const qry_start_pos = reader.readF64();
  const qry_end_pos = reader.readF64();
  const ref_start_pos = reader.readF64();
  const ref_end_pos = reader.readF64();
  const orientation = reader.readChar();
  const confidence = reader.readF64();
  const ref_len = reader.readF64();

  return {
    file_index,
    ref_contig_id,
    qry_start_pos,
    qry_end_pos,
    ref_start_pos,
    ref_end_pos,
    orientation,
    confidence,
    ref_len
  };
}

/**
 * Decodes a complete BackendMatch from binary data
 * @param bytes - Binary data containing serialized match
 * @returns Decoded BackendMatch
 * @throws Error if decoding fails or data is invalid
 */
function decodeBackendMatch(bytes: Uint8Array): BackendMatch {
  const reader = new ByteReader(bytes);

  const qry_contig_id = reader.readU32();

  const file_indices_len = Number(reader.readU64());
  if (file_indices_len > 100) {
    throw new Error(`Unrealistic file_indices_len: ${file_indices_len}`);
  }

  const file_indices: number[] = [];
  for (let i = 0; i < file_indices_len; i++) {
    const idx = Number(reader.readU64());
    file_indices.push(idx);
  }

  const records_len = Number(reader.readU64());
  if (records_len > 100) {
    throw new Error(`Unrealistic records_len: ${records_len}`);
  }

  const records: MatchedRecord[] = [];
  for (let i = 0; i < records_len; i++) {
    const record = decodeMatchedRecord(reader);
    records.push(record);
  }

  return {
    qry_contig_id,
    file_indices,
    records
  };
}

/**
 * Processes streaming response and yields decoded chromosome info and matches
 * @param response - Fetch Response object with streaming body
 * @returns AsyncGenerator yielding BackendResponse object
 */
export async function* processMatchStream(response: Response): AsyncGenerator<BackendResponse> {
  const reader = response.body?.getReader();
  if (!reader) {
    throw new Error('No response body available');
  }

  let buffer = new Uint8Array(0);
  let messageCount = 0;
  let chromosomeInfo: ChromosomeInfo[][] | null = null;

  try {
    while (true) {
      const { done, value } = await reader.read();

      if (done) {
        if (buffer.length > 0) {
          console.warn(`Leftover bytes: ${buffer.length}`);
        }
        break;
      }

      const newBuffer = new Uint8Array(buffer.length + value.length);
      newBuffer.set(buffer);
      newBuffer.set(value, buffer.length);
      buffer = newBuffer;

      while (buffer.length >= 4) {
        const length = buffer[0] | (buffer[1] << 8) | (buffer[2] << 16) | (buffer[3] << 24);
        
        if (length > 1000000) {
          throw new Error(`Invalid message length: ${length}`);
        }

        if (buffer.length < 4 + length) {
          break;
        }
        
        const messageBytes = buffer.slice(4, 4 + length);
        buffer = buffer.slice(4 + length);

        try {
          if (!chromosomeInfo) {
            const reader = new ByteReader(messageBytes);
            chromosomeInfo = [];
            const num_files = Number(reader.readU64());
            for (let i = 0; i < num_files; i++) {
              chromosomeInfo.push(decodeChromosomeInfo(reader));
            }
            yield { chromosomeInfo, matches: [] };
          } else {
            const match = decodeBackendMatch(messageBytes);
            messageCount++;
            yield { chromosomeInfo, matches: [match] };
          }
        } catch (error) {
          console.error(`Failed to decode message ${messageCount + 1}:`, error);
        }
      }
    }
  } finally {
    reader.releaseLock();
  }
}

/**
 * Fetches matches from backend API for given XMAP files
 * @param files - Array of XMAP File objects (2-3 files)
 * @param onProgress - Optional progress callback
 * @param signal - Optional AbortSignal for cancellation
 * @returns Promise resolving to BackendResponse object
 */
export async function fetchMatches(
  files: File[],
  onProgress?: (count: number) => void,
  signal?: AbortSignal
): Promise<BackendResponse> {
  if (files.length < 2 || files.length > 3) {
    throw new Error('Please provide 2-3 XMAP files');
  }

  const formData = new FormData();
  files.forEach((file, i) => {
    formData.append(`file${i}`, file);
  });

  const response = await fetch('http://localhost:8080/api/match', {
    method: 'POST',
    body: formData,
    signal
  });

  if (!response.ok) {
    throw new Error(`Server error: ${response.status} ${response.statusText}`);
  }

  const matches: BackendMatch[] = [];
  let chromosomeInfo: ChromosomeInfo[][] = [];

  for await (const responseData of processMatchStream(response)) {
    if (responseData.chromosomeInfo.length > 0 && chromosomeInfo.length === 0) {
      chromosomeInfo = responseData.chromosomeInfo;
    }
    if (responseData.matches.length > 0) {
      matches.push(...responseData.matches);
      if (onProgress) {
        onProgress(matches.length);
      }
    }
  }

  return { chromosomeInfo, matches };
}

export const __testUtils = {
  decodeBackendMatch,
  decodeChromosomeInfo,
};