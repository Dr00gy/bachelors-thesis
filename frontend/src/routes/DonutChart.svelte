<script lang="ts">
  import type { BackendMatch, ChromosomeInfo } from '$lib/bincodeDecoder';
  import type { FileData, DonutSegment, FlowPath, ChromosomeDivision } from '$lib/types';
  import DonutInfo from './DonutInfo.svelte';
  import { donutFilterState } from '$lib/filterStateStore';

  /**
   * Component props
   */
  export let files: FileData[] = [];
  export let matches: BackendMatch[] = [];
  export let chromosomeInfo: ChromosomeInfo[][] = [];
  export let showDuplicates = false;

  /**
   * Filter state from store
   */
  let selectedQueryContigId = $donutFilterState.selectedQueryContigId;
  let selectedGenome1 = $donutFilterState.selectedGenome1;
  let selectedGenome2 = $donutFilterState.selectedGenome2;
  let selectedChromosome = $donutFilterState.selectedChromosome;
  let selectedGenomeForChromosome = $donutFilterState.selectedGenomeForChromosome;

  /**
   * Update store when filters change
   */
  $: donutFilterState.set({
    selectedQueryContigId,
    selectedGenome1,
    selectedGenome2,
    selectedChromosome,
    selectedGenomeForChromosome,
    showDuplicates,
    scale
  });

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
  $: showChromosomes = scale >= 1.1;

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
   * Available chromosomes (1-22 + X and Y)
   */
  $: availableChromosomes = Array.from({ length: 24 }, (_, i) => (i + 1).toString());

  /**
   * Calculates genome sizes from chromosome information
   */
  $: genomeSizes = (() => {
    const sizes = new Map<number, number>();
    
    if (chromosomeInfo.length > 0) {
      chromosomeInfo.forEach((chromosomes, fileIndex) => {
        const totalSize = chromosomes.reduce((sum, chr) => sum + chr.ref_len, 0);
        sizes.set(fileIndex, totalSize);
      });
    } else {
      // fallback only gets refLen where there are matches
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
      
      for (const [fileIdx, chromosomes] of chromosomesByGenome.entries()) {
        const totalSize = Array.from(chromosomes.values()).reduce((sum, len) => sum + len, 0);
        sizes.set(fileIdx, totalSize);
      }
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
   */
  function getChromosomeDivisions(seg: DonutSegment): ChromosomeDivision[] {
    const divisions = [];
    const segmentRange = seg.endAngle - seg.startAngle;
    
    for (let i = 0; i < 24; i++) {
      const chrStart = seg.startAngle + (segmentRange * i / 24);
      const chrEnd = seg.startAngle + (segmentRange * (i + 1) / 24);
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
   * Gets angle for record within its chromosome
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
   */
  function getPointOnDonut(angle: number, radiusOffset: number = 0) {
    return getRadialPoint(angle, radiusOffset);
  }

  /**
   * Creates curved flow path between two points
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

    if (selectedQueryContigId !== '') {
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

    if (selectedChromosome !== '' && selectedGenomeForChromosome !== '') {
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
                    stroke="var(--text-primary)"
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
                      fill="var(--text-primary)"
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

        <circle cx={centerX} cy={centerY} r={2} fill="var(--text-secondary)" />
      </svg>
    </div>
  </div>

  <DonutInfo
      {files}
      {matches}
      {segments}
      {genomeSizes}
      {totalGenomeSize}
      {filteredFlowPaths}
      {showDuplicates}
      bind:selectedQueryContigId
      bind:selectedGenome1
      bind:selectedGenome2
      bind:selectedChromosome
      bind:selectedGenomeForChromosome
      {availableQueryContigIds}
      {availableGenomes}
      {availableChromosomes}
      {queryContigStats}
      {clearAllFilters}
    />
</div>

<style>
  .container {
    display: flex;
    gap: 3rem;
    align-items: flex-start;
    flex-wrap: wrap;
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
    border: 1px solid var(--border-color);
    border-radius: 0.5rem;
    padding: 1rem;
    background: var(--bg-secondary);
    max-width: 100%;
    width: clamp(260px, 90vw, 420px);
    box-sizing: border-box;
    z-index: 10;
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
    color: var(--text-secondary);
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.25rem 0.75rem;
  }

  .confidence-stat {
    font-weight: 600;
    color: var(--accent-primary);
  }

  @media (max-width: 1024px) {
    .stats {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 768px) {
    .container {
      flex-direction: column;
      gap: 1.25rem;
    }
  }
</style>