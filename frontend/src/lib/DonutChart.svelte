<script lang="ts">
  
  /**
   * Represents file metadata for visualization
   */
  interface FileData {
    name: string;
    rows: number;
    color: string;
  }

  /**
   * Represents a single matched record position
   */
  interface MatchedRecord {
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
   * Represents a complete backend match with multiple records
   */
  interface BackendMatch {
    qry_contig_id: number;
    file_indices: number[];
    records: MatchedRecord[];
  }

  /**
   * Represents a donut chart segment
   */
  interface DonutSegment {
    name: string;
    rows: number;
    color: string;
    index: number;
    genomeSize: number;
    dashArray: string;
    dashOffset: number;
    percentage: string;
    showLabel: boolean;
    showChromosomes: boolean;
    startAngle: number;
    endAngle: number;
    angleRange: number;
  }

  /**
   * Represents a flow path between two points
   */
  interface FlowPath {
    path: string;
    p1: { x: number; y: number };
    p2: { x: number; y: number };
    fromOrientation: string;
    toOrientation: string;
    color: string;
    opacity: number;
    width: number;
    fromChromosome: number;
    toChromosome: number;
    confidence: number;
    fromFileIndex: number;
    toFileIndex: number;
    isSameGenome: boolean;
    qryContigId: number;
    fromRecord: MatchedRecord;
    toRecord: MatchedRecord;
  }

  /**
   * Represents chromosome division markers
   */
  interface ChromosomeDivision {
    chromosome: number;
    startAngle: number;
    endAngle: number;
    midAngle: number;
  }

  /**
   * Component props
   */
  export let files: FileData[] = [];
  export let matches: BackendMatch[] = [];
  export let showDuplicates = false;

  /**
   * Filter state
   */
  let selectedQueryContigId = '';
  let selectedGenome1 = '';
  let selectedGenome2 = '';
  let selectedChromosome = '';
  let selectedGenomeForChromosome = '';

  /**
   * Visualization scaling
   */
  export let scale = 1.0;
  const baseRadius = 80;
  const baseStrokeWidth = 20;

  /**
   * Computed visualization properties
   */
  $: radius = baseRadius * scale;
  $: strokeWidth = baseStrokeWidth * scale;
  $: centerX = 200;
  $: centerY = 200;
  $: circumference = 2 * Math.PI * (radius - strokeWidth / 2);
  $: showChromosomes = scale >= 1.2;

  /**
   * Calculates maximum confidence value for normalization
   */
  $: maxConfidence = (() => {
    let max = 0;
    for (const match of matches) {
      for (const record of match.records) {
        if (record.confidence > max) {
          max = record.confidence;
        }
      }
    }
    return max || 1;
  })();

  /**
   * Statistics per query contig ID
   */
  $: queryContigStats = (() => {
    const stats = new Map<number, {
      totalOccurrences: number;
      genomeOccurrences: Map<number, number>;
      chromosomeOccurrences: Map<string, number>;
      maxConfidence: number;
    }>();

    for (const match of matches) {
      const qryId = match.qry_contig_id;
      
      if (!stats.has(qryId)) {
        stats.set(qryId, {
          totalOccurrences: 0,
          genomeOccurrences: new Map(),
          chromosomeOccurrences: new Map(),
          maxConfidence: 0
        });
      }

      const stat = stats.get(qryId)!;

      for (const record of match.records) {
        stat.totalOccurrences++;
        
        const genomeCount = stat.genomeOccurrences.get(record.file_index) || 0;
        stat.genomeOccurrences.set(record.file_index, genomeCount + 1);
        
        const chrKey = `${record.file_index}-${record.ref_contig_id}`;
        const chrCount = stat.chromosomeOccurrences.get(chrKey) || 0;
        stat.chromosomeOccurrences.set(chrKey, chrCount + 1);
        
        if (record.confidence > stat.maxConfidence) {
          stat.maxConfidence = record.confidence;
        }
      }
    }

    return stats;
  })();

  /**
   * Available query contig IDs from matches
   */
  $: availableQueryContigIds = (() => {
    const ids = new Set<number>();
    matches.forEach(match => ids.add(match.qry_contig_id));
    return Array.from(ids).sort((a, b) => a - b);
  })();

  /**
   * Available genomes for filtering
   */
  $: availableGenomes = files.map((file, index) => ({
    value: index.toString(),
    label: file.name,
    color: file.color
  }));

  /**
   * Available chromosomes (1-22 + X)
   */
  $: availableChromosomes = Array.from({ length: 23 }, (_, i) => (i + 1).toString());

  /**
   * Calculates genome sizes from RefLen fields
   */
  $: genomeSizes = (() => {
    const chromosomesByGenome = new Map<number, Map<number, number>>();
    
    for (const match of matches) {
      for (const record of match.records) {
        const fileIdx = record.file_index;
        const chrNum = record.ref_contig_id;
        
        if (!chromosomesByGenome.has(fileIdx)) {
          chromosomesByGenome.set(fileIdx, new Map());
        }
        
        const chromosomes = chromosomesByGenome.get(fileIdx)!;
        
        if (!chromosomes.has(chrNum)) {
          chromosomes.set(chrNum, record.ref_len);
        }
      }
    }
    
    const sizes = new Map<number, number>();
    for (const [fileIdx, chromosomes] of chromosomesByGenome.entries()) {
      const totalSize = Array.from(chromosomes.values()).reduce((sum, len) => sum + len, 0);
      sizes.set(fileIdx, totalSize);
    }
    
    files.forEach((_, idx) => {
      if (!sizes.has(idx) || sizes.get(idx) === 0) {
        sizes.set(idx, 100000);
      }
    });
    
    return sizes;
  })();

  /**
   * Total genome size for percentage calculations
   */
  $: totalGenomeSize = Array.from(genomeSizes.values()).reduce((sum, size) => sum + size, 0);

  /**
   * Donut chart segments with calculated positions
   */
  $: segments = (() => {
    if (totalGenomeSize === 0) return [];
    
    let offset = 0;
    return files.map((file, idx) => {
      const genomeSize = genomeSizes.get(idx) || 1;
      const pct = genomeSize / totalGenomeSize;
      const length = pct * circumference;
      const startAngle = (offset / circumference) * 360 - 90;
      const endAngle = ((offset + length) / circumference) * 360 - 90;

      const segment = {
        ...file,
        index: idx,
        genomeSize,
        dashArray: `${length} ${circumference}`,
        dashOffset: -offset,
        percentage: (pct * 100).toFixed(1),
        showLabel: pct >= 0.01,
        showChromosomes: pct >= 0.20,
        startAngle,
        endAngle,
        angleRange: endAngle - startAngle
      };
      offset += length;
      return segment;
    });
  })();

  /**
   * Gets chromosome divisions for a segment
   * @param seg - Donut segment to divide
   * @returns Array of chromosome divisions
   */
  function getChromosomeDivisions(seg: DonutSegment): ChromosomeDivision[] {
    const divisions = [];
    const segmentRange = seg.endAngle - seg.startAngle;
    
    for (let i = 0; i < 23; i++) {
      const chrStart = seg.startAngle + (segmentRange * i / 23);
      const chrEnd = seg.startAngle + (segmentRange * (i + 1) / 23);
      const chrMid = (chrStart + chrEnd) / 2;
      
      divisions.push({
        chromosome: i + 1,
        startAngle: chrStart,
        endAngle: chrEnd,
        midAngle: chrMid
      });
    }
    
    return divisions;
  }

  /**
   * Gets angle for specific chromosome position
   * @param fileIndex - File/genome index
   * @param chromosome - Chromosome number
   * @param position - Position in chromosome
   * @returns Angle in degrees
   */
  function getChromosomeAngle(fileIndex: number, chromosome: number, position: 'start' | 'mid' | 'end'): number {
    const seg = segments[fileIndex];
    if (!seg) return 0;
    
    const divisions = getChromosomeDivisions(seg);
    const chrDiv = divisions.find(d => d.chromosome === chromosome);
    
    if (!chrDiv) return seg.startAngle;
    
    if (position === 'start') return chrDiv.startAngle;
    if (position === 'end') return chrDiv.endAngle;
    return chrDiv.midAngle;
  }

  /**
   * Calculates position within chromosome as ratio
   * @param refStartPos - Start position in reference
   * @param refEndPos - End position in reference
   * @param refLen - Chromosome length
   * @returns Position ratio (0-1)
   */
  function getPositionInChromosome(refStartPos: number, refEndPos: number, refLen: number): number {
    const avgPos = (refStartPos + refEndPos) / 2;
    return Math.min(1, Math.max(0, avgPos / refLen));
  }

  /**
   * Gets angle for record within its chromosome
   * @param fileIndex - File/genome index
   * @param chromosome - Chromosome number
   * @param refStartPos - Start position
   * @param refEndPos - End position
   * @param refLen - Chromosome length
   * @returns Angle in degrees
   */
  function getAngleInChromosome(
    fileIndex: number, 
    chromosome: number, 
    refStartPos: number, 
    refEndPos: number,
    refLen: number
  ): number {
    return getChromosomeAngle(fileIndex, chromosome, 'mid');
  }

  /**
   * Calculates radial point coordinates
   * @param angle - Angle in degrees
   * @param radiusOffset - Radius offset from base
   * @returns Point coordinates
   */
  function getRadialPoint(angle: number, radiusOffset: number = 0) {
    const rad = (angle * Math.PI) / 180;
    const r = radius + radiusOffset;
    return {
      x: centerX + r * Math.cos(rad),
      y: centerY + r * Math.sin(rad)
    };
  }

  /**
   * Gets point on donut circumference
   * @param angle - Angle in degrees
   * @param radiusOffset - Radius offset
   * @returns Point coordinates
   */
  function getPointOnDonut(angle: number, radiusOffset: number = 0) {
    return getRadialPoint(angle, radiusOffset);
  }

  /**
   * Creates curved flow path between two points
   * @param fromAngle - Starting angle
   * @param toAngle - Ending angle
   * @param intensity - Path intensity for styling
   * @param fromOrientation - Starting orientation
   * @param toOrientation - Ending orientation
   * @returns Flow path data
   */
  function createFlowPath(
    fromAngle: number, 
    toAngle: number, 
    intensity: number,
    fromOrientation: string,
    toOrientation: string
  ): { path: string; p1: { x: number; y: number }; p2: { x: number; y: number }; fromOrientation: string; toOrientation: string } {
    const p1 = getPointOnDonut(fromAngle, -strokeWidth / 2);
    const p2 = getPointOnDonut(toAngle, -strokeWidth / 2);
    
    const midAngle = (fromAngle + toAngle) / 2;
    const angleDiff = Math.abs(toAngle - fromAngle);
    const controlDist = radius * (0.85 + Math.min(angleDiff / 180, 1) * 0.15);
    const cp = getPointOnDonut(midAngle, -controlDist);

    return {
      path: `M ${p1.x} ${p1.y} Q ${cp.x} ${cp.y} ${p2.x} ${p2.y}`,
      p1,
      p2,
      fromOrientation,
      toOrientation
    };
  }

  /**
   * Generated flow paths between all record pairs
   */
$: flowPaths = (() => {
    const paths: FlowPath[] = [];
    const drawnSelfFlowPaths = new Set<string>();
    
    for (const match of matches) {
      if (match.records.length < 2) continue;
      
      for (let i = 0; i < match.records.length; i++) {
        for (let j = i + 1; j < match.records.length; j++) {
          const fromRecord = match.records[i];
          const toRecord = match.records[j];
          
          if (fromRecord.file_index >= files.length || toRecord.file_index >= files.length) {
            continue;
          }
          
          const isSameGenome = fromRecord.file_index === toRecord.file_index;
          
          if (isSameGenome) {
            const pathKey = `${match.qry_contig_id}-${fromRecord.file_index}-${Math.min(fromRecord.ref_contig_id, toRecord.ref_contig_id)}-${Math.max(fromRecord.ref_contig_id, toRecord.ref_contig_id)}`;
            
            if (drawnSelfFlowPaths.has(pathKey)) {
              continue;
            }
            drawnSelfFlowPaths.add(pathKey);
          }
          
          const fromAngle = getAngleInChromosome(
            fromRecord.file_index,
            fromRecord.ref_contig_id,
            fromRecord.ref_start_pos,
            fromRecord.ref_end_pos,
            fromRecord.ref_len
          );
          
          const toAngle = getAngleInChromosome(
            toRecord.file_index,
            toRecord.ref_contig_id,
            toRecord.ref_start_pos,
            toRecord.ref_end_pos,
            toRecord.ref_len
          );
          
          const avgConfidence = (fromRecord.confidence + toRecord.confidence) / 2;
          const normalizedConfidence = avgConfidence / maxConfidence;
          const opacity = 0.1 + (normalizedConfidence * 0.9);
          
          const flowData = createFlowPath(
            fromAngle, 
            toAngle, 
            normalizedConfidence,
            fromRecord.orientation,
            toRecord.orientation
          );
          
          paths.push({
            ...flowData,
            color: files[fromRecord.file_index]?.color || '#888',
            opacity: opacity,
            width: (1 + normalizedConfidence * 2) * scale,
            fromChromosome: fromRecord.ref_contig_id,
            toChromosome: toRecord.ref_contig_id,
            confidence: Math.max(fromRecord.confidence, toRecord.confidence),
            fromFileIndex: fromRecord.file_index,
            toFileIndex: toRecord.file_index,
            isSameGenome: isSameGenome,
            qryContigId: match.qry_contig_id,
            fromRecord,
            toRecord
          });
        }
      }
    }
    
    return paths;
  })();

  /**
   * Flow paths filtered by current filter settings
   */
  $: filteredFlowPaths = (() => {
    let filtered = flowPaths;

    if (showDuplicates) {
      filtered = filtered.filter(path => path.isSameGenome);
    } else {
      filtered = filtered.filter(path => !path.isSameGenome);
    }

    if (selectedQueryContigId) {
      const queryId = parseInt(selectedQueryContigId);
      filtered = filtered.filter(path => path.qryContigId === queryId);
    }

    if (selectedGenome1 !== '' && selectedGenome2 !== '') {
      const genome1 = parseInt(selectedGenome1);
      const genome2 = parseInt(selectedGenome2);
      filtered = filtered.filter(path => 
        (path.fromFileIndex === genome1 && path.toFileIndex === genome2) ||
        (path.fromFileIndex === genome2 && path.toFileIndex === genome1)
      );
    } else if (selectedGenome1 !== '') {
      const genome = parseInt(selectedGenome1);
      filtered = filtered.filter(path => 
        path.fromFileIndex === genome || path.toFileIndex === genome
      );
    }

    if (selectedChromosome && selectedGenomeForChromosome !== '') {
      const chromosome = parseInt(selectedChromosome);
      const genome = parseInt(selectedGenomeForChromosome);
      filtered = filtered.filter(path => 
        (path.fromFileIndex === genome && path.fromChromosome === chromosome) ||
        (path.toFileIndex === genome && path.toChromosome === chromosome)
      );
    }

    return filtered;
  })();

  /**
   * Clears all active filters
   */
  function clearAllFilters() {
    selectedQueryContigId = '';
    selectedGenome1 = '';
    selectedGenome2 = '';
    selectedChromosome = '';
    selectedGenomeForChromosome = '';
  }

  /**
   * Creates orientation marker SVG path
   * @param point - Marker position
   * @param orientation - Orientation symbol (+/-)
   * @param angle - Angle for direction
   * @returns SVG path string
   */
  function getOrientationMarker(point: {x: number, y: number}, orientation: string, angle: number): string {
    const markerSize = 6 * scale;
    const rad = (angle * Math.PI) / 180;
    
    if (orientation === '+') {
      const tipX = point.x + markerSize * Math.cos(rad);
      const tipY = point.y + markerSize * Math.sin(rad);
      return `M ${point.x} ${point.y} L ${tipX} ${tipY}`;
    } else {
      const tipX = point.x - markerSize * Math.cos(rad);
      const tipY = point.y - markerSize * Math.sin(rad);
      return `M ${point.x} ${point.y} L ${tipX} ${tipY}`;
    }
  }
</script>

<div class="container">
  <div class="chart-section">
    <div class="controls">
      <div class="stats">
        <span>{matches.length} matches | {files.length} genomes</span>
        <span>Total genome size: {totalGenomeSize.toLocaleString()} bp</span>
        <span>Flow lines: {filteredFlowPaths.length} {showDuplicates ? '(self-flow)' : '(cross-genome)'}</span>
        <span class="confidence-stat">Max confidence: {maxConfidence.toFixed(2)}</span>
      </div>
    </div>

    <div class="chart-scale-wrapper" style="transform: scale({scale}); transform-origin: top left;">
      <svg class="chart-svg" width="400" height="400" viewBox="0 0 400 400" preserveAspectRatio="xMidYMid meet">
        <g class="flow-lines">
          {#each filteredFlowPaths as flow}
            <path
              d={flow.path}
              stroke={flow.color}
              stroke-width={flow.width}
              fill="none"
              opacity={flow.opacity}
              stroke-linecap="round"
            />
            
            <g class="orientation-markers">
              <path
                d={getOrientationMarker(
                  flow.p1, 
                  flow.fromOrientation,
                  (Math.atan2(flow.p1.y - centerY, flow.p1.x - centerX) * 180 / Math.PI)
                )}
                stroke={flow.color}
                stroke-width={2 * scale}
                stroke-linecap="round"
                opacity="0.8"
              />
              
              <path
                d={getOrientationMarker(
                  flow.p2, 
                  flow.toOrientation,
                  (Math.atan2(flow.p2.y - centerY, flow.p2.x - centerX) * 180 / Math.PI)
                )}
                stroke={flow.color}
                stroke-width={2 * scale}
                stroke-linecap="round"
                opacity="0.8"
              />
            </g>
          {/each}
        </g>

        <g class="donut-segments">
          {#each segments as seg}
            <circle
              cx={centerX}
              cy={centerY}
              r={radius - strokeWidth / 2}
              fill="transparent"
              stroke={seg.color}
              stroke-width={strokeWidth}
              stroke-dasharray={seg.dashArray}
              stroke-dashoffset={seg.dashOffset}
              transform="rotate(-90 {centerX} {centerY})"
            />
          {/each}
        </g>

        {#if showChromosomes}
          <g class="chromosome-markers">
            {#each segments as seg}
              {#if seg.showChromosomes}
                {#each getChromosomeDivisions(seg) as chr}
                  <line
                    x1={getRadialPoint(chr.startAngle, -strokeWidth).x}
                    y1={getRadialPoint(chr.startAngle, -strokeWidth).y}
                    x2={getRadialPoint(chr.startAngle, 0).x}
                    y2={getRadialPoint(chr.startAngle, 0).y}
                    stroke="#666"
                    stroke-width={1 * scale}
                    opacity="0.7"
                  />
                  
                  {#if chr.chromosome % 2 === 1}
                    <text
                      x={getRadialPoint(chr.midAngle, strokeWidth * 0.35).x}
                      y={getRadialPoint(chr.midAngle, strokeWidth * 0.35).y}
                      text-anchor="middle"
                      dominant-baseline="middle"
                      font-size={7 * scale}
                      font-weight="600"
                      fill="#666"
                      opacity="0.9"
                    >
                      {chr.chromosome}
                    </text>
                  {/if}
                {/each}
              {/if}
            {/each}
          </g>
        {/if}

        <circle cx={centerX} cy={centerY} r={2} fill="#666" />
      </svg>
    </div>
  </div>

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
          {#each Array.from(queryContigStats.entries()).sort((a, b) => b[1].totalOccurrences - a[1].totalOccurrences).slice(0, 10) as [qryId, stat]}
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
          {#if queryContigStats.size > 10}
            <div class="more-matches">
              +{queryContigStats.size - 10} more query contigs...
            </div>
          {/if}
        </div>
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
              <option value={genome.value}>{genome.label}</option>
            {/each}
          </select>
        </div>

        <div class="filter-group">
          <label for="genome2-filter">Genome 2 (optional):</label>
          <select id="genome2-filter" bind:value={selectedGenome2}>
            <option value="">Any Genome</option>
            {#each availableGenomes as genome}
              <option value={genome.value}>{genome.label}</option>
            {/each}
          </select>
        </div>

        <div class="filter-group">
          <label for="genome-chromosome-filter">Genome for Chromosome:</label>
          <select id="genome-chromosome-filter" bind:value={selectedGenomeForChromosome}>
            <option value="">Select Genome</option>
            {#each availableGenomes as genome}
              <option value={genome.value}>{genome.label}</option>
            {/each}
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
        <h2>Chromosome Matches ({matches.length})</h2>
        <div class="match-list">
          {#each matches.slice(0, 20) as match}
            <div class="match-item">
              <div class="match-detail">
                <strong>QryContig {match.qry_contig_id}</strong>
                <span class="confidence">({match.records[0]?.confidence.toFixed(1)})</span>
              </div>
              <div class="match-chromosomes">
                {#each match.records as record, i}
                  <span class="chr-badge" style="background: {files[record.file_index]?.color}20; color: {files[record.file_index]?.color}; border-color: {files[record.file_index]?.color}">
                    File{record.file_index} Chr{record.ref_contig_id} ({record.orientation})
                  </span>
                  {#if i < match.records.length - 1}
                    <span class="arrow">→</span>
                  {/if}
                {/each}
              </div>
            </div>
          {/each}
          {#if matches.length > 20}
            <div class="more-matches">
              +{matches.length - 20} more matches...
            </div>
          {/if}
        </div>
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
</div>

<style>
  .container {
    display: flex;
    gap: 3rem;
    align-items: flex-start;
    flex-wrap: wrap;            /* narrow win wrap */
  }

  .chart-section {
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    max-width: 100%;
  }

  .chart-scale-wrapper {
    display: inline-block;
    border: 1px solid #e5e7eb;
    border-radius: 0.5rem;
    padding: 1rem;
    background: white;
    max-width: 100%;
    width: clamp(260px, 90vw, 420px);
    box-sizing: border-box;
  }

  .chart-svg {
    display: block;
    max-width: 100%;
    height: auto;
  }

  .controls {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-top: 0.5rem;
  }

  .stats {
    font-size: 0.8rem;
    color: #6b7280;
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.25rem 0.75rem;
  }

  .confidence-stat {
    font-weight: 600;
    color: #3b82f6;
  }

  .info {
    flex: 1;
    min-width: 280px; /* avoid overflow w/ very small windows */
  }

  .section {
    margin-bottom: 1.5rem;
    padding: clamp(0.75rem, 1.5vw, 1rem);
    background: white;
    border-radius: 0.5rem;
    border: 1px solid #e5e7eb;
  }

  h2 {
    font-size: clamp(0.95rem, 1.4vw, 1rem);
    font-weight: 600;
    margin-bottom: 0.75rem;
    color: #374151;
  }

  h3 {
    font-size: clamp(0.85rem, 1.3vw, 0.95rem);
    font-weight: 600;
    margin-bottom: 0.5rem;
    color: #374151;
  }

  .overview-section {
    background: white;
    border-color: #e2e8f0;
  }

  .overview-list {
    max-height: 500px;
    overflow-y: auto;
  }

  .overview-item {
    margin-bottom: 1rem;
    padding: 1rem;
    background: white;
    border-radius: 0.5rem;
    border: 1px solid #e5e7eb;
  }

  .overview-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 0.75rem;
    flex-wrap: wrap;
  }

  .overview-header strong {
    color: #1e40af;
    font-size: 0.875rem;
  }

  .overview-total {
    padding: 0.25rem 0.5rem;
    background: #dbeafe;
    color: #1e40af;
    border-radius: 0.25rem;
    font-size: 0.7rem;
    font-weight: 600;
    white-space: nowrap;
  }

  .overview-confidence {
    padding: 0.25rem 0.5rem;
    background: #dcfce7;
    color: #166534;
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
    color: #6b7280;
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

  .filters-section {
    background: #f8fafc;
    border-color: #e2e8f0;
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
    color: #374151;
  }

  .filter-group select {
    padding: 0.5rem;
    border: 1px solid #d1d5db;
    border-radius: 0.375rem;
    font-size: 0.8rem;
    background: white;
    width: 100%;
    box-sizing: border-box;
  }

  .filter-group select:disabled {
    background: #f3f4f6;
    color: #9ca3af;
    cursor: not-allowed;
  }

  .clear-filters-btn {
    padding: 0.5rem 1rem;
    background: #3b82f6;
    color: white;
    border: none;
    border-radius: 0.375rem;
    font-size: 0.8rem;
    cursor: pointer;
    margin-top: 1.25rem;
    width: 100%;
  }

  .clear-filters-btn:hover {
    background: #4b5563;
  }

  .active-filters {
    margin-top: 1rem;
    padding-top: 1rem;
    border-top: 1px solid #e5e7eb;
  }

  .filter-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  .filter-tag {
    padding: 0.25rem 0.5rem;
    background: #3b82f6;
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
    flex: 1 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .file-size,
  .file-pct {
    flex-shrink: 0;
    color: #6b7280;
    font-size: 0.75rem;
    white-space: nowrap;
  }


  .file-pct {
    color: #6b7280;
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
    background: #f9fafb;
    border-radius: 0.5rem;
    border: 1px solid #e5e7eb;
  }

  .match-detail {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: #374151;
    margin-bottom: 0.5rem;
    font-weight: 600;
    flex-wrap: wrap;
  }

  .confidence {
    font-size: 0.75rem;
    color: #6b7280;
    font-weight: normal;
  }

  .match-chromosomes {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .chr-badge {
    padding: 0.25rem 0.5rem;
    border-radius: 0.25rem;
    font-size: 0.75rem;
    font-weight: 500;
    border: 1px solid;
    white-space: nowrap;
  }

  .arrow {
    color: #9ca3af;
    font-weight: bold;
  }

  .more-matches {
    text-align: center;
    padding: 0.5rem;
    color: #6b7280;
    font-size: 0.75rem;
    font-style: italic;
  }

  .debug-info {
    background: #fef3f3;
    border-color: #fecaca;
  }

  .debug-item {
    font-size: 0.75rem;
    margin-bottom: 0.25rem;
    color: #7c2d12;
  }

  /* --- Responsivity --- */
  @media (max-width: 1024px) {
    .container {
      gap: 2rem;
    }
    .stats {
      grid-template-columns: 1fr; /* stack */
    }
    .overview-list {
      max-height: 420px;
    }
  }

  @media (max-width: 768px) {
    .container {
      flex-direction: column;
      gap: 1.25rem;
    }
    .filters-grid {
      grid-template-columns: 1fr; /* collapse to column */
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
    .chart-scale-wrapper {
      width: 100%;
      padding: 0.75rem;
    }
    h2 { font-size: 0.9rem; }
    h3 { font-size: 0.85rem; }
    .genome-badge,
    .chr-badge,
    .filter-tag {
      font-size: 0.7rem;
    }
    .overview-header strong {
      font-size: 0.8rem;
    }
    .overview-list {
      max-height: 320px;
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
    flex-wrap: wrap;             /* prevent early wrap */
    gap: 0.25rem 0.5rem;
  }
  .file-name {
    flex: 1 1 100%;
  }
}
</style>
