<script lang="ts">
  import type { BackendMatch } from './bincodeDecoder';
  import type { FileData } from './types';

  /**
   * Component props
   */
  export let matches: BackendMatch[] = [];
  export let files: FileData[] = [];

  /**
   * Filter and display state
   */
  let selectedFiles: number[] = [0];
  let selectedChromosome = 1;
  let windowSize = 100000;
  let currentWindowIndex = 0;
  let hoveredContig: any = null;

  /**
   * Generates consistent HSL color for contig IDs
   */
  function generateContigColor(contigId: number): string {
    const hue = (contigId * 137.508) % 360;
    return `hsl(${hue}, 70%, 60%)`;
  }

  /**
   * Filters records by selected files and chromosome, deduplicating contigs
   */
  function getRecordsForChromosome(matches: BackendMatch[], fileIndices: number[], chromosome: number) {
    if (fileIndices.length === 0) return [];
    
    const records: any[] = [];
    const seenContigs = new Set();
    
    for (const match of matches) {
      for (const record of match.records) {
        if (fileIndices.includes(record.file_index) && record.ref_contig_id === chromosome) {
          const contigKey = `${match.qry_contig_id}_${record.ref_start_pos}_${record.ref_end_pos}`;
          
          if (!seenContigs.has(contigKey)) {
            seenContigs.add(contigKey);
            records.push({
              ...record,
              qry_contig_id: match.qry_contig_id
            });
          }
        }
      }
    }
    return records;
  }

  /**
   * Calculates min/max reference positions for chromosome records
   */
  function getChromosomeRange(records: any[]) {
    if (records.length === 0) return { min: 0, max: 100000 };
    const min = Math.min(...records.map(r => r.ref_start_pos));
    const max = Math.max(...records.map(r => r.ref_end_pos));
    return { min: Math.floor(min), max: Math.ceil(max) };
  }

  /**
   * Stacks overlapping contigs into tracks using reference positions
   */
  function stackContigs(records: any[], windowStart: number, windowEnd: number) {
    const visibleRecords = records.filter(r => 
      r.ref_end_pos >= windowStart && r.ref_start_pos <= windowEnd
    );

    visibleRecords.sort((a, b) => a.ref_start_pos - b.ref_start_pos);

    const stacked: any[][] = [];
    for (const record of visibleRecords) {
      let placed = false;
      
      for (let trackIndex = 0; trackIndex < stacked.length; trackIndex++) {
        const track = stacked[trackIndex];
        const lastInTrack = track[track.length - 1];
        
        if (record.ref_start_pos > lastInTrack.ref_end_pos) {
          track.push(record);
          placed = true;
          break;
        }
      }
      
      if (!placed) {
        stacked.push([record]);
      }
    }

    return stacked;
  }

  /**
   * Converts genomic position to percentage within current window
   */
  function posToX(pos: number, windowStart: number, windowSize: number): number {
    const relativePos = pos - windowStart;
    const percentage = (relativePos / windowSize) * 100;
    return Math.max(0, Math.min(100, percentage));
  }

  /**
   * Toggles file selection - now allows complete deselection
   */
  function toggleFileSelection(fileIndex: number) {
    if (selectedFiles.includes(fileIndex)) {
      selectedFiles = selectedFiles.filter(i => i !== fileIndex);
    } else {
      selectedFiles = [...selectedFiles, fileIndex].sort((a, b) => a - b);
    }
    currentWindowIndex = 0;
  }

  /**
   * Selects all available files
   */
  function selectAllFiles() {
    selectedFiles = files.map((_, idx) => idx);
    currentWindowIndex = 0;
  }

  /**
   * Resets file selection to empty array (complete clear)
   */
  function clearFileSelection() {
    selectedFiles = [];
    currentWindowIndex = 0;
  }

  /**
   * Reactive computed values for chromosome data and visualization
   */
  $: chromosomeRecords = getRecordsForChromosome(matches, selectedFiles, selectedChromosome);
  $: chromosomeRange = getChromosomeRange(chromosomeRecords);
  $: chromosomeRefLen = chromosomeRecords.length > 0 ? chromosomeRecords[0].ref_len : windowSize;
  $: windowStart = chromosomeRange.min + (currentWindowIndex * windowSize);
  $: windowEnd = Math.min(windowStart + windowSize, chromosomeRefLen);
  $: totalWindows = Math.ceil(chromosomeRefLen / windowSize);
  $: stackedContigs = stackContigs(chromosomeRecords, windowStart, windowEnd);
  $: uniqueContigs = Array.from(new Set(chromosomeRecords.map(r => r.qry_contig_id))).sort((a, b) => a - b);
  $: canGoPrev = currentWindowIndex > 0;
  $: canGoNext = currentWindowIndex < totalWindows - 1;

  /**
   * Available chromosomes (1-23)
   */
  const chromosomes = Array.from({ length: 23 }, (_, i) => i + 1);

  /**
   * Resets window position when chromosome changes
   */
  function handleChromosomeChange() {
    currentWindowIndex = 0;
  }
</script>

<div class="analysis-container">
  <!-- File selection controls -->
  <div class="controls">
    <div class="control-group full-width">
      <label for="file-selection">Select Files:</label>
      <div class="file-selection" id="file-selection">
        {#each files as file, idx}
          <label class="file-checkbox">
            <input 
              type="checkbox" 
              checked={selectedFiles.includes(idx)}
              on:change={() => toggleFileSelection(idx)}
            />
            <span class="file-checkbox-label">
              <span class="file-color-indicator" style="background: {file.color}"></span>
              {file.name}
            </span>
          </label>
        {/each}
      </div>
      <div class="file-selection-actions">
        <button class="action-btn" on:click={selectAllFiles}>Select All</button>
        <button class="action-btn" on:click={clearFileSelection}>Clear All</button>
        <span class="selected-count">{selectedFiles.length} of {files.length} selected</span>
      </div>
    </div>

    <div class="control-group">
      <label for="chromosome-select">Select Chromosome:</label>
      <select id="chromosome-select" bind:value={selectedChromosome} on:change={handleChromosomeChange}>
        {#each chromosomes as chr}
          <option value={chr}>Chromosome {chr}</option>
        {/each}
      </select>
    </div>
  </div>

  <!-- Query contig legend -->
  {#if uniqueContigs.length > 0}
    <div class="legend">
      <h3>Query Contigs ({uniqueContigs.length})</h3>
      <div class="legend-items">
        {#each uniqueContigs as contigId}
          <div class="legend-item">
            <div class="legend-color" style="background: {generateContigColor(contigId)}"></div>
            <span>QryContig {contigId}</span>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Genomic window navigation and information -->
  <div class="window-info">
    <div class="window-position">
      <strong>Window:</strong> {windowStart.toLocaleString()} - {windowEnd.toLocaleString()} bp
      <span class="window-count">({currentWindowIndex + 1} / {totalWindows})</span>
    </div>
    <div class="window-navigation">
      <button on:click={() => currentWindowIndex--} disabled={!canGoPrev}>
        ← Previous
      </button>
      <button on:click={() => currentWindowIndex++} disabled={!canGoNext}>
        Next →
      </button>
    </div>
  </div>

  <!-- Main chromosome browser visualization -->
  <div class="browser">
    {#if selectedFiles.length === 0}
      <div class="empty-state">
        No files selected. Please select one or more files to view mappings.
      </div>
    {:else if chromosomeRecords.length === 0}
      <div class="empty-state">
        No mappings found for this chromosome in selected files
      </div>
    {:else if stackedContigs.length === 0}
      <div class="empty-state">
        No mappings in this window. Use navigation buttons to explore other regions.
      </div>
    {:else}
      <!-- Genomic position markers -->
      <div class="position-markers">
        {#each [0, 0.25, 0.5, 0.75, 1] as fraction}
          {@const pos = windowStart + (windowSize * fraction)}
          {#if pos <= windowEnd}
            <div class="marker" style="left: {fraction * 100}%">
              <div class="marker-tick"></div>
              <div class="marker-label">{Math.round(pos).toLocaleString()}</div>
            </div>
          {/if}
        {/each}
      </div>

      <!-- Stacked contig tracks visualization -->
      <div class="contigs-container">
        {#each stackedContigs as track, trackIndex}
          <div class="contig-track">
            {#each track as record, recordIndex}
              {@const startX = posToX(record.ref_start_pos, windowStart, windowSize)}
              {@const endX = posToX(record.ref_end_pos, windowStart, windowSize)}
              {@const width = endX - startX}
              <div
                class="contig"
                class:hovered={hoveredContig === record}
                style="left: {startX}%; width: {width}%; background: {generateContigColor(record.qry_contig_id)}"
                on:mouseenter={() => hoveredContig = record}
                on:mouseleave={() => hoveredContig = null}
                role="button"
                tabindex="0"
              ></div>
            {/each}
          </div>
        {/each}
      </div>

      <!-- Hover tooltip with detailed contig information -->
      {#if hoveredContig}
        <div class="tooltip">
          <div class="tooltip-header">
            Query Contig {hoveredContig.qry_contig_id}
          </div>
          <div class="tooltip-body">
            <div class="tooltip-file">
              <span class="file-badge" style="background: {files[hoveredContig.file_index]?.color}20; color: {files[hoveredContig.file_index]?.color}; border-color: {files[hoveredContig.file_index]?.color}">
                {files[hoveredContig.file_index]?.name}
              </span>
            </div>
            <div class="tooltip-content">
              <div><strong>Ref Position:</strong> {hoveredContig.ref_start_pos.toLocaleString()} - {hoveredContig.ref_end_pos.toLocaleString()} bp</div>
              <div><strong>Query Position:</strong> {hoveredContig.qry_start_pos.toLocaleString()} - {hoveredContig.qry_end_pos.toLocaleString()} bp</div>
              <div><strong>Orientation:</strong> {hoveredContig.orientation}</div>
              <div><strong>Confidence:</strong> {hoveredContig.confidence.toFixed(2)}</div>
              <div><strong>Ref Length:</strong> {(hoveredContig.ref_end_pos - hoveredContig.ref_start_pos).toLocaleString()} bp</div>
              <div><strong>Query Length:</strong> {(hoveredContig.qry_end_pos - hoveredContig.qry_start_pos).toLocaleString()} bp</div>
            </div>
          </div>
        </div>
      {/if}
    {/if}
  </div>
</div>

<style>
  .analysis-container {
    padding: 2rem;
    max-width: 1400px;
    margin: 0 auto;
  }

  .controls {
    display: grid;
    grid-template-columns: 1fr;
    gap: 1rem;
    margin-bottom: 2rem;
    padding: 1.5rem;
    background: var(--bg-secondary);
    border-radius: 0.5rem;
    border: 1px solid var(--border-color);
  }

  .control-group {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .control-group.full-width {
    grid-column: 1 / -1;
  }

  .control-group label {
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--text-primary);
  }

  .control-group select {
    padding: 0.5rem;
    border-radius: 0.375rem;
    border: 1px solid var(--border-color);
    background: var(--bg-primary);
    color: var(--text-primary);
    font-size: 0.875rem;
  }

  .file-selection {
    display: flex;
    flex-wrap: wrap;
    gap: 1rem;
    margin-top: 0.5rem;
  }

  .file-checkbox {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
    padding: 0.5rem 0.75rem;
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: 0.375rem;
    transition: all 0.2s;
  }

  .file-checkbox:hover {
    background: var(--bg-hover);
    border-color: var(--accent-primary);
  }

  .file-checkbox input[type="checkbox"] {
    cursor: pointer;
  }

  .file-checkbox-label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.875rem;
    color: var(--text-primary);
  }

  .file-color-indicator {
    width: 1rem;
    height: 1rem;
    border-radius: 0.25rem;
  }

  .file-selection-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-top: 0.5rem;
  }

  .action-btn {
    padding: 0.375rem 0.75rem;
    background: var(--accent-primary);
    color: white;
    border: none;
    border-radius: 0.375rem;
    font-size: 0.8rem;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.2s;
  }

  .action-btn:hover {
    background: var(--accent-hover);
  }

  .selected-count {
    margin-left: auto;
    font-size: 0.8rem;
    color: var(--text-secondary);
  }

  .legend {
    margin-bottom: 2rem;
    padding: 1rem;
    background: var(--bg-secondary);
    border-radius: 0.5rem;
    border: 1px solid var(--border-color);
  }

  .legend h3 {
    margin-bottom: 1rem;
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .legend-items {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
  }

  .legend-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .legend-color {
    width: 1.5rem;
    height: 0.75rem;
    border-radius: 0.125rem;
  }

  .legend-item span {
    font-size: 0.8rem;
    color: var(--text-secondary);
  }

  .window-info {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
    padding: 1rem;
    background: var(--bg-accent);
    border-radius: 0.5rem;
    border: 1px solid var(--border-color);
  }

  .window-position {
    font-size: 0.875rem;
    color: var(--text-primary);
  }

  .window-count {
    margin-left: 1rem;
    color: var(--text-secondary);
  }

  .window-navigation {
    display: flex;
    gap: 0.5rem;
  }

  .window-navigation button {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 0.375rem;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }

  .window-navigation button:not(:disabled) {
    background: var(--accent-primary);
    color: white;
  }

  .window-navigation button:not(:disabled):hover {
    background: var(--accent-hover);
  }

  .window-navigation button:disabled {
    background: var(--bg-hover);
    color: var(--text-tertiary);
    cursor: not-allowed;
  }

  .browser {
    position: relative;
    padding: 1.5rem 2.5rem;
    background: var(--bg-secondary);
    border-radius: 0.5rem;
    border: 1px solid var(--border-color);
    min-height: 300px;
  }

  .empty-state {
    text-align: center;
    padding: 4rem;
    color: var(--text-secondary);
  }

  .position-markers {
    position: relative;
    height: 30px;
    margin-bottom: 2rem;
    border-bottom: 2px solid var(--border-color-dark);
  }

  .marker {
    position: absolute;
    transform: translateX(-50%);
    font-size: 0.7rem;
    color: var(--text-secondary);
  }

  .marker-tick {
    width: 1px;
    height: 10px;
    background: var(--border-color-dark);
    margin: 0 auto 0.25rem;
  }

  .marker-label {
    white-space: nowrap;
  }

  .contigs-container {
    position: relative;
  }

  .contig-track {
    position: relative;
    margin-bottom: -1.0rem;
  }

  .contig {
    position: absolute;
    height: 5px;
    border-radius: 2px;
    cursor: pointer;
    border: 1px solid rgba(0, 0, 0, 0.2);
    transition: transform 0.1s;
  }

  .contig.hovered {
    transform: scaleY(1.2);
    z-index: 10;
  }

  .tooltip {
    position: fixed;
    bottom: 2rem;
    left: 50%;
    transform: translateX(-50%);
    padding: 1rem;
    background: var(--bg-primary);
    border: 2px solid var(--border-color-dark);
    border-radius: 0.5rem;
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
    z-index: 1000;
    min-width: 320px;
  }

  .tooltip-header {
    margin-bottom: 0.5rem;
    font-weight: 600;
    color: var(--accent-primary);
    font-size: 0.95rem;
  }

  .tooltip-body {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .tooltip-file {
    padding-bottom: 0.5rem;
    border-bottom: 1px solid var(--border-color);
  }

  .file-badge {
    padding: 0.25rem 0.5rem;
    border-radius: 0.25rem;
    font-size: 0.75rem;
    font-weight: 500;
    border: 1px solid;
    white-space: nowrap;
  }

  .tooltip-content {
    font-size: 0.8rem;
    color: var(--text-secondary);
    display: grid;
    gap: 0.25rem;
  }

  .tooltip-content strong {
    color: var(--text-primary);
  }

  @media (max-width: 768px) {
    .analysis-container {
      padding: 1rem;
    }

    .window-info {
      flex-direction: column;
      gap: 1rem;
    }

    .file-selection {
      flex-direction: column;
    }
  }
</style>