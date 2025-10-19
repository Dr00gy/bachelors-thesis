<script lang="ts">
  import { onMount } from 'svelte';
  import FileUpload from '$lib/FileUpload.svelte';
  import ErrorBanner from '$lib/ErrorBanner.svelte';
  import DonutVisualization from '$lib/DonutVisualisation.svelte';
  import DisplayControls from '$lib/DisplayControls.svelte';
  import LoadingSpinner from '$lib/LoadingSpinner.svelte';
  import { fetchMatches, type BackendMatch } from '$lib/bincodeDecoder';

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
  let isLoading = false;
  let error = '';
  let matchCount = 0;
  let abortController: AbortController | null = null;
  let showDuplicates = false;

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
    matchCount = 0;

    files = Array.from(fileList).map((file, i) => ({
      name: file.name,
      rows: 0,
      color: ['#3b82f6', '#10b981', '#f59e0b'][i],
    }));

    try {
      matches = await fetchMatches(
        Array.from(fileList),
        (count) => {
          matchCount = count;
        },
        abortController.signal
      );

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
    } finally {
      isLoading = false;
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

  /**
   * Tests backend server connectivity
   * @returns Promise resolving to connection status
   */
  async function testBackendConnection(): Promise<boolean> {
    try {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), 5000);

      const response = await fetch('http://localhost:8080/', {
        method: 'HEAD',
        signal: controller.signal,
      });

      clearTimeout(timeoutId);
      return response.ok;
    } catch {
      return false;
    }
  }
</script>

<main class="page">
  <h1>Chromosome Flow Visualization</h1>

  <FileUpload
    {isLoading}
    {matchCount}
    on:upload={(e) => handleFileUpload(e.detail)}
    on:cancel={cancelUpload}
  />

  {#if error}
    <ErrorBanner {error} />
  {/if}

  <DisplayControls bind:showDuplicates />

  {#if isLoading}
    <LoadingSpinner />
  {:else if matches.length > 0}
    <DonutVisualization {files} {matches} {showDuplicates} />
  {:else}
    <div class="placeholder">Upload XMAP files to see chromosome flow visualization</div>
  {/if}
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

  .placeholder {
    text-align: center;
    padding: 4rem;
    color: #9ca3af;
    font-size: 1.125rem;
  }
</style>
