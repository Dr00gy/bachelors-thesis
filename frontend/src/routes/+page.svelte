<script lang="ts">
  import { onMount } from 'svelte';
  import FileUpload from './FileUpload.svelte';
  import ErrorBanner from './ErrorBanner.svelte';
  import DonutVisualisation from './DonutVisualisation.svelte';
  import DisplayControls from './DisplayControls.svelte';
  import LoadingSpinner from './LoadingSpinner.svelte';
  import TabNav from './TabNav.svelte';
  import DarkModeToggle from './DarkModeToggle.svelte';
  import AreaAnalysis from './AreaAnalysis.svelte'; 
  import { fetchMatches, type BackendMatch, type ChromosomeInfo } from '$lib/bincodeDecoder';
  import { darkMode } from '$lib/darkModeStore';

  /**
   * Represents file metadata for the application
   */
  interface FileData {
    name: string;
    rows: number;
    color: string;
  }

  /**
   * Application state
   */
  let files: FileData[] = [];
  let matches: BackendMatch[] = [];
  let chromosomeInfo: ChromosomeInfo[][] = [];
  let isLoading = false;
  let error = '';
  let matchCount = 0;
  let abortController: AbortController | null = null;
  let showDuplicates = false;
  let activeTab: 'visualization' | 'analysis' = 'visualization';
  let hasUploadedFiles = false;
  let scale = 1.0;

  /**
   * Initialize dark mode on mount
   */
  onMount(() => {
    document.documentElement.classList.toggle('dark', $darkMode);
  });

  /**
   * Handles file upload and match processing
   * @param fileList - List of uploaded XMAP files
   */
  async function handleFileUpload(fileList: FileList) {
    if (!fileList || fileList.length === 0) return;

    if (fileList.length < 2 || fileList.length > 3) {
      error = 'Please upload 2-3 XMAP files';
      return;
    }

    if (abortController) {
      abortController.abort();
    }

    abortController = new AbortController();
    isLoading = true;
    error = '';
    matches = [];
    chromosomeInfo = [];
    matchCount = 0;
    hasUploadedFiles = false;

    files = Array.from(fileList).map((file, i) => ({
      name: file.name,
      rows: 0,
      color: ['#3b82f6', '#10b981', '#f59e0b'][i],
    }));

    try {
      const response = await fetchMatches(
        Array.from(fileList),
        (count) => {
          matchCount = count;
        },
        abortController.signal
      );

      matches = response.matches;
      chromosomeInfo = response.chromosomeInfo;
      updateFileCounts();
      hasUploadedFiles = true;
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
    } finally {
      isLoading = false;
      abortController = null;
    }
  }

  /**
   * Resets the application state for new upload
   */
  function resetUpload() {
    files = [];
    matches = [];
    chromosomeInfo = [];
    error = '';
    matchCount = 0;
    hasUploadedFiles = false;
    if (abortController) {
      abortController.abort();
      abortController = null;
    }
  }

  /**
   * Updates file row counts based on match data
   */
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
      rows: fileCounts.get(i) || 0,
    }));
  }

  /**
   * Cancels ongoing file upload
   */
  function cancelUpload() {
    if (abortController) {
      abortController.abort();
    }
  }
</script>

<main class="page">
  <div class="header">
    <h1>OGM Visualiser</h1>
    <div class="header-actions">
      <DarkModeToggle />
      {#if hasUploadedFiles && !isLoading}
        <button class="reset-button" on:click={resetUpload}>
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <path d="M2 8a6 6 0 0 1 10.5-4M14 8a6 6 0 0 1-10.5 4" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
            <path d="M12.5 2v4h-4" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          Reupload Files
        </button>
      {/if}
    </div>
  </div>

  <TabNav bind:activeTab />

  {#if activeTab === 'visualization'}
    <div class="tab-content">
      {#if !hasUploadedFiles && !isLoading}
        <FileUpload
          on:upload={(e) => handleFileUpload(e.detail)}
          on:cancel={cancelUpload}
        />
      {/if}

      {#if error}
        <ErrorBanner {error} />
      {/if}

      {#if isLoading}
        <div class="loading-container">
          <LoadingSpinner />
          <p class="loading-text">Processing {matchCount} matches...</p>
          <button class="cancel-button" on:click={cancelUpload}>Cancel</button>
        </div>
      {/if}

      {#if hasUploadedFiles && !isLoading}
        <DisplayControls bind:showDuplicates bind:scale />
        <DonutVisualisation {files} {matches} {chromosomeInfo} {showDuplicates} {scale} />
      {/if}
    </div>
  {:else if activeTab === 'analysis'}
    <div class="tab-content">
      {#if matches.length > 0}
        <AreaAnalysis {matches} {files} />
      {:else}
        <div class="placeholder-tab">
          <h2>Analysis Tab</h2>
          <p class="data-status">No data loaded. Switch to Chromosome Flow tab to upload files.</p>
        </div>
      {/if}
    </div>
  {/if}
</main>

<style>
  :global(:root) {
    --bg-primary: #ffffff;
    --bg-secondary: #f9fafb;
    --bg-hover: #f3f4f6;
    --bg-accent: #f0f9ff;
    --text-primary: #1f2937;
    --text-secondary: #6b7280;
    --text-tertiary: #9ca3af;
    --border-color: #e5e7eb;
    --border-color-dark: #d1d5db;
    --accent-primary: #3b82f6;
    --accent-hover: #2563eb;
    --accent-light: #dbeafe;
    --success: #10b981;
    --warning: #f59e0b;
    --error: #ef4444;
    --error-bg: #fef2f2;
    --error-border: #fecaca;
  }

  :global(.dark) {
    --bg-primary: #0f1419;
    --bg-secondary: #1a1f2e;
    --bg-hover: #242b3d;
    --bg-accent: #1e2b3f;
    --text-primary: #e8edf4;
    --text-secondary: #b1bfd0;
    --text-tertiary: #8f9db2;
    --border-color: #2d3748;
    --border-color-dark: rgb(50, 60, 80);
    --accent-primary: #5295e7;
    --accent-hover: #3b82f6;
    --accent-light: #1e3a5f;
    --success: #34d399;
    --warning: #fbbf24;
    --error: #f87171;
    --error-bg: #2d1f1f;
    --error-border: #4a2020;
  }

  :global(body) {
    background: var(--bg-primary);
    color: var(--text-primary);
    transition: background-color 0.2s, color 0.2s;
  }

  .page {
    padding: 2rem;
    max-width: 1400px;
    margin: 0 auto;
    background: var(--bg-primary);
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 2rem;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  h1 {
    margin: 0;
    color: var(--text-primary);
  }

  .reset-button {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.625rem 1.25rem;
    background: var(--bg-primary);
    color: var(--accent-primary);
    border: 2px solid var(--accent-primary);
    border-radius: 0.5rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
    font-size: 0.875rem;
  }

  .reset-button:hover {
    background: var(--accent-primary);
    color: var(--bg-primary);
  }

  .tab-content {
    animation: fadeIn 0.2s ease-in;
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .loading-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
    padding: 4rem;
    background: var(--bg-secondary);
    border-radius: 0.5rem;
    border: 1px solid var(--border-color);
  }

  .loading-text {
    color: var(--text-secondary);
    font-weight: 500;
  }

  .cancel-button {
    padding: 0.5rem 1.5rem;
    background: var(--error);
    color: white;
    border: none;
    border-radius: 0.5rem;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.2s;
  }

  .cancel-button:hover {
    background: #dc2626;
  }

  .placeholder-tab {
    text-align: center;
    padding: 4rem;
    background: var(--bg-secondary);
    border-radius: 0.5rem;
    border: 1px solid var(--border-color);
  }

  .placeholder-tab h2 {
    color: var(--text-primary);
    margin-bottom: 1rem;
  }

  .placeholder-tab p {
    color: var(--text-secondary);
    margin-bottom: 0.5rem;
  }

  .data-status {
    margin-top: 2rem;
    padding: 1rem;
    background: var(--bg-accent);
    border-radius: 0.375rem;
    font-weight: 500;
    color: var(--text-primary);
  }
</style>