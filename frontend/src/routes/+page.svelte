<script lang="ts">
  import { onMount } from 'svelte';
  import DonutChart from '$lib/DonutChart.svelte';

  interface MatchedRecord {
    file_index: number;
    ref_contig_id: number;
    qry_start_pos: number;
    qry_end_pos: number;
    ref_start_pos: number;
    ref_end_pos: number;
    orientation: string;
    confidence: number;
  }

  interface BackendMatch {
    qry_contig_id: number;
    file_indices: number[];
    records: MatchedRecord[];
  }

  let files = [
    { name: "genome1.xmap", rows: 0, color: "#3b82f6" },
    { name: "genome2.xmap", rows: 0, color: "#10b981" },
    { name: "genome3.xmap", rows: 0, color: "#f59e0b" }
  ];

  let matches: BackendMatch[] = [];
  let isLoading = false;
  let error = '';

  // without DataView
  class BincodeDecoder {
    private data: Uint8Array;
    private offset: number;
    private textDecoder: TextDecoder;

    constructor(buffer: Uint8Array) {
      this.data = buffer;
      this.offset = 0;
      this.textDecoder = new TextDecoder('utf-8');
    }

    deserializeU32(): number {
      const value = 
        this.data[this.offset] |
        (this.data[this.offset + 1] << 8) |
        (this.data[this.offset + 2] << 16) |
        (this.data[this.offset + 3] << 24);
      this.offset += 4;
      return value >>> 0; // convert 2 unsigned
    }

    deserializeU8(): number {
      const value = this.data[this.offset];
      this.offset += 1;
      return value;
    }

    deserializeF64(): number {
      const bytes = this.data.slice(this.offset, this.offset + 8);
      const view = new DataView(bytes.buffer);
      const value = view.getFloat64(0, true);
      this.offset += 8;
      return value;
    }

    deserializeChar(): string {
      const bytes = [];
      let byte;
      do {
        byte = this.data[this.offset];
        this.offset += 1;
        bytes.push(byte);
      } while (byte !== 0);
      
      // yoink null terminator and decode
      bytes.pop();
      return this.textDecoder.decode(new Uint8Array(bytes));
    }

    deserializeLen(): number {
      // bincode uses LEB128 for length encoding
      let result = 0;
      let shift = 0;
      let byte;

      do {
        byte = this.data[this.offset];
        this.offset += 1;
        result |= (byte & 0x7f) << shift;
        shift += 7;
      } while (byte & 0x80);

      return result;
    }

    getRemainingBytes(): number {
      return this.data.length - this.offset;
    }
  }

  async function uploadFiles(fileInputs: FileList) {
    if (fileInputs.length < 2 || fileInputs.length > 3) {
      error = 'Please upload 2-3 XMAP files';
      return;
    }

    isLoading = true;
    error = '';
    matches = [];

    // file names from actual uploaded files
    files = Array.from(fileInputs).map((f, i) => ({
      name: f.name,
      rows: 0,
      color: ['#3b82f6', '#10b981', '#f59e0b'][i]
    }));

    try {
      const formData = new FormData();
      for (let i = 0; i < fileInputs.length; i++) {
        formData.append(`file${i}`, fileInputs[i]);
      }

      console.log('Sending files:', Array.from(fileInputs).map(f => f.name));

      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), 60000);

      const response = await fetch('/api/match', {
        method: 'POST',
        body: formData,
        signal: controller.signal
      });

      clearTimeout(timeoutId);

      if (!response.ok) {
        throw new Error(`Server returned ${response.status}: ${response.statusText}`);
      }

      console.log('Response received, processing stream...');

      const reader = response.body?.getReader();
      if (!reader) throw new Error('No reader available');

      let buffer = new Uint8Array();
      let matchesCount = 0;

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        const newBuffer = new Uint8Array(buffer.length + value.length);
        newBuffer.set(buffer);
        newBuffer.set(value, buffer.length);
        buffer = newBuffer;

        let offset = 0;
        while (offset + 4 <= buffer.length) {
          const length = new DataView(buffer.buffer, offset, 4).getUint32(0, true);
          offset += 4;

          if (offset + length > buffer.length) {
            offset -= 4;
            break;
          }

          const messageBytes = buffer.slice(offset, offset + length);
          offset += length;

          try {
            const matchData = decodeBackendMatch(messageBytes);
            matches = [...matches, matchData];
            matchesCount++;
            
            if (matchesCount % 100 === 0) {
              console.log(`Processed ${matchesCount} matches...`);
            }
          } catch (e) {
            console.error('Decode error:', e);
            if (matchesCount === 0) {
              debugBincodeStream(messageBytes);
            }
          }
        }

        buffer = buffer.slice(offset);
      }

      console.log(`Complete! Found ${matches.length} total matches`);

    } catch (err) {
      if (err instanceof Error) {
        if (err.name === 'AbortError') {
          error = 'Request timeout';
        } else {
          error = `Error: ${err.message}`;
        }
      } else {
        error = 'Unknown error';
      }
      console.error(err);
    } finally {
      isLoading = false;
    }
  }

  function decodeBackendMatch(bytes: Uint8Array): BackendMatch {
    const decoder = new BincodeDecoder(bytes);
    
    // decode in the same order as Rust struct serialization
    const qry_contig_id = decoder.deserializeU32();
    
    //decode file_indices: Box<[usize]>
    const file_indices_len = decoder.deserializeLen();
    const file_indices: number[] = [];
    for (let i = 0; i < file_indices_len; i++) {
      file_indices.push(decoder.deserializeU32()); // usize as u32
    }
    
    // decode records: Box<[MatchedRecord]>
    const records_len = decoder.deserializeLen();
    const records: MatchedRecord[] = [];
    for (let i = 0; i < records_len; i++) {
      records.push(decodeMatchedRecord(decoder));
    }
    
    return {
      qry_contig_id,
      file_indices,
      records
    };
  }

  function decodeMatchedRecord(decoder: BincodeDecoder): MatchedRecord {
    return {
      file_index: decoder.deserializeU32(), // usize as u32
      ref_contig_id: decoder.deserializeU8(),
      qry_start_pos: decoder.deserializeF64(),
      qry_end_pos: decoder.deserializeF64(),
      ref_start_pos: decoder.deserializeF64(),
      ref_end_pos: decoder.deserializeF64(),
      orientation: decoder.deserializeChar(),
      confidence: decoder.deserializeF64()
    };
  }

  function updateFileCounts() {
    const fileCounts = new Map<number, number>();
    for (const match of matches) {
      for (const record of match.records) {
        fileCounts.set(record.file_index, (fileCounts.get(record.file_index) || 0) + 1);
      }
    }

    files = files.map((f, i) => ({
      ...f,
      rows: fileCounts.get(i) || 0
    }));
  }

  // troubleshooting bincode stream
  function debugBincodeStream(bytes: Uint8Array) {
    console.log('Raw bytes length:', bytes.length);
    console.log('First 20 bytes:', Array.from(bytes.slice(0, 20)));
    
    try {
      const decoder = new BincodeDecoder(bytes);
      const qry_contig_id = decoder.deserializeU32();
      console.log('qry_contig_id:', qry_contig_id);
      
      const file_indices_len = decoder.deserializeLen();
      console.log('file_indices_len:', file_indices_len);
      
      const file_indices: number[] = [];
      for (let i = 0; i < file_indices_len; i++) {
        const idx = decoder.deserializeU32();
        file_indices.push(idx);
        console.log(`file_indices[${i}]:`, idx);
      }
      
      const records_len = decoder.deserializeLen();
      console.log('records_len:', records_len);
      
      console.log('Remaining bytes:', decoder.getRemainingBytes());
      
    } catch (e) {
      console.error('Debug decoding failed:', e);
    }
  }

  let fileInput: HTMLInputElement;

  async function testBackendConnection() {
    try {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), 5000);
      
      const response = await fetch('/', {
        method: 'HEAD',
        signal: controller.signal
      });
      
      clearTimeout(timeoutId);
      console.log('Backend is reachable');
      return true;
    } catch (err) {
      console.error('Backend is not reachable:', err);
      error = 'Backend server is not running. Please start the server on http://127.0.0.1:8080';
      return false;
    }
  }

  onMount(async () => {
    await testBackendConnection();
  });
</script>

<main>
  <div class="page">
    <h1>XMAP Chromosome Flow Visualization</h1>
    
    <div class="upload-section">
      <input
        type="file"
        accept=".xmap"
        multiple
        bind:this={fileInput}
        on:change={(e) => uploadFiles(e.currentTarget.files!)}
      />
      <button on:click={() => fileInput.click()}>
        Upload 2-3 XMAP Files
      </button>
      
      {#if isLoading}
        <div class="status">Processing... Found {matches.length} matches so far</div>
      {/if}
      
      {#if error}
        <div class="error">{error}</div>
      {/if}
    </div>

    {#if matches.length > 0}
      <DonutChart {files} {matches} />
    {:else if !isLoading}
      <div class="placeholder">
        Upload XMAP files to see chromosome flow visualization
      </div>
    {/if}
  </div>
</main>

<style>
  .page {
    padding: 2rem;
    max-width: 1400px;
    margin: 0 auto;
  }

  h1 {
    margin-bottom: 2rem;
  }

  .upload-section {
    margin-bottom: 2rem;
    padding: 1.5rem;
    background: #f9fafb;
    border-radius: 0.5rem;
    border: 2px dashed #d1d5db;
  }

  input[type="file"] {
    display: none;
  }

  button {
    padding: 0.75rem 1.5rem;
    background: #3b82f6;
    color: white;
    border: none;
    border-radius: 0.5rem;
    font-weight: 500;
    cursor: pointer;
  }

  button:hover {
    background: #2563eb;
  }

  .status {
    margin-top: 1rem;
    color: #6b7280;
    font-style: italic;
  }

  .error {
    margin-top: 1rem;
    padding: 0.75rem;
    background: #fef2f2;
    color: #dc2626;
    border-radius: 0.375rem;
  }

  .placeholder {
    text-align: center;
    padding: 4rem;
    color: #9ca3af;
    font-size: 1.125rem;
  }
</style>