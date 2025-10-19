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
 * Utility class for reading binary data with various type support
 */
class ByteReader {
  private data: Uint8Array;
  private pos: number;

  constructor(data: Uint8Array) {
    this.data = data;
    this.pos = 0;
  }

  /** Reads a single unsigned 8-bit integer */
  readU8(): number {
    if (this.pos >= this.data.length) {
      throw new Error(`Read past end: pos=${this.pos}, len=${this.data.length}`);
    }
    return this.data[this.pos++];
  }

  /** Reads a 32-bit unsigned integer in little-endian format */
  readU32(): number {
    const b0 = this.readU8();
    const b1 = this.readU8();
    const b2 = this.readU8();
    const b3 = this.readU8();
    return b0 | (b1 << 8) | (b2 << 16) | (b3 << 24);
  }

  /** Reads a 64-bit unsigned integer in little-endian format */
  readU64(): bigint {
    const low = BigInt(this.readU32());
    const high = BigInt(this.readU32());
    return low | (high << 32n);
  }

  /** Reads a 64-bit floating point number in little-endian format */
  readF64(): number {
    const bytes = new Uint8Array(8);
    for (let i = 0; i < 8; i++) {
      bytes[i] = this.readU8();
    }
    return new DataView(bytes.buffer).getFloat64(0, true);
  }

  /** Reads a UTF-8 character with proper encoding handling */
  readChar(): string {
    const charBytes: number[] = [];
    const firstByte = this.readU8();
    charBytes.push(firstByte);
    
    let additionalBytes = 0;
    if ((firstByte & 0x80) === 0) {
      additionalBytes = 0;
    } else if ((firstByte & 0xE0) === 0xC0) {
      additionalBytes = 1;
    } else if ((firstByte & 0xF0) === 0xE0) {
      additionalBytes = 2;
    } else if ((firstByte & 0xF8) === 0xF0) {
      additionalBytes = 3;
    }
    
    for (let i = 0; i < additionalBytes; i++) {
      charBytes.push(this.readU8());
    }
    
    const decoder = new TextDecoder('utf-8');
    return decoder.decode(new Uint8Array(charBytes));
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
 * Processes streaming response and yields decoded matches
 * @param response - Fetch Response object with streaming body
 * @returns AsyncGenerator yielding BackendMatch objects
 */
export async function* processMatchStream(response: Response): AsyncGenerator<BackendMatch> {
  const reader = response.body?.getReader();
  if (!reader) {
    throw new Error('No response body available');
  }

  let buffer = new Uint8Array(0);
  let messageCount = 0;

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
        
        if (length > 10000) {
          throw new Error(`Invalid message length: ${length}`);
        }

        if (buffer.length < 4 + length) {
          break;
        }
        
        const messageBytes = buffer.slice(4, 4 + length);
        buffer = buffer.slice(4 + length);

        try {
          const match = decodeBackendMatch(messageBytes);
          messageCount++;
          yield match;
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
 * @returns Promise resolving to array of BackendMatch objects
 */
export async function fetchMatches(
  files: File[],
  onProgress?: (count: number) => void,
  signal?: AbortSignal
): Promise<BackendMatch[]> {
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

  for await (const match of processMatchStream(response)) {
    matches.push(match);
    if (onProgress) {
      onProgress(matches.length);
    }
  }

  return matches;
}