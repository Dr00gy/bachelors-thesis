<script lang="ts">
  import { onMount, afterUpdate, onDestroy } from 'svelte';
  import * as d3 from 'd3';
  import type { BackendMatch, ChromosomeInfo } from '$lib/bincodeDecoder';
  import type { FileData } from '$lib/types';
  import DonutInfo from './DonutInfo.svelte';
  import { donutFilterState } from '$lib/filterStateStore';

  export let files: FileData[] = [];
  export let matches: BackendMatch[] = [];
  export let chromosomeInfo: ChromosomeInfo[][] = [];
  export let showDuplicates = false;
  export let scale = 1.1;
  export let isStreaming = false;

  let svgElement: SVGSVGElement;
  let containerElement: HTMLDivElement;

  let currentZoom = 1;
  let currentTranslateX = 0;
  let currentTranslateY = 0;

  /**
   * Progressive rendering state
   */
  let isInitialized = false;
  let lastMatchCount = 0;
  let updateThrottleTimer: any = null;

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
   * Calculates genome sizes either from chromosomeInfo or match data
   */
  $: genomeSizes = (() => {
    const sizes = new Map<number, number>();

    if (chromosomeInfo.length > 0) {
      chromosomeInfo.forEach((chroms, fileIdx) => {
        const total = chroms.reduce((s, c) => s + c.ref_len, 0);
        sizes.set(fileIdx, total);
      });
    } else {
      const byGenome = new Map<number, Map<number, number>>();
      for (const m of matches) {
        for (const r of m.records) {
          if (!byGenome.has(r.file_index)) byGenome.set(r.file_index, new Map());
          const mg = byGenome.get(r.file_index)!;
          if (!mg.has(r.ref_contig_id)) mg.set(r.ref_contig_id, r.ref_len);
        }
      }
      for (const [fileIdx, chrMap] of byGenome.entries()) {
        sizes.set(fileIdx, Array.from(chrMap.values()).reduce((s, v) => s + v, 0));
      }
    }

    files.forEach((_, idx) => {
      if (!sizes.has(idx) || sizes.get(idx) === 0) sizes.set(idx, 100000);
    });

    return sizes;
  })();

  $: totalGenomeSize = Array.from(genomeSizes.values()).reduce((s, v) => s + v, 0);

  /**
   * Calculates maximum confidence value for normalization
   */
  $: maxConfidence = (() => {
    let max = 0;
    for (const m of matches) {
      for (const r of m.records) if (r.confidence > max) max = r.confidence;
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
      const q = match.qry_contig_id;
      if (!stats.has(q)) {
        stats.set(q, {
          totalOccurrences: 0,
          genomeOccurrences: new Map(),
          chromosomeOccurrences: new Map(),
          maxConfidence: 0
        });
      }
      const s = stats.get(q)!;
      for (const r of match.records) {
        s.totalOccurrences++;
        s.genomeOccurrences.set(r.file_index, (s.genomeOccurrences.get(r.file_index) || 0) + 1);
        const key = `${r.file_index}-${r.ref_contig_id}`;
        s.chromosomeOccurrences.set(key, (s.chromosomeOccurrences.get(key) || 0) + 1);
        if (r.confidence > s.maxConfidence) s.maxConfidence = r.confidence;
      }
    }
    return stats;
  })();

  /**
   * Available query contig IDs from matches
   */
  $: availableQueryContigIds = (() => {
    const ids = new Set<number>();
    matches.forEach(m => ids.add(m.qry_contig_id));
    return Array.from(ids).sort((a, b) => a - b);
  })();

  /**
   * Available genomes for filtering
   */
  $: availableGenomes = files.map((f, i) => ({ value: i.toString(), label: f.name, color: f.color }));

  /**
   * Available chromosomes (1-22 + X and Y)
   */
  $: availableChromosomes = Array.from({ length: 24 }, (_, i) => (i + 1).toString());

  $: centerX = 200;
  $: centerY = 200;
  $: baseRadius = 80;
  $: baseStrokeWidth = 20;
  $: radius = baseRadius * scale;
  $: strokeWidth = baseStrokeWidth * scale;
  $: circleR = radius - strokeWidth / 2;
  $: circumference = 2 * Math.PI * circleR;

  /**
   * Donut chart segments with calculated positions
   */
  $: segments = (() => {
    if (totalGenomeSize === 0) return [];

    let offset = 0;
    const segs = files.map((file, idx) => {
      const genomeSize = genomeSizes.get(idx) || 1;
      const pct = genomeSize / totalGenomeSize;
      const length = pct * circumference;

      const startAngle = (offset / circumference) * 360 - 90;
      const endAngle = ((offset + length) / circumference) * 360 - 90;
      const angleRange = endAngle - startAngle;

      const dashArray = `${length} ${circumference}`;
      const dashOffset = -offset;

      offset += length;

      return {
        ...file,
        index: idx,
        genomeSize,
        percentage: (pct * 100).toFixed(1),
        showLabel: pct >= 0.01,
        showChromosomes: pct >= 0.20,
        startAngle,
        endAngle,
        angleRange,
        dashArray,
        dashOffset
      };
    });

    return segs;
  })();

  /**
   * Flow paths for matches between genomes or self-flows
   */
  $: flowPaths = (() => {
    const flows: any[] = [];
    const seenSelf = new Set<string>();
    const maxFlowsToProcess = 1000;
    let processedCount = 0;

    for (const match of matches) {
      if (processedCount >= maxFlowsToProcess) break;
      if (match.records.length < 2) continue;

      for (let i = 0; i < match.records.length; i++) {
        for (let j = i + 1; j < match.records.length; j++) {
          if (processedCount >= maxFlowsToProcess) break;
          
          const from = match.records[i];
          const to = match.records[j];
          if (from.file_index >= files.length || to.file_index >= files.length) continue;

          const isSameGenome = from.file_index === to.file_index;
          if (isSameGenome) {
            const key = `${match.qry_contig_id}-${from.file_index}-${Math.min(from.ref_contig_id, to.ref_contig_id)}-${Math.max(from.ref_contig_id, to.ref_contig_id)}`;
            if (seenSelf.has(key)) continue;
            seenSelf.add(key);
          }

          const avg = (from.confidence + to.confidence) / 2;
          const normalized = avg / maxConfidence;

          flows.push({
            fromFileIndex: from.file_index,
            fromChromosome: from.ref_contig_id,
            toFileIndex: to.file_index,
            toChromosome: to.ref_contig_id,
            color: files[from.file_index]?.color || '#888',
            opacity: 0.1 + (normalized * 0.9),
            width: (1 + normalized * 2) * scale,
            confidence: Math.max(from.confidence, to.confidence),
            isSameGenome,
            qryContigId: match.qry_contig_id,
            fromRecord: from,
            toRecord: to
          });
          
          processedCount++;
        }
      }
    }
    return flows;
  })();

  /**
   * Flow paths filtered by current filter settings
   */
  $: filteredFlowPaths = (() => {
    let f = flowPaths;

    f = showDuplicates ? f.filter(p => p.isSameGenome) : f.filter(p => !p.isSameGenome);

    if (selectedQueryContigId !== '') {
      const q = parseInt(selectedQueryContigId);
      f = f.filter(p => p.qryContigId === q);
    }

    if (selectedGenome1 !== '' && selectedGenome2 !== '') {
      const g1 = parseInt(selectedGenome1);
      const g2 = parseInt(selectedGenome2);
      f = f.filter(p =>
        (p.fromFileIndex === g1 && p.toFileIndex === g2) ||
        (p.fromFileIndex === g2 && p.toFileIndex === g1)
      );
    } else if (selectedGenome1 !== '') {
      const g = parseInt(selectedGenome1);
      f = f.filter(p => p.fromFileIndex === g || p.toFileIndex === g);
    }

    if (selectedChromosome !== '' && selectedGenomeForChromosome !== '') {
      const chr = parseInt(selectedChromosome);
      const g = parseInt(selectedGenomeForChromosome);
      f = f.filter(p =>
        (p.fromFileIndex === g && p.fromChromosome === chr) ||
        (p.toFileIndex === g && p.toChromosome === chr)
      );
    }

    return f;
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
   * Whether to display chromosome labels based on zoom/scale
   */
  $: showChromosomeLabels = scale >= 1.1;

  /**
   * Calculate chromosome node positions for each genome
   */
  $: chromosomeNodes = (() => {
    if (!files.length) return [];

    const nodes: Array<{
      id: string;
      fileIndex: number;
      chromosome: number;
      angle: number;
      x: number;
      y: number;
      color: string;
    }> = [];

    for (let fileIdx = 0; fileIdx < files.length; fileIdx++) {
      const seg = segments[fileIdx];
      if (!seg) continue;

      const segStart = seg.startAngle;
      const segRange = seg.angleRange;

      for (let i = 1; i <= 24; i++) {
        const chrMidDeg = segStart + (segRange * (i - 0.5) / 24);
        const rad = (chrMidDeg * Math.PI) / 180;
        const x = centerX + (radius - strokeWidth) * Math.cos(rad);
        const y = centerY + (radius - strokeWidth) * Math.sin(rad);
        nodes.push({
          id: `chr_${fileIdx}_${i}`,
          fileIndex: fileIdx,
          chromosome: i,
          angle: chrMidDeg,
          x, y,
          color: files[fileIdx].color
        });
      }
    }
    return nodes;
  })();

  let mainGroup: d3.Selection<SVGGElement, unknown, null, undefined>;
  let flowsLayer: d3.Selection<SVGGElement, unknown, null, undefined>;
  let donutLayer: d3.Selection<SVGGElement, unknown, null, undefined>;
  let ticksLayer: d3.Selection<SVGGElement, unknown, null, undefined>;
  let labelsLayer: d3.Selection<SVGGElement, unknown, null, undefined>;

  /**
   * Initialize D3 chart structure
   */
  function initializeChart() {
    if (!svgElement || !files.length) return;

    d3.select(svgElement).selectAll('*').remove();

    const svg = d3
      .select(svgElement)
      .attr('width', 400)
      .attr('height', 400)
      .attr('viewBox', '0 0 400 400')
      .attr('preserveAspectRatio', 'xMidYMid meet');

    mainGroup = svg.append('g').attr('class', 'main-group');
    flowsLayer = mainGroup.append('g').attr('class', 'flow-lines');
    donutLayer = mainGroup.append('g').attr('class', 'donut-segments');
    ticksLayer = mainGroup.append('g').attr('class', 'chromosome-markers');
    labelsLayer = mainGroup.append('g').attr('class', 'chromosome-labels');

    mainGroup.attr('transform', `translate(${currentTranslateX},${currentTranslateY}) scale(${currentZoom})`);
    const zoom = d3.zoom()
      .scaleExtent([0.5, 5])
      .on('zoom', (event) => {
        mainGroup.attr('transform', event.transform);
        currentZoom = event.transform.k;
        currentTranslateX = event.transform.x;
        currentTranslateY = event.transform.y;
      });

    svg.call(zoom as any);
    if (currentZoom !== 1 || currentTranslateX !== 0 || currentTranslateY !== 0) {
      svg.call(zoom.transform as any, d3.zoomIdentity.translate(currentTranslateX, currentTranslateY).scale(currentZoom));
    }

    isInitialized = true;
    updateChart();
  }

  /**
   * Set up initial chart elements with data joins
   */
  function setupChartElements() {
    if (!mainGroup) return;
    updateChart();
  }

  /**
   * Throttled update scheduling for progressive rendering
   */
  function scheduleUpdate() {
    if (updateThrottleTimer) {
      clearTimeout(updateThrottleTimer);
    }
    
    updateThrottleTimer = setTimeout(() => {
      updateChart();
    }, 100);
  }

  /**
   * Track match count changes for progressive updates
   */
  $: if (isInitialized && matches.length !== lastMatchCount) {
    lastMatchCount = matches.length;
    scheduleUpdate();
  }

  /**
   * Update chart elements using internal selections
   */
  function updateChart() {
    if (!mainGroup || !flowsLayer || !ticksLayer || !labelsLayer || !donutLayer) return;

    const cumulativeOffsets = new Map<number, number>();
    let off = 0;
    segments.forEach((s) => {
      cumulativeOffsets.set(s.index, off);
      off += (s.angleRange / 360) * circumference;
    });

    donutLayer
      .selectAll('circle.segment')
      .data(segments, (d: any) => d.index)
      .join(
        enter => enter
          .append('circle')
          .attr('class', 'segment')
          .attr('cx', centerX)
          .attr('cy', centerY)
          .attr('r', circleR)
          .attr('fill', 'transparent')
          .attr('stroke', (d: any) => d.color)
          .attr('stroke-width', strokeWidth)
          .attr('stroke-dasharray', (d: any) => {
            const length = (d.angleRange / 360) * circumference;
            return `${length} ${circumference}`;
          })
          .attr('stroke-dashoffset', (d: any) => {
            const offset = cumulativeOffsets.get(d.index) || 0;
            return -offset;
          })
          .attr('transform', `rotate(-90 ${centerX} ${centerY})`),
        update => update
          .attr('r', circleR)
          .attr('stroke-width', strokeWidth)
          .attr('stroke', (d: any) => d.color)
          .attr('stroke-dasharray', (d: any) => {
            const length = (d.angleRange / 360) * circumference;
            return `${length} ${circumference}`;
          })
          .attr('stroke-dashoffset', (d: any) => {
            const offset = cumulativeOffsets.get(d.index) || 0;
            return -offset;
          }),
        exit => exit.remove()
      );

    const ticks: Array<{ x1: number; y1: number; x2: number; y2: number; key: string }> = [];
    for (const seg of segments) {
      const start = seg.startAngle * Math.PI / 180;
      const end = seg.endAngle * Math.PI / 180;
      const range = end - start;

      for (let i = 0; i <= 24; i++) {
        const a = start + (range * i / 24);
        const x1 = centerX + (radius - strokeWidth) * Math.cos(a);
        const y1 = centerY + (radius - strokeWidth) * Math.sin(a);
        const x2 = centerX + radius * Math.cos(a);
        const y2 = centerY + radius * Math.sin(a);
        ticks.push({ x1, y1, x2, y2, key: `t-${seg.index}-${i}` });
      }
    }

    ticksLayer
      .selectAll('line.tick')
      .data(ticks, (d: any) => d.key)
      .join(
        enter => enter
          .append('line')
          .attr('class', 'tick')
          .attr('x1', d => d.x1)
          .attr('y1', d => d.y1)
          .attr('x2', d => d.x2)
          .attr('y2', d => d.y2)
          .attr('stroke', 'var(--text-primary)')
          .attr('stroke-width', 1 * scale)
          .attr('opacity', 0.7),
        update => update
          .attr('x1', d => d.x1)
          .attr('y1', d => d.y1)
          .attr('x2', d => d.x2)
          .attr('y2', d => d.y2)
          .attr('stroke-width', 1 * scale),
        exit => exit.remove()
      );

    const flowLines = filteredFlowPaths.map(flow => {
      const fromNode = chromosomeNodes.find(n => n.fileIndex === flow.fromFileIndex && n.chromosome === flow.fromChromosome);
      const toNode = chromosomeNodes.find(n => n.fileIndex === flow.toFileIndex && n.chromosome === flow.toChromosome);
      return (fromNode && toNode) ? { ...flow, fromNode, toNode } : null;
    }).filter(Boolean) as any[];

    flowsLayer
      .selectAll('path.flow')
      .data(flowLines, (d: any) => `${d.qryContigId}-${d.fromFileIndex}-${d.fromChromosome}-${d.toFileIndex}-${d.toChromosome}`)
      .join(
        enter => enter
          .append('path')
          .attr('class', 'flow')
          .attr('d', (d: any) => {
            const x1 = d.fromNode.x, y1 = d.fromNode.y;
            const x2 = d.toNode.x, y2 = d.toNode.y;
            return `M ${x1} ${y1} Q ${centerX} ${centerY} ${x2} ${y2}`;
          })
          .attr('stroke', (d: any) => d.color)
          .attr('stroke-width', (d: any) => d.width)
          .attr('fill', 'none')
          .attr('opacity', (d: any) => d.opacity)
          .attr('stroke-linecap', 'round'),
        update => update
          .attr('d', (d: any) => {
            const x1 = d.fromNode.x, y1 = d.fromNode.y;
            const x2 = d.toNode.x, y2 = d.toNode.y;
            return `M ${x1} ${y1} Q ${centerX} ${centerY} ${x2} ${y2}`;
          })
          .attr('stroke-width', (d: any) => d.width)
          .attr('opacity', (d: any) => d.opacity),
        exit => exit.remove()
      );

    const chromLabels = labelsLayer
      .selectAll('text.chrom-label')
      .data(showChromosomeLabels ? chromosomeNodes.filter(d => d.chromosome % 2 === 1) : [], (d: any) => d.id);

    chromLabels.join(
      enter => enter
        .append('text')
        .attr('class', 'chrom-label')
        .attr('x', d => centerX + (radius + 10 * scale) * Math.cos((d.angle * Math.PI) / 180))
        .attr('y', d => centerY + (radius + 10 * scale) * Math.sin((d.angle * Math.PI) / 180))
        .attr('text-anchor', 'middle')
        .attr('dominant-baseline', 'middle')
        .attr('font-size', 7 * scale)
        .attr('font-weight', 600)
        .attr('fill', 'var(--text-primary)')
        .attr('opacity', 0.9)
        .text(d => d.chromosome),
      update => update
        .attr('x', d => centerX + (radius + 10 * scale) * Math.cos((d.angle * Math.PI) / 180))
        .attr('y', d => centerY + (radius + 10 * scale) * Math.sin((d.angle * Math.PI) / 180))
        .attr('font-size', 7 * scale),
      exit => exit.remove()
    );

    const centerDot = mainGroup.selectAll('circle.center').data([1]);
    centerDot.join(
      enter => enter
        .append('circle')
        .attr('class', 'center')
        .attr('cx', centerX)
        .attr('cy', centerY)
        .attr('r', 2)
        .attr('fill', 'var(--text-secondary)'),
      update => update
        .attr('cx', centerX)
        .attr('cy', centerY),
      exit => exit.remove()
    );
  }

  /**
   * Reset zoom to default
   */
  function resetZoom() {
    if (!svgElement) return;
    const svg = d3.select(svgElement);
    svg.call(d3.zoom().transform as any, d3.zoomIdentity);
    currentZoom = 1;
    currentTranslateX = 0;
    currentTranslateY = 0;
    if (mainGroup) {
      mainGroup.attr('transform', `translate(0,0) scale(1)`);
    }
  }

  afterUpdate(() => {
    if (!isInitialized && chromosomeInfo.length > 0 && files.length > 0) {
      initializeChart();
    } else if (mainGroup) {
      updateChart();
    }
  });

  onMount(() => {
    if (chromosomeInfo.length > 0 && files.length > 0) {
      initializeChart();
    }
  });

  onDestroy(() => {
    if (updateThrottleTimer) {
      clearTimeout(updateThrottleTimer);
    }
  });
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

    {#if !files.length || !chromosomeInfo.length}
      <div class="no-data">
        {#if !files.length}
          No data to display. Upload XMAP files to begin.
        {:else}
          <div class="loading-state">
            <div class="spinner"></div>
            <span>Initializing visualization...</span>
          </div>
        {/if}
      </div>
    {:else}
      <div class="chart-wrapper">
        <div class="zoom-controls">
          <button class="zoom-btn" on:click={resetZoom} title="Reset zoom" aria-label="Reset zoom">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <path d="M2 8a6 6 0 0 1 10.5-4M14 8a6 6 0 0 1-10.5 4" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
              <path d="M12.5 2v4h-4" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </button>
          <span class="zoom-indicator">{(currentZoom * 100).toFixed(0)}%</span>
        </div>
        <div class="chart-container" bind:this={containerElement}>
          <svg bind:this={svgElement} class="chart-svg"></svg>
        </div>
        <div class="zoom-hint">
          Scroll to zoom • Drag to pan
        </div>
        {#if isStreaming}
          <div class="streaming-notice">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <circle cx="8" cy="8" r="3" fill="currentColor" opacity="0.3">
                <animate attributeName="r" values="3;6;3" dur="1.5s" repeatCount="indefinite"/>
                <animate attributeName="opacity" values="0.3;0;0.3" dur="1.5s" repeatCount="indefinite"/>
              </circle>
              <circle cx="8" cy="8" r="2" fill="currentColor"/>
            </svg>
            Updating visualization as data streams in...
          </div>
        {/if}
      </div>
    {/if}
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

  .chart-wrapper {
    position: relative;
    border: 1px solid var(--border-color);
    border-radius: 0.5rem;
    background: var(--bg-secondary);
    padding: 1rem;
    width: clamp(260px, 90vw, 500px);
    box-sizing: border-box;
  }

  .zoom-controls {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }

  .zoom-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2rem;
    height: 2rem;
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: 0.375rem;
    color: var(--text-primary);
    cursor: pointer;
  }

  .zoom-btn:hover {
    background: var(--accent-primary);
    color: white;
    border-color: var(--accent-primary);
  }

  .zoom-indicator {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-secondary);
    padding: 0.25rem 0.5rem;
    background: var(--bg-primary);
    border-radius: 0.25rem;
  }

  .chart-container {
    width: 100%;
    height: 500px;
    overflow: hidden;
    cursor: grab;
    border-radius: 0.375rem;
    background: var(--bg-primary);
  }

  .chart-container:active {
    cursor: grabbing;
  }

  .chart-svg {
    display: block;
    width: 100%;
    height: 100%;
  }

  .zoom-hint {
    margin-top: 0.5rem;
    padding-top: 0.5rem;
    font-size: 0.7rem;
    color: var(--text-tertiary);
    text-align: center;
  }

  .no-data {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 4rem;
    background: var(--bg-secondary);
    border-radius: 0.5rem;
    border: 1px solid var(--border-color);
    color: var(--text-secondary);
    min-height: 300px;
  }

  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
  }

  .spinner {
    width: 40px;
    height: 40px;
    border: 4px solid var(--accent-primary);
    border-top-color: transparent;
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .streaming-notice {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-top: 0.75rem;
    padding: 0.75rem;
    background: var(--accent-light);
    border-radius: 0.375rem;
    border: 1px solid var(--accent-primary);
    color: var(--accent-primary);
    font-size: 0.8rem;
    font-weight: 500;
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
    
    .chart-container {
      height: 400px;
    }
  }
</style>