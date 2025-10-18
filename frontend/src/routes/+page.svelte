<script lang="ts">
  import { onMount } from 'svelte';
  import DonutChart from '$lib/DonutChart.svelte';
  import { fetchMatches, type BackendMatch } from '$lib/bincodeDecoder';

  interface FileData {
    name: string;
    rows: number;
    color: string;
  }

  let files: FileData[] = [
    { name: "genome1.xmap", rows: 0, color: "#3b82f6" },
    { name: "genome2.xmap", rows: 0, color: "#10b981" },
    { name: "genome3.xmap", rows: 0, color: "#f59e0b" }
  ];

  let matches: BackendMatch[] = [];
  let isLoading = false;
  let error = '';
  let matchCount = 0;
  let fileInput: HTMLInputElement;
  let abortController: AbortController | null = null;
  let showDuplicates = false;

  async function handleFileUpload(fileList: FileList | null) {
    if (!fileList || fileList.length === 0) return;

    if (fileList.length < 2 || fileList.length > 3) {
      error = 'Please upload 2-3 XMAP files';
      return;
    }

    // for cancelling later, rn tests are so fast that i cant test it
    if (abortController) {
      abortController.abort();
    }

    abortController = new AbortController();
    isLoading = true;
    error = '';
    matches = [];
    matchCount = 0;

    files = Array.from(fileList).map((file, i) => ({
      name: file.name,
      rows: 0,
      color: ['#3b82f6', '#10b981', '#f59e0b'][i]
    }));

    console.log('Uploading files:', files.map(f => f.name));

    try {
      matches = await fetchMatches(
        Array.from(fileList),
        (count) => {
          matchCount = count;
          if (count % 100 === 0) {
            console.log(`Processed ${count} matches...`);
          }
        },
        abortController.signal
      );

      console.log(`Complete! Found ${matches.length} total matches`);
      updateFileCounts();

    } catch (err) {
      if (err instanceof Error) {
        if (err.name === 'AbortError') {
          error = 'Upload cancelled';
        } else {
          error = err.message;
        }
      } else {
        error = 'Unknown error occurred';
      }
      console.error('Upload error:', err);
    } finally {
      isLoading = false;
      abortController = null;
    }
  }

  function updateFileCounts() {
    const fileCounts = new Map<number, number>();
    
    for (const match of matches) {
      for (const record of match.records) {
        const count = fileCounts.get(record.file_index) || 0;
        fileCounts.set(record.file_index, count + 1);
      }
    }

    files = files.map((file, i) => ({
      ...file,
      rows: fileCounts.get(i) || 0
    }));
  }

  function cancelUpload() {
    if (abortController) {
      abortController.abort();
    }
  }

  async function testBackendConnection(): Promise<boolean> {
    try {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), 5000);
      
      const response = await fetch('http://localhost:8080/', {
        method: 'HEAD',
        signal: controller.signal
      });
      
      clearTimeout(timeoutId);
      
      if (response.ok) {
        console.log('✓ Backend is reachable');
        return true;
      } else {
        throw new Error(`Backend returned ${response.status}`);
      }
    } catch (err) {
      console.error('✗ Backend is not reachable:', err);
      error = 'Backend server is not running. Please start the server on http://localhost:8080';
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
        on:change={(e) => handleFileUpload(e.currentTarget.files)}
      />
      
      <div class="upload-controls">
        <button 
          on:click={() => fileInput.click()}
          disabled={isLoading}
        >
          {isLoading ? 'Processing...' : 'Upload 2-3 XMAP Files'}
        </button>

        {#if isLoading}
          <button 
            on:click={cancelUpload}
            class="cancel-button"
          >
            Cancel
          </button>
        {/if}
      </div>
      
      {#if isLoading}
        <div class="status">
          Processing... Found {matchCount} matches so far
        </div>
      {/if}
      
      {#if error}
        <div class="error">{error}</div>
      {/if}
    </div>

          {#if matches.length > 0}
        <div class="display-controls">
          <label class="toggle-label">
            <input 
              type="checkbox" 
              bind:checked={showDuplicates}
            />
            <span class="toggle-slider"></span>
            Show self-flow lines (same genome)
          </label>
          <div class="toggle-description">
            {#if showDuplicates}
              Showing self-flow lines within the same genome
            {:else}
              Showing only flow lines between different genomes
            {/if}
          </div>
        </div>
      {/if}

    {#if matches.length > 0}
      <DonutChart {files} {matches} {showDuplicates} />
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

  .upload-controls {
    display: flex;
    gap: 1rem;
    align-items: center;
  }

  button {
    padding: 0.75rem 1.5rem;
    background: #3b82f6;
    color: white;
    border: none;
    border-radius: 0.5rem;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.2s;
  }

  button:hover:not(:disabled) {
    background: #2563eb;
  }

  button:disabled {
    background: #9ca3af;
    cursor: not-allowed;
  }

  .cancel-button {
    background: #ef4444;
  }

  .cancel-button:hover {
    background: #dc2626;
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
    border: 1px solid #fecaca;
  }

  .placeholder {
    text-align: center;
    padding: 4rem;
    color: #9ca3af;
    font-size: 1.125rem;
  }

  .display-controls {
    margin-top: 1rem;
    padding: 1rem;
    background: white;
    border-radius: 0.375rem;
    border: 1px solid #e5e7eb;
  }

  .toggle-label {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    cursor: pointer;
    font-weight: 500;
    color: #374151;
  }

  .toggle-label input {
    display: none;
  }

  .toggle-slider {
    width: 3rem;
    height: 1.5rem;
    background: #d1d5db;
    border-radius: 1rem;
    position: relative;
    transition: background 0.2s;
  }

  .toggle-slider::before {
    content: '';
    position: absolute;
    width: 1.25rem;
    height: 1.25rem;
    background: white;
    border-radius: 50%;
    top: 0.125rem;
    left: 0.125rem;
    transition: transform 0.2s;
  }

  .toggle-label input:checked + .toggle-slider {
    background: #3b82f6;
  }

  .toggle-label input:checked + .toggle-slider::before {
    transform: translateX(1.5rem);
  }

  .toggle-description {
    margin-top: 0.5rem;
    font-size: 0.875rem;
    color: #6b7280;
    font-style: italic;
  }
</style>