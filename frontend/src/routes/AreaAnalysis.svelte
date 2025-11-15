<script lang="ts">
  import type { BackendMatch } from '../lib/bincodeDecoder';
  import type { FileData } from '../lib/types';
  import { searchStore } from '../lib/searchStore';
  import { areaAnalysisFilterState } from '../lib/filterStateStore';

  /**
   * Component props
   */
  export let matches: BackendMatch[] = [];
  export let files: FileData[] = [];

  /**
   * Filter and display state from store
   */
  let selectedFiles: number[] = [0];
  let selectedChromosome = 1;
  let windowSize = 100000;
  let currentWindowIndex = 0;
  let hoveredContig: any = null;
  
  /**
   * Search state from store
   */
  let searchQuery = '';
  
  /**
   * Subscribe to stores
   */
  const unsubscribeSearch = searchStore.subscribe(state => {
    searchQuery = state.areaSearchQuery;
  });

  const unsubscribeFilter = areaAnalysisFilterState.subscribe(state => {
    selectedFiles = state.selectedFiles;
    selectedChromosome = state.selectedChromosome;
    windowSize = state.windowSize;
    currentWindowIndex = state.currentWindowIndex;
  });

  /**
   * Update search store when local value changes
   */
  $: if (searchQuery !== $searchStore.areaSearchQuery) {
    searchStore.update(state => ({ ...state, areaSearchQuery: searchQuery }));
  }

  /**
   * Update filter store when local values change
   */
  $: if (JSON.stringify(selectedFiles) !== JSON.stringify($areaAnalysisFilterState.selectedFiles)) {
    areaAnalysisFilterState.update(state => ({ 
      ...state, 
      selectedFiles 
    }));
  }

  $: if (selectedChromosome !== $areaAnalysisFilterState.selectedChromosome) {
    areaAnalysisFilterState.update(state => ({ 
      ...state, 
      selectedChromosome 
    }));
  }

  $: if (windowSize !== $areaAnalysisFilterState.windowSize) {
    areaAnalysisFilterState.update(state => ({ 
      ...state, 
      windowSize 
    }));
  }

  $: if (currentWindowIndex !== $areaAnalysisFilterState.currentWindowIndex) {
    areaAnalysisFilterState.update(state => ({ 
      ...state, 
      currentWindowIndex 
    }));
  }

  /**
   * Window jump editing state
   */
  let editingWindowPage = false;
  let windowPageInput = '';

  /**
   * Track filtered windows when searching
   */
  let filteredWindows: number[] = [];
  let isSearching = false;

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
        
        let hasOverlap = false;
        for (const existingRecord of track) {
          if (record.ref_start_pos < existingRecord.ref_end_pos && 
              record.ref_end_pos > existingRecord.ref_start_pos) {
            hasOverlap = true;
            break;
          }
        }
        
        if (!hasOverlap) {
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
    resetSearch();
  }

  /**
   * Selects all available files
   */
  function selectAllFiles() {
    selectedFiles = files.map((_, idx) => idx);
    currentWindowIndex = 0;
    resetSearch();
  }

  /**
   * Resets file selection to empty array (complete clear)
   */
  function clearFileSelection() {
    selectedFiles = [];
    currentWindowIndex = 0;
    resetSearch();
  }

  /**
   * Resets search state
   */
  function resetSearch() {
    searchQuery = '';
    isSearching = false;
    filteredWindows = [];
    currentWindowIndex = 0;
  }

  /**
   * Finds windows that contain a specific contig ID
   */
  function findWindowsWithContig(contigId: number, records: any[], chromosomeRange: any, windowSize: number): number[] {
    const contigRecords = records.filter(record => record.qry_contig_id === contigId);
    if (contigRecords.length === 0) return [];

    const windowsWithContig = new Set<number>();
    
    for (const record of contigRecords) {
      const startWindow = Math.floor((record.ref_start_pos - chromosomeRange.min) / windowSize);
      const endWindow = Math.floor((record.ref_end_pos - chromosomeRange.min) / windowSize);
      
      for (let windowIndex = startWindow; windowIndex <= endWindow; windowIndex++) {
        if (windowIndex >= 0) {
          windowsWithContig.add(windowIndex);
        }
      }
    }
    
    return Array.from(windowsWithContig).sort((a, b) => a - b);
  }

  /**
   * Finds the first occurrence of a contig ID and jumps to its window
   */
  function jumpToContig(contigId: number) {
    const records = getRecordsForChromosome(matches, selectedFiles, selectedChromosome);
    const contigRecords = records.filter(record => record.qry_contig_id === contigId);
    
    if (contigRecords.length === 0) return;
    filteredWindows = findWindowsWithContig(contigId, records, chromosomeRange, windowSize);
    isSearching = true;
    
    if (filteredWindows.length > 0) {
      currentWindowIndex = filteredWindows[0];
    }
  }

  /**
   * Handles search submission
   */
  function handleSearch() {
    if (!searchQuery.trim()) {
      resetSearch();
      return;
    }
    
    const contigId = parseInt(searchQuery.trim());
    if (!isNaN(contigId)) {
      jumpToContig(contigId);
    }
  }

  /**
   * Handles keydown in search input
   */
  function handleSearchKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      handleSearch();
    } else if (e.key === 'Escape') {
      resetSearch();
    }
  }

  /**
   * Clears search and shows all records
   */
  function clearSearch() {
    resetSearch();
  }

  /**
   * Navigate to next/previous window considering filtered windows
   */
  function goToNextWindow() {
    if (isSearching && filteredWindows.length > 0) {
      const currentIndexInFiltered = filteredWindows.indexOf(currentWindowIndex);
      if (currentIndexInFiltered < filteredWindows.length - 1) {
        currentWindowIndex = filteredWindows[currentIndexInFiltered + 1];
      }
    } else {
      currentWindowIndex++;
    }
  }

  function goToPrevWindow() {
    if (isSearching && filteredWindows.length > 0) {
      const currentIndexInFiltered = filteredWindows.indexOf(currentWindowIndex);
      if (currentIndexInFiltered > 0) {
        currentWindowIndex = filteredWindows[currentIndexInFiltered - 1];
      }
    } else {
      currentWindowIndex--;
    }
  }

  /**
   * Reactive computed values for chromosome data and visualization
   */
  $: chromosomeRecords = getRecordsForChromosome(matches, selectedFiles, selectedChromosome);
  $: filteredChromosomeRecords = chromosomeRecords.filter(record => {
    if (!searchQuery || !isSearching) return true;
    const query = searchQuery.toLowerCase();
    return record.qry_contig_id.toString().includes(query);
  });
  $: chromosomeRange = getChromosomeRange(chromosomeRecords);
  $: chromosomeRefLen = chromosomeRecords.length > 0 ? chromosomeRecords[0].ref_len : windowSize;
  
  $: totalWindows = Math.ceil(chromosomeRefLen / windowSize);
  $: effectiveTotalWindows = isSearching ? filteredWindows.length : totalWindows;
  $: effectiveCurrentWindowIndex = isSearching ? 
    (filteredWindows.indexOf(currentWindowIndex) + 1 || 1) : 
    (currentWindowIndex + 1);
  
  $: windowStart = chromosomeRange.min + (currentWindowIndex * windowSize);
  $: windowEnd = Math.min(windowStart + windowSize, chromosomeRefLen);
  
  $: stackedContigs = stackContigs(filteredChromosomeRecords, windowStart, windowEnd);
  $: uniqueContigs = Array.from(new Set(filteredChromosomeRecords.map(r => r.qry_contig_id))).sort((a, b) => a - b);
  
  $: canGoPrev = isSearching ? 
    filteredWindows.indexOf(currentWindowIndex) > 0 : 
    currentWindowIndex > 0;
  $: canGoNext = isSearching ? 
    filteredWindows.indexOf(currentWindowIndex) < filteredWindows.length - 1 : 
    currentWindowIndex < totalWindows - 1;

  /**
   * Available chromosomes (1-24)
   */
  const chromosomes = Array.from({ length: 24 }, (_, i) => i + 1);

  /**
   * Resets window position when chromosome changes
   */
  function handleChromosomeChange() {
    currentWindowIndex = 0;
    resetSearch();
  }

  /**
   * Window jump functions
   */
  function startEditingWindowPage() {
    editingWindowPage = true;
    windowPageInput = effectiveCurrentWindowIndex.toString();
  }

  function submitWindowPageJump() {
    const pageNum = parseInt(windowPageInput);
    if (!isNaN(pageNum)) {
      if (isSearching && filteredWindows.length > 0) {
        const newFilteredIndex = Math.max(0, Math.min(pageNum - 1, filteredWindows.length - 1));
        currentWindowIndex = filteredWindows[newFilteredIndex];
      } else {
        const newIndex = Math.max(0, Math.min(pageNum - 1, totalWindows - 1));
        currentWindowIndex = newIndex;
      }
    }
    editingWindowPage = false;
  }

  function handleWindowPageKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      submitWindowPageJump();
    } else if (e.key === 'Escape') {
      editingWindowPage = false;
    }
  }

  /**
   * Cleanup store subscriptions
   */
  import { onDestroy } from 'svelte';
  onDestroy(() => {
    unsubscribeSearch();
    unsubscribeFilter();
  });
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
      <div class="legend-header">
        <h3>
          {#if isSearching}
            Showing Contig {searchQuery} ({filteredWindows.length} windows)
          {:else}
            Query Contigs ({uniqueContigs.length})
          {/if}
        </h3>
        <div class="search-bar">
          <input
            type="text"
            placeholder="Search contig ID and press Enter..."
            bind:value={searchQuery}
            on:keydown={handleSearchKeydown}
            class="search-input"
          />
        </div>
      </div>
      {#if !isSearching}
        <div class="legend-items">
          {#each uniqueContigs as contigId}
            <div class="legend-item">
              <div class="legend-color" style="background: {generateContigColor(contigId)}"></div>
              <span>QryContig {contigId}</span>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}

  <!-- Genomic window navigation and information -->
  <div class="window-info">
    <div class="window-position">
      <strong>Window:</strong> {windowStart.toLocaleString()} - {windowEnd.toLocaleString()} bp
      {#if isSearching}
        <span class="search-indicator">(Searching: Contig {searchQuery})</span>
      {/if}
      {#if editingWindowPage}
        <input
          type="text"
          class="window-page-input"
          bind:value={windowPageInput}
          on:keydown={handleWindowPageKeydown}
          on:blur={submitWindowPageJump}
          on:focus
        />
      {:else}
        <span 
          class="window-count" 
          on:dblclick={startEditingWindowPage}
          role="button"
          tabindex="0"
          on:keydown={(e) => { // AIRA role, focused, key sup
            if (e.key === 'Enter' || e.key === ' ') {
              startEditingWindowPage();
            }
          }}
        >
          ({effectiveCurrentWindowIndex} / {effectiveTotalWindows})
          {#if isSearching && filteredWindows.length > 0}
            <span class="filtered-pages">(Filtered)</span>
          {/if}
        </span>
      {/if}
    </div>
    <div class="window-navigation">
      <button on:click={goToPrevWindow} disabled={!canGoPrev}>
        ← Previous
      </button>
      <button on:click={goToNextWindow} disabled={!canGoNext}>
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
        {#if isSearching}
          No occurrences of Contig {searchQuery} in this window
        {:else}
          No mappings in this window. Use navigation buttons to explore other regions.
        {/if}
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
                class:search-match={isSearching && record.qry_contig_id.toString() === searchQuery}
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

  .legend-header {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    margin-bottom: 1rem;
  }

  .legend h3 {
    margin: 0;
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
    cursor: pointer;
    user-select: none;
    padding: 0.25rem 0.5rem;
    border-radius: 0.25rem;
    transition: background 0.2s;
  }

  .window-count:hover {
    background: rgba(100, 123, 212, 0.1);
    color: white;
    border-color: var(--accent-primary);
    transform: translateY(-1px);
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
  }

  .window-page-input {
    width: 5rem;
    padding: 0.25rem 0.5rem;
    margin-left: 0.5rem;
    text-align: center;
    font-size: 0.875rem;
    border: 2px solid var(--accent-primary);
    border-radius: 0.25rem;
    background: var(--bg-primary);
    color: var(--text-primary);
  }

  .search-bar {
    position: relative;
    max-width: 100%;
        width: 100%;
    box-sizing: border-box;
  }

  .search-input {
    width: 100%;
    padding: 0.5rem 2.5rem 0.5rem 0.75rem;
    font-size: 0.8rem;
    border: 1px solid var(--border-color-dark);
    border-radius: 0.375rem;
    background: var(--bg-primary);
    color: var(--text-primary);
    transition: border-color 0.2s;
    box-sizing: border-box;
  }

  .search-input:focus {
    outline: none;
    border-color: var(--accent-primary);
    box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.1);
  }

  .search-input::placeholder {
    color: var(--text-tertiary);
  }

    .search-indicator {
    margin-left: 0.5rem;
    font-size: 0.8rem;
    color: var(--accent-primary);
    font-weight: 500;
  }

  .filtered-pages {
    margin-left: 0.25rem;
    font-size: 0.7rem;
    color: var(--accent-primary);
    font-style: italic;
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
    display: flex;
    flex-direction: column;
  }

  .contig-track {
    position: relative;
    margin-bottom: -1.0rem;
  }

  .contig {
    top: 0;
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