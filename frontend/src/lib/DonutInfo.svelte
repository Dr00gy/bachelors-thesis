<script lang="ts">
    import type { FileData, BackendMatch, DonutSegment, MatchedRecord } from './types';

    export let files: FileData[] = [];
    export let matches: BackendMatch[] = [];
    export let segments: DonutSegment[] = [];
    export let genomeSizes: Map<number, number> = new Map();
    export let totalGenomeSize = 0;
    export let filteredFlowPaths: any[] = [];
    export let showDuplicates = false;
  
    export let selectedQueryContigId = '';
    export let selectedGenome1 = '';
    export let selectedGenome2 = '';
    export let selectedChromosome = '';
    export let selectedGenomeForChromosome = '';
    
    export let availableQueryContigIds: number[] = [];
    export let availableGenomes: { value: string; label: string; color: string }[] = [];
    export let availableChromosomes: string[] = [];
    export let queryContigStats: Map<number, {
        totalOccurrences: number;
        genomeOccurrences: Map<number, number>;
        chromosomeOccurrences: Map<string, number>;
        maxConfidence: number;
    }> = new Map();

    export let clearAllFilters: () => void = () => {};

    /**
     * Pagination state for query contig overview
     */
    let overviewPage = 1;
    const overviewItemsPerPage = 10;

    /**
     * Pagination state for chromosome matches
     */
    let matchesPage = 1;
    const matchesItemsPerPage = 10;

    /**
     * Calculate paginated overview items
     */
    $: sortedQueryContigs = Array.from(queryContigStats.entries())
      .sort((a, b) => b[1].totalOccurrences - a[1].totalOccurrences);
    
    $: totalOverviewPages = Math.ceil(sortedQueryContigs.length / overviewItemsPerPage);
    $: paginatedOverview = sortedQueryContigs.slice(
      (overviewPage - 1) * overviewItemsPerPage,
      overviewPage * overviewItemsPerPage
    );

    /**
     * Calculate paginated match items
     */
    $: mergedMatches = (() => {
      const merged = new Map<number, Map<string, MatchedRecord>>();
      for (const match of matches) {
        const id = match.qry_contig_id;
        if (!merged.has(id)) {
          merged.set(id, new Map());
        }
        const recordMap = merged.get(id)!;

        for (const record of match.records) {
          const key = `${record.file_index}-${record.ref_contig_id}-${record.orientation}-${record.confidence}`;
          if (!recordMap.has(key)) {
            recordMap.set(key, record);
          }
        }
      }

      return Array.from(merged.entries()).map(([id, recordMap]) => ({
        qry_contig_id: id,
        records: Array.from(recordMap.values()).sort((a, b) => {
          if (a.file_index !== b.file_index) return a.file_index - b.file_index;
          return a.ref_contig_id - b.ref_contig_id;
        })
      }));
    })();

    $: totalMatchesPages = Math.ceil(mergedMatches.length / matchesItemsPerPage);
    $: paginatedMatches = mergedMatches.slice(
      (matchesPage - 1) * matchesItemsPerPage,
      matchesPage * matchesItemsPerPage
    );

    /**
     * Pagination controls
     */
    function goToOverviewPage(page: number) {
      overviewPage = Math.max(1, Math.min(page, totalOverviewPages));
    }

    function goToMatchesPage(page: number) {
      matchesPage = Math.max(1, Math.min(page, totalMatchesPages));
    }
</script>

<div class="info">
  <div class="section">
    <h2>Genomes ({files.length})</h2>
    {#each files as file, idx}
      <div class="file-item">
        <div class="color-box" style="background: {file.color}"></div>
        <span class="file-name">{file.name}</span>
        <span class="file-size">{(genomeSizes.get(idx) || 0).toLocaleString()} bp</span>
        <span class="file-pct">({segments[idx]?.percentage}%)</span>
      </div>
    {/each}
  </div>

  {#if matches.length > 0}
    <div class="section overview-section">
      <h2>Query Contig Overview ({queryContigStats.size} unique)</h2>
      <div class="overview-list">
        {#each paginatedOverview as [qryId, stat]}
          <div class="overview-item">
            <div class="overview-header">
              <strong>QryContig {qryId}</strong>
              <span class="overview-total">{stat.totalOccurrences} total occurrences</span>
              <span class="overview-confidence">Max conf: {stat.maxConfidence.toFixed(2)}</span>
            </div>
            
            <div class="genome-breakdown">
              <div class="breakdown-label">Per genome:</div>
              {#each Array.from(stat.genomeOccurrences.entries()) as [genomeIdx, count]}
                <span class="genome-badge" style="background: {files[genomeIdx]?.color}20; color: {files[genomeIdx]?.color}; border-color: {files[genomeIdx]?.color}">
                  {files[genomeIdx]?.name}: {count}x
                </span>
              {/each}
            </div>
            
            <div class="chromosome-breakdown">
              <div class="breakdown-label">Per chromosome:</div>
              <div class="chr-grid">
                {#each Array.from(stat.chromosomeOccurrences.entries()).sort((a, b) => {
                  const [aGenome, aChr] = a[0].split('-').map(Number);
                  const [bGenome, bChr] = b[0].split('-').map(Number);
                  return aGenome !== bGenome ? aGenome - bGenome : aChr - bChr;
                }) as [chrKey, count]}
                  {@const [genomeIdx, chrNum] = chrKey.split('-').map(Number)}
                  <span class="chr-mini-badge" style="background: {files[genomeIdx]?.color}20; color: {files[genomeIdx]?.color}; border-color: {files[genomeIdx]?.color}">
                    G{genomeIdx} Chr{chrNum}: {count}
                  </span>
                {/each}
              </div>
            </div>
          </div>
        {/each}
      </div>
      
      {#if totalOverviewPages > 1}
        <div class="pagination">
          <button 
            class="page-btn" 
            on:click={() => goToOverviewPage(1)}
            disabled={overviewPage === 1}
          >
            ««
          </button>
          <button 
            class="page-btn" 
            on:click={() => goToOverviewPage(overviewPage - 1)}
            disabled={overviewPage === 1}
          >
            «
          </button>
          <span class="page-info">
            ({overviewPage} / {totalOverviewPages})
          </span>
          <button 
            class="page-btn" 
            on:click={() => goToOverviewPage(overviewPage + 1)}
            disabled={overviewPage === totalOverviewPages}
          >
            »
          </button>
          <button 
            class="page-btn" 
            on:click={() => goToOverviewPage(totalOverviewPages)}
            disabled={overviewPage === totalOverviewPages}
          >
            »»
          </button>
        </div>
      {/if}
    </div>
  {/if}

  <div class="section filters-section">
    <h2>Filters</h2>
    <div class="filters-grid">
      
      <div class="filter-group">
        <label for="query-contig-filter">Query Contig ID:</label>
        <select id="query-contig-filter" bind:value={selectedQueryContigId}>
          <option value="">All Query Contigs</option>
          {#each availableQueryContigIds as id}
            <option value={id}>QryContig {id}</option>
          {/each}
        </select>
      </div>

      <div class="filter-group">
        <label for="genome1-filter">Genome 1:</label>
        <select id="genome1-filter" bind:value={selectedGenome1}>
          <option value="">All Genomes</option>
          {#each availableGenomes as genome}
            {#if genome.value !== selectedGenome2}
              <option value={genome.value}>{genome.label}</option>
            {/if}
          {/each}
        </select>
      </div>

      <div class="filter-group">
        <label for="genome2-filter">Genome 2 (optional):</label>
        <select id="genome2-filter" bind:value={selectedGenome2}>
          <option value="">Any Genome</option>
          {#each availableGenomes as genome}
            {#if genome.value !== selectedGenome1}
              <option value={genome.value}>{genome.label}</option>
            {/if}
          {/each}
        </select>
      </div>

      <div class="filter-group">
        <label for="genome-chromosome-filter">Genome for Chromosome:</label>
        <select id="genome-chromosome-filter" bind:value={selectedGenomeForChromosome}>
          <option value="">Select Genome</option>
          {#if selectedGenome1 !== ''}
            <option value={selectedGenome1}>
              {availableGenomes.find(g => g.value === selectedGenome1)?.label}
            </option>
          {/if}
          {#if selectedGenome2 !== '' && selectedGenome2 !== selectedGenome1}
            <option value={selectedGenome2}>
              {availableGenomes.find(g => g.value === selectedGenome2)?.label}
            </option>
          {/if}
          {#if selectedGenome1 === '' && selectedGenome2 === ''}
            {#each availableGenomes as genome}
              <option value={genome.value}>{genome.label}</option>
            {/each}
          {/if}
        </select>
      </div>

      <div class="filter-group">
        <label for="chromosome-filter">Chromosome:</label>
        <select id="chromosome-filter" bind:value={selectedChromosome} disabled={!selectedGenomeForChromosome}>
          <option value="">All Chromosomes</option>
          {#each availableChromosomes as chr}
            <option value={chr}>Chr {chr}</option>
          {/each}
        </select>
      </div>

      <div class="filter-group">
        <button on:click={clearAllFilters} class="clear-filters-btn">
          Clear All Filters
        </button>
      </div>
    </div>

    {#if selectedQueryContigId || selectedGenome1 || selectedChromosome}
      <div class="active-filters">
        <h3>Active Filters:</h3>
        <div class="filter-tags">
          {#if selectedQueryContigId}
            <span class="filter-tag">Query Contig: {selectedQueryContigId}</span>
          {/if}
          {#if selectedGenome1}
            <span class="filter-tag">
              Genome: {availableGenomes.find(g => g.value === selectedGenome1)?.label}
              {#if selectedGenome2}
                ↔ {availableGenomes.find(g => g.value === selectedGenome2)?.label}
              {/if}
            </span>
          {/if}
          {#if selectedChromosome && selectedGenomeForChromosome}
            <span class="filter-tag">
              Chromosome {selectedChromosome} on {availableGenomes.find(g => g.value === selectedGenomeForChromosome)?.label}
            </span>
          {/if}
        </div>
      </div>
    {/if}
  </div>

  {#if matches.length > 0}
    <div class="section">
      <h2>Chromosome Matches ({mergedMatches.length} unique contigs)</h2>
      <div class="match-list">
        {#each paginatedMatches as match}
          <div class="match-item">
            <div class="match-header">
              <strong>QryContig {match.qry_contig_id}</strong>
              <span class="occurrence-count">{match.records.length} occurrence{match.records.length !== 1 ? 's' : ''}</span>
            </div>
            <div class="occurrence-list">
              {#each match.records as record}
                <div class="occurrence">
                  <span class="file-badge" style="background: {files[record.file_index]?.color}20; color: {files[record.file_index]?.color}; border-color: {files[record.file_index]?.color}">
                    {files[record.file_index]?.name}
                  </span>
                  <span class="chr-info">Chr {record.ref_contig_id}</span>
                  <span class="orientation-badge" class:plus={record.orientation === '+'} class:minus={record.orientation === '-'}>
                    {record.orientation}
                  </span>
                  <span class="confidence-value">conf: {record.confidence.toFixed(2)}</span>
                </div>
              {/each}
            </div>
          </div>
        {/each}
      </div>
      
      {#if totalMatchesPages > 1}
        <div class="pagination">
          <button 
            class="page-btn" 
            on:click={() => goToMatchesPage(1)}
            disabled={matchesPage === 1}
          >
            ««
          </button>
          <button 
            class="page-btn" 
            on:click={() => goToMatchesPage(matchesPage - 1)}
            disabled={matchesPage === 1}
          >
            «
          </button>
          <span class="page-info">
            ({matchesPage} / {totalMatchesPages})
          </span>
          <button 
            class="page-btn" 
            on:click={() => goToMatchesPage(matchesPage + 1)}
            disabled={matchesPage === totalMatchesPages}
          >
            »
          </button>
          <button 
            class="page-btn" 
            on:click={() => goToMatchesPage(totalMatchesPages)}
            disabled={matchesPage === totalMatchesPages}
          >
            »»
          </button>
        </div>
      {/if}
    </div>
  {/if}

  <div class="section debug-info">
    <h2>Debug Info</h2>
    <div class="debug-item">
      <strong>Total Genome Size:</strong> {totalGenomeSize.toLocaleString()} bp
    </div>
    <div class="debug-item">
      <strong>Flow Paths:</strong> {filteredFlowPaths.length} {showDuplicates ? '(self-flow)' : '(cross-genome)'}
    </div>
    <div class="debug-item">
      <strong>Show Self-Flow:</strong> {showDuplicates ? 'ON' : 'OFF'}
    </div>
    <div class="debug-item">
      <strong>Active Filters:</strong> 
      {selectedQueryContigId ? 'QueryContig ' + selectedQueryContigId + ' ' : ''}
      {selectedGenome1 ? 'Genome1:' + selectedGenome1 + ' ' : ''}
      {selectedGenome2 ? 'Genome2:' + selectedGenome2 + ' ' : ''}
      {selectedChromosome ? 'Chr:' + selectedChromosome + ' ' : ''}
      {!selectedQueryContigId && !selectedGenome1 && !selectedChromosome ? 'None' : ''}
    </div>
  </div>
</div>

<style>
  .info {
    flex: 1;
    min-width: 280px;
  }

  .section {
    margin-bottom: 1.5rem;
    padding: clamp(0.75rem, 1.5vw, 1rem);
    background: var(--bg-secondary);
    border-radius: 0.5rem;
    border: 1px solid var(--border-color);
  }

  h2 {
    font-size: clamp(0.95rem, 1.4vw, 1rem);
    font-weight: 600;
    margin-bottom: 0.75rem;
    color: var(--text-primary);
  }

  h3 {
    font-size: clamp(0.85rem, 1.3vw, 0.95rem);
    font-weight: 600;
    margin-bottom: 0.5rem;
    color: var(--text-primary);
  }

  .overview-section {
    background: var(--bg-secondary);
    border-color: var(--border-color);
  }

  .overview-list {
    max-height: 500px;
    overflow-y: auto;
  }

  .overview-item {
    margin-bottom: 1rem;
    padding: 1rem;
    background: var(--bg-primary);
    border-radius: 0.5rem;
    border: 1px solid var(--border-color);
  }

  .overview-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 0.75rem;
    flex-wrap: wrap;
  }

  .overview-header strong {
    color: var(--accent-primary);
    font-size: 0.875rem;
  }

  .overview-total {
    padding: 0.25rem 0.5rem;
    background: var(--accent-light);
    color: var(--accent-primary);
    border-radius: 0.25rem;
    font-size: 0.7rem;
    font-weight: 600;
    white-space: nowrap;
  }

  .overview-confidence {
    padding: 0.25rem 0.5rem;
    background: rgba(16, 185, 129, 0.2);
    color: var(--success);
    border-radius: 0.25rem;
    font-size: 0.7rem;
    font-weight: 600;
    white-space: nowrap;
  }

  .genome-breakdown,
  .chromosome-breakdown {
    margin-bottom: 0.5rem;
    font-size: 0.75rem;
  }

  .breakdown-label {
    font-weight: 500;
    color: var(--text-secondary);
    margin-bottom: 0.25rem;
  }

  .genome-breakdown {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
    align-items: center;
  }

  .genome-badge {
    padding: 0.25rem 0.5rem;
    border-radius: 0.25rem;
    font-size: 0.7rem;
    font-weight: 600;
    border: 1px solid;
    white-space: nowrap;
  }

  .chr-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    margin-top: 0.25rem;
  }

  .chr-mini-badge {
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
    font-size: 0.65rem;
    font-weight: 500;
    border: 1px solid;
    white-space: nowrap;
  }

  .pagination {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    margin-top: 1rem;
    padding-top: 1rem;
    border-top: 1px solid var(--border-color);
  }

  .page-btn {
    padding: 0.5rem 0.75rem;
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: 0.375rem;
    color: var(--text-primary);
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
    min-width: 2.5rem;
  }

  .page-btn:hover:not(:disabled) {
    background: var(--accent-primary);
    color: white;
    border-color: var(--accent-primary);
  }

  .page-btn:disabled {
    background: var(--bg-hover);
    color: var(--text-tertiary);
    cursor: not-allowed;
    opacity: 0.5;
  }

  .page-info {
    padding: 0.5rem 0.75rem;
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--text-primary);
  }

  .filters-section {
    background: var(--bg-accent);
    border-color: var(--border-color);
  }

  .filters-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .filter-group {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .filter-group label {
    font-size: 0.75rem;
    font-weight: 500;
    color: var(--text-primary);
  }

  .filter-group select {
    padding: 0.5rem;
    border: 1px solid var(--border-color-dark);
    border-radius: 0.375rem;
    font-size: 0.8rem;
    background: var(--bg-primary);
    color: var(--text-primary);
    width: 100%;
    box-sizing: border-box;
  }

  .filter-group select:disabled {
    background: var(--bg-hover);
    color: var(--text-tertiary);
    cursor: not-allowed;
  }

  .clear-filters-btn {
    padding: 0.5rem 1rem;
    background: var(--accent-primary);
    color: white;
    border: none;
    border-radius: 0.375rem;
    font-size: 0.8rem;
    cursor: pointer;
    margin-top: 1.25rem;
    width: 100%;
    transition: background 0.2s;
  }

  .clear-filters-btn:hover {
    background: var(--accent-hover);
  }

  .active-filters {
    margin-top: 1rem;
    padding-top: 1rem;
    border-top: 1px solid var(--border-color);
  }

  .filter-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  .filter-tag {
    padding: 0.25rem 0.5rem;
    background: var(--accent-primary);
    color: white;
    border-radius: 0.25rem;
    font-size: 0.7rem;
    font-weight: 500;
    white-space: nowrap;
  }

  .file-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
    font-size: 0.875rem;
    flex-wrap: nowrap;
    min-width: 0;
  }

  .color-box {
    width: 1rem;
    height: 1rem;
    border-radius: 0.25rem;
    flex-shrink: 0;
  }

  .file-name {
    font-weight: 500;
    color: var(--text-primary);
    flex: 1 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .file-size,
  .file-pct {
    flex-shrink: 0;
    color: var(--text-secondary);
    font-size: 0.75rem;
    white-space: nowrap;
  }

  .match-list {
    max-height: 400px;
    overflow-y: auto;
  }

  .match-item {
    font-size: 0.8rem;
    margin-bottom: 0.75rem;
    padding: 0.75rem;
    background: var(--bg-primary);
    border-radius: 0.5rem;
    border: 1px solid var(--border-color);
  }

  .file-badge {
    padding: 0.25rem 0.5rem;
    margin-right: 0.5rem;
    border-radius: 0.25rem;
    font-size: 0.75rem;
    font-weight: 500;
    border: 1px solid;
    white-space: nowrap;
  }

  .occurrence {
    margin-top: 1.0rem;
  }

  .occurrence-list {
    margin-bottom: 0.5rem;
  }

  .debug-info {
    background: var(--error-bg);
    border-color: var(--error-border);
  }

  .debug-item {
    font-size: 0.75rem;
    margin-bottom: 0.25rem;
    color: var(--text-primary);
  }

  .debug-item strong {
    color: var(--text-primary);
  }

  @media (max-width: 1024px) {
    .overview-list {
      max-height: 420px;
    }
  }

  @media (max-width: 768px) {
    .filters-grid {
      grid-template-columns: 1fr;
    }
    .file-item {
      grid-template-columns: auto 1fr;
      grid-auto-rows: auto;
      row-gap: 0.25rem;
    }
    .file-size,
    .file-pct {
      grid-column: 2 / -1;
    }
    .match-list {
      max-height: 360px;
    }
  }

  @media (max-width: 520px) {
    .section {
      padding: 0.75rem;
    }
    h2 { font-size: 0.9rem; }
    h3 { font-size: 0.85rem; }
    .genome-badge,
    .file-badge,
    .filter-tag {
      font-size: 0.7rem;
    }
    .overview-header strong {
      font-size: 0.8rem;
    }
    .overview-list {
      max-height: 320px;
    }
    .page-btn {
      padding: 0.4rem 0.6rem;
      font-size: 0.8rem;
      min-width: 2rem;
    }
    .page-info {
      font-size: 0.8rem;
      padding: 0.4rem 0.6rem;
    }
  }

  @media (max-width: 380px) {
    .overview-total,
    .overview-confidence {
      font-size: 0.65rem;
      padding: 0.2rem 0.4rem;
    }
    .filter-group select {
      font-size: 0.75rem;
      padding: 0.45rem;
    }
    .clear-filters-btn {
      font-size: 0.75rem;
    }
  }

  @media (max-width: 300px) {
    .file-item {
      flex-wrap: wrap;
      gap: 0.25rem 0.5rem;
    }
    .file-name {
      flex: 1 1 100%;
    }
  }
</style>