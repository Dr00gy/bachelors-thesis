<script lang="ts">
  interface FileData {
    name: string;
    rows: number;
    color: string;
  }

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

  export let files: FileData[] = [];
  export let matches: BackendMatch[] = [];

  let scale = 1.0;
  const baseRadius = 80;
  const baseStrokeWidth = 20;

  $: radius = baseRadius * scale;
  $: strokeWidth = baseStrokeWidth * scale;
  $: centerX = 200;
  $: centerY = 200;
  $: circumference = 2 * Math.PI * (radius - strokeWidth / 2);
  $: showChromosomes = scale >= 1.0;

  // sizes from RefStartPos and RefEndPos, will refactor later
  $: genomeSizes = (() => {
    const sizes = new Map<number, number>();
    
    for (const match of matches) {
      for (const record of match.records) {
        const fileIdx = record.file_index;
        const currentMax = sizes.get(fileIdx) || 0;
        // maximum RefEndPos as the genome size indicator
        const recordSize = Math.max(record.ref_start_pos, record.ref_end_pos);
        if (recordSize > currentMax) {
          sizes.set(fileIdx, recordSize);
        }
      }
    }
    
    // minimum sizes for display
    files.forEach((_, idx) => {
      if (!sizes.has(idx) || sizes.get(idx) === 0) {
        sizes.set(idx, 100000);
      }
    });
    
    return sizes;
  })();

  $: totalGenomeSize = Array.from(genomeSizes.values()).reduce((sum, size) => sum + size, 0);

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
        showChromosomes: true, // always show chromosomes rn, will fix zoom later
        startAngle,
        endAngle,
        angleRange: endAngle - startAngle
      };
      offset += length;
      return segment;
    });
  })();

  function getChromosomeDivisions(seg: typeof segments[0]) {
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

  function getPositionInChromosome(refStartPos: number, refEndPos: number): number {
    const avgPos = (refStartPos + refEndPos) / 2;
    const chromosomeLength = 250_000_000; // temp hardcoded val
    return Math.min(1, Math.max(0, avgPos / chromosomeLength));
  }

  function getAngleInChromosome(
    fileIndex: number, 
    chromosome: number, 
    refStartPos: number, 
    refEndPos: number
  ): number {
    const chrStartAngle = getChromosomeAngle(fileIndex, chromosome, 'start');
    const chrEndAngle = getChromosomeAngle(fileIndex, chromosome, 'end');
    const positionPct = getPositionInChromosome(refStartPos, refEndPos);
    
    return chrStartAngle + (chrEndAngle - chrStartAngle) * positionPct;
  }

  function getRadialPoint(angle: number, radiusOffset: number = 0) {
    const rad = (angle * Math.PI) / 180;
    const r = radius + radiusOffset;
    return {
      x: centerX + r * Math.cos(rad),
      y: centerY + r * Math.sin(rad)
    };
  }

  function getPointOnDonut(angle: number, radiusOffset: number = 0) {
    return getRadialPoint(angle, radiusOffset);
  }

  function createFlowPath(
    fromAngle: number, 
    toAngle: number, 
    intensity: number,
    fromOrientation: string,
    toOrientation: string
  ) {
    const p1 = getPointOnDonut(fromAngle, -strokeWidth / 2);
    const p2 = getPointOnDonut(toAngle, -strokeWidth / 2);
    
    // control point for curved path
    const midAngle = (fromAngle + toAngle) / 2;
    const angleDiff = Math.abs(toAngle - fromAngle);
    const controlDist = radius * (0.5 + Math.min(angleDiff / 180, 1) * 0.5);
    const cp = getPointOnDonut(midAngle, -controlDist);

    return {
      path: `M ${p1.x} ${p1.y} Q ${cp.x} ${cp.y} ${p2.x} ${p2.y}`,
      p1,
      p2,
      fromOrientation,
      toOrientation
    };
  }

  $: flowPaths = (() => {
    const paths = [];
    
    console.log('Generating flow paths for', matches.length, 'matches');
    
    for (const match of matches) {
      if (match.records.length < 2) continue;
      
      const maxConfidence = Math.max(...match.records.map(r => r.confidence));
      
      for (let i = 0; i < match.records.length - 1; i++) {
        const fromRecord = match.records[i];
        const toRecord = match.records[i + 1];
        
        // if file indices are out of bounds, skip
        if (fromRecord.file_index >= files.length || toRecord.file_index >= files.length) {
          continue;
        }
        
        const fromAngle = getAngleInChromosome(
          fromRecord.file_index,
          fromRecord.ref_contig_id,
          fromRecord.ref_start_pos,
          fromRecord.ref_end_pos
        );
        
        const toAngle = getAngleInChromosome(
          toRecord.file_index,
          toRecord.ref_contig_id,
          toRecord.ref_start_pos,
          toRecord.ref_end_pos
        );
        
        const intensity = Math.min(1, fromRecord.confidence / 20); // nromalising confidence might b needed
        const flowData = createFlowPath(
          fromAngle, 
          toAngle, 
          intensity,
          fromRecord.orientation,
          toRecord.orientation
        );
        
        paths.push({
          ...flowData,
          color: files[fromRecord.file_index]?.color || '#888',
          opacity: 0.4 + intensity * 0.4,
          width: (1 + intensity * 2) * scale,
          fromChromosome: fromRecord.ref_contig_id,
          toChromosome: toRecord.ref_contig_id,
          confidence: fromRecord.confidence
        });
      }
    }
    
    console.log('Generated', paths.length, 'flow paths');
    return paths;
  })();

  function getOrientationMarker(point: {x: number, y: number}, orientation: string, angle: number) {
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
  $: console.log('Segments:', segments);
  $: console.log('Genome sizes:', genomeSizes);
  $: console.log('Flow paths count:', flowPaths.length);
</script>

<div class="container">
  <div class="chart-section">
    <div class="controls">
      <label for="scale-slider">
        Size: {Math.round(scale * 100)}%
      </label>
      <input
        id="scale-slider"
        type="range"
        min="0.5"
        max="2.5"
        step="0.1"
        bind:value={scale}
      />
      <div class="stats">
        <span>{matches.length} matches | {files.length} genomes</span>
        <span>Total genome size: {totalGenomeSize.toLocaleString()} bp</span>
      </div>
    </div>

    <div class="chart-scale-wrapper" style="transform: scale({scale}); transform-origin: top left;">
      <svg width="400" height="400" viewBox="0 0 400 400">
        <!-- Flow lines -->
        <g class="flow-lines">
          {#each flowPaths as flow}
            <path
              d={flow.path}
              stroke={flow.color}
              stroke-width={flow.width}
              fill="none"
              opacity={flow.opacity}
              stroke-linecap="round"
            />
            
            <!-- Orientation markers? not working well yet -->
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

        <!-- Donut segments -->
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

        <!-- Chromosome divisions -->
        {#if showChromosomes}
          <g class="chromosome-markers">
            {#each segments as seg}
              {#each getChromosomeDivisions(seg) as chr}
                <line
                  x1={getRadialPoint(chr.startAngle, -strokeWidth/2).x}
                  y1={getRadialPoint(chr.startAngle, -strokeWidth/2).y}
                  x2={getRadialPoint(chr.startAngle, strokeWidth/2).x}
                  y2={getRadialPoint(chr.startAngle, strokeWidth/2).y}
                  stroke="white"
                  stroke-width={1 * scale}
                  opacity="0.7"
                />
                
                {#if chr.chromosome % 2 === 1}
                  <text
                    x={getRadialPoint(chr.midAngle, 0).x}
                    y={getRadialPoint(chr.midAngle, 0).y}
                    text-anchor="middle"
                    dominant-baseline="middle"
                    font-size={7 * scale}
                    font-weight="600"
                    fill="white"
                    opacity="0.9"
                  >
                    {chr.chromosome}
                  </text>
                {/if}
              {/each}
            {/each}
          </g>
        {/if}

        <!-- centet pt? -->
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

    <!-- Debug stuff -->
    <div class="section debug-info">
      <h2>Debug Info</h2>
      <div class="debug-item">
        <strong>Total Genome Size:</strong> {totalGenomeSize.toLocaleString()} bp
      </div>
      <div class="debug-item">
        <strong>Flow Paths:</strong> {flowPaths.length}
      </div>
      <div class="debug-item">
        <strong>Segments:</strong> {segments.length}
      </div>
      {#each segments as seg, i}
        <div class="debug-item">
          <strong>Segment {i}:</strong> {seg.genomeSize.toLocaleString()} bp ({seg.percentage}%)
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
  .container {
    display: flex;
    gap: 3rem;
    align-items: flex-start;
  }

  .chart-section {
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .chart-scale-wrapper {
    display: inline-block;
    border: 1px solid #e5e7eb;
    border-radius: 0.5rem;
    padding: 1rem;
    background: white;
  }

  .controls {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .controls label {
    font-size: 0.875rem;
    font-weight: 500;
    color: #374151;
  }

  .controls input[type="range"] {
    width: 100%;
    max-width: 300px;
  }

  .stats {
    font-size: 0.75rem;
    color: #6b7280;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .info {
    flex: 1;
    min-width: 0;
  }

  .section {
    margin-bottom: 1.5rem;
    padding: 1rem;
    background: white;
    border-radius: 0.5rem;
    border: 1px solid #e5e7eb;
  }

  h2 {
    font-size: 1rem;
    font-weight: 600;
    margin-bottom: 0.75rem;
    color: #374151;
  }

  .file-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
    font-size: 0.875rem;
  }

  .color-box {
    width: 1rem;
    height: 1rem;
    border-radius: 0.25rem;
    flex-shrink: 0;
  }

  .file-name {
    font-weight: 500;
    flex: 1;
  }

  .file-size {
    color: #6b7280;
    font-size: 0.75rem;
  }

  .file-pct {
    color: #6b7280;
    font-size: 0.75rem;
  }

  .match-list {
    max-height: 400px;
    overflow-y: auto;
  }

  .match-item {
    font-size: 0.75rem;
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
  }

  .confidence {
    font-size: 0.7rem;
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
    font-size: 0.7rem;
    font-weight: 500;
    border: 1px solid;
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
</style>