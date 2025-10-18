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

export interface BackendMatch {
  qry_contig_id: number;
  file_indices: number[];
  records: MatchedRecord[];
}

class ByteReader {
  private data: Uint8Array;
  private pos: number;

  constructor(data: Uint8Array) {
    this.data = data;
    this.pos = 0;
  }

  readU8(): number {
    if (this.pos >= this.data.length) {
      throw new Error(`Read past end: pos=${this.pos}, len=${this.data.length}`);
    }
    return this.data[this.pos++];
  }

  readU32(): number {
    const b0 = this.readU8();
    const b1 = this.readU8();
    const b2 = this.readU8();
    const b3 = this.readU8();
    return b0 | (b1 << 8) | (b2 << 16) | (b3 << 24);
  }

  readU64(): bigint {
    const low = BigInt(this.readU32());
    const high = BigInt(this.readU32());
    return low | (high << 32n);
  }

  readF64(): number {
    const bytes = new Uint8Array(8);
    for (let i = 0; i < 8; i++) {
      bytes[i] = this.readU8();
    }
    return new DataView(bytes.buffer).getFloat64(0, true);
  }

  readChar(): string {
    const charBytes: number[] = [];
    const firstByte = this.readU8();
    charBytes.push(firstByte);
    
    // which UTF 8 are we workin on
    let additionalBytes = 0;
    if ((firstByte & 0x80) === 0) {
      // single byte (ASCII)
      additionalBytes = 0;
    } else if ((firstByte & 0xE0) === 0xC0) {
      additionalBytes = 1;
    } else if ((firstByte & 0xF0) === 0xE0) {
      additionalBytes = 2;
    } else if ((firstByte & 0xF8) === 0xF0) {
      additionalBytes = 3;
    }
    
    // read additional bytes
    for (let i = 0; i < additionalBytes; i++) {
      charBytes.push(this.readU8());
    }
    
    const decoder = new TextDecoder('utf-8');
    return decoder.decode(new Uint8Array(charBytes));
  }

  remaining(): number {
    return this.data.length - this.pos;
  }

  getPos(): number {
    return this.pos;
  }
}

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

function decodeBackendMatch(bytes: Uint8Array): BackendMatch {
  const reader = new ByteReader(bytes);

  console.log(`\n=== Decoding message (${bytes.length} bytes) ===`);
  console.log('First 64 bytes:', Array.from(bytes.slice(0, 64)));

  try {
    // read qry_contig_id: u32
    const qry_contig_id = reader.readU32();
    console.log(`qry_contig_id: ${qry_contig_id} (pos: ${reader.getPos()})`);

    // read file_indices length: u64
    const file_indices_len = Number(reader.readU64());
    console.log(`file_indices_len: ${file_indices_len} (pos: ${reader.getPos()})`);

    if (file_indices_len > 100) {
      throw new Error(`Unrealistic file_indices_len: ${file_indices_len}`);
    }

    // read file_indices: [u64; len]
    const file_indices: number[] = [];
    for (let i = 0; i < file_indices_len; i++) {
      const idx = Number(reader.readU64());
      file_indices.push(idx);
      console.log(`  file_indices[${i}]: ${idx} (pos: ${reader.getPos()})`);
    }

    // rread records length: u64
    const records_len = Number(reader.readU64());
    console.log(`records_len: ${records_len} (pos: ${reader.getPos()})`);

    if (records_len > 100) {
      throw new Error(`Unrealistic records_len: ${records_len}`);
    }

    // read records: [MatchedRecord; len]
    const records: MatchedRecord[] = [];
    for (let i = 0; i < records_len; i++) {
      console.log(`Reading record ${i} at pos ${reader.getPos()}`);
      const record = decodeMatchedRecord(reader);
      records.push(record);
      console.log(`  record[${i}]:`, record, `(pos: ${reader.getPos()})`);
    }

    console.log(`Successfully decoded. Remaining bytes: ${reader.remaining()}`);

    return {
      qry_contig_id,
      file_indices,
      records
    };
  } catch (error) {
    console.error(`Decoding failed at position ${reader.getPos()}`);
    throw error;
  }
}

export async function* processMatchStream(response: Response): AsyncGenerator<BackendMatch> {
  const reader = response.body?.getReader();
  if (!reader) {
    throw new Error('No response body available');
  }

  let buffer = new Uint8Array(0);
  let messageCount = 0;

  try {
    console.log('\n=== Starting stream processing ===');

    while (true) {
      const { done, value } = await reader.read();

      if (done) {
        console.log(`\n=== Stream complete: ${messageCount} messages ===`);
        if (buffer.length > 0) {
          console.warn(`Leftover bytes: ${buffer.length}`, Array.from(buffer));
        }
        break;
      }

      console.log(`\nReceived chunk: ${value.length} bytes`);

      // appends to buffer
      const newBuffer = new Uint8Array(buffer.length + value.length);
      newBuffer.set(buffer);
      newBuffer.set(value, buffer.length);
      buffer = newBuffer;

      console.log(`Buffer size: ${buffer.length} bytes`);

      // proocesses complete messages
      while (buffer.length >= 4) {
        const length = buffer[0] | (buffer[1] << 8) | (buffer[2] << 16) | (buffer[3] << 24); // length prefix is u32 little-endian
        console.log(`Next message length: ${length} bytes`);

        if (length > 10000) {
          console.error(`Suspiciously large message: ${length} bytes`);
          console.error('Buffer start:', Array.from(buffer.slice(0, 32)));
          throw new Error(`Invalid message length: ${length}`);
        }

        // wait for complete message and extract it here
        if (buffer.length < 4 + length) {
          console.log(`Waiting for ${4 + length - buffer.length} more bytes`);
          break;
        }
        const messageBytes = buffer.slice(4, 4 + length);
        buffer = buffer.slice(4 + length);

        try {
          const match = decodeBackendMatch(messageBytes);
          messageCount++;
          console.log(`Message ${messageCount} decoded successfully`);
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

  console.log('\n=== Sending request ===');
  const response = await fetch('http://localhost:8080/api/match', {
    method: 'POST',
    body: formData,
    signal
  });

  if (!response.ok) {
    throw new Error(`Server error: ${response.status} ${response.statusText}`);
  }

  console.log('Response received, processing stream...');

  const matches: BackendMatch[] = [];

  for await (const match of processMatchStream(response)) {
    matches.push(match);
    if (onProgress) {
      onProgress(matches.length);
    }
  }

  console.log(`\n=== Fetch complete: ${matches.length} matches ===`);
  return matches;
}