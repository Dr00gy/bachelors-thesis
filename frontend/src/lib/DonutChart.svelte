<script lang="ts">
  interface FileData {
    name: string;
    rows: number;
    color: string;
  }

  interface Match {
    from: number;
    fromPct: number;
    to: number;
    toPct: number;
    rows: number;
  }

  export let files: FileData[] = [
    { name: "File A", rows: 100, color: "#EF4444" },
    { name: "File B", rows: 50, color: "#3B82F6" },
    { name: "File C", rows: 150, color: "#10B981" },
  ];

  export let matches: Match[] = [
    { from: 0, fromPct: 0.5, to: 1, toPct: 0.2, rows: 30 },
    { from: 1, fromPct: 0.5, to: 2, toPct: 0.1, rows: 10 },
  ];

  let scale = 1.0;
  const baseRadius = 80;
  const baseStrokeWidth = 20;

  $: radius = baseRadius * scale;
  $: strokeWidth = baseStrokeWidth * scale;
  $: centerX = 200;
  $: centerY = 200;
  $: circumference = 2 * Math.PI * (radius - strokeWidth / 2);
  $: showPercentages = scale >= 1.0;
  $: showChromosomes = scale >= 1.8;

  $: totalRows = files.reduce((sum, f) => sum + f.rows, 0);

  $: segments = (() => {
    let offset = 0;
    return files.map((file, idx) => {
      const pct = file.rows / totalRows;
      const length = pct * circumference;
      const startAngle = (offset / circumference) * 360 - 90;
      const endAngle = ((offset + length) / circumference) * 360 - 90;

      const segment = {
        ...file,
        index: idx,
        dashArray: `${length} ${circumference}`,
        dashOffset: -offset,
        percentage: (pct * 100).toFixed(1),
        showLabel: pct >= 0.15,
        showChromosomes: pct >= 0.25,
        startAngle,
        endAngle
      };
      offset += length;
      return segment;
    });
  })();

  // gen 23 chromosome divs within a colored portion
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

  // get point on circle for chromosome divider lines
  function getRadialPoint(angle: number, radiusOffset: number = 0) {
    const rad = (angle * Math.PI) / 180;
    const r = radius + radiusOffset;
    return {
      x: centerX + r * Math.cos(rad),
      y: centerY + r * Math.sin(rad)
    };
  }

  function getSegmentMarkers(seg: typeof segments[0]) {
    const markers = [];
    const positions = [
      { pct: 0.25, label: '25%' },
      { pct: 0.5, label: '50%' },
      { pct: 0.75, label: '75%' },
    ];

    for (const pos of positions) {
      const angle = seg.startAngle + (seg.endAngle - seg.startAngle) * pos.pct;
      const point = getPointOnDonut(angle, strokeWidth / 2 + 15 * scale);
      markers.push({
        ...point,
        label: pos.label,
        angle
      });
    }

    return markers;
  }

  function getPointOnDonut(angle: number, radiusOffset: number = 0) {
    const rad = (angle * Math.PI) / 180;
    const r = radius + radiusOffset;
    return {
      x: centerX + r * Math.cos(rad),
      y: centerY + r * Math.sin(rad)
    };
  }

  function getAngleInSegment(segmentIdx: number, percent: number) {
    const seg = segments[segmentIdx];
    return seg.startAngle + (seg.endAngle - seg.startAngle) * percent;
  }

  function createCurvePath(fromAngle: number, toAngle: number, intensity: number) {
    const p1 = getPointOnDonut(fromAngle, -strokeWidth / 2);
    const p2 = getPointOnDonut(toAngle, -strokeWidth / 2);
    const controlDist = radius * (0.3 + intensity * 0.4);
    const midAngle = (fromAngle + toAngle) / 2;
    const cp = getPointOnDonut(midAngle, -controlDist);

    return `M ${p1.x} ${p1.y} Q ${cp.x} ${cp.y} ${p2.x} ${p2.y}`;
  }

  $: flowPaths = (() => {
    const _ = scale; // ensure reactivity
    return matches.map((match) => {
      const fromAngle = getAngleInSegment(match.from, match.fromPct);
      const toAngle = getAngleInSegment(match.to, match.toPct);

      const maxRows = Math.max(...matches.map(m => m.rows));
      const intensity = match.rows / maxRows;

      const path = createCurvePath(fromAngle, toAngle, intensity);

      return {
        path,
        color: files[match.from].color,
        opacity: 0.3 + intensity * 0.4,
        width: (1 + intensity * 3) * scale,
        match
      };
    });
  })();
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
    </div>

    <div class="chart-scale-wrapper" style="transform: scale({scale}); transform-origin: top left;">
      <svg width="400" height="400">
        <!-- flow lines -->
        <g style="opacity: 0.6;">
          {#each flowPaths as flow}
            <path
              d={flow.path}
              stroke={flow.color}
              stroke-width={flow.width}
              fill="none"
              opacity={flow.opacity}
              stroke-linecap="round"
            />
          {/each}
        </g>

        <!-- donut segments -->
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

        <!-- chromosome divisions -->
        {#if showChromosomes}
          {#each segments as seg}
            {#if seg.showChromosomes}
              {#each getChromosomeDivisions(seg) as chr}
                <line
                  x1={getRadialPoint(chr.startAngle, -strokeWidth/2).x}
                  y1={getRadialPoint(chr.startAngle, -strokeWidth/2).y}
                  x2={getRadialPoint(chr.startAngle, strokeWidth/2).x}
                  y2={getRadialPoint(chr.startAngle, strokeWidth/2).y}
                  stroke="white"
                  stroke-width={1.5 * scale}
                  opacity="0.6"
                />
                
                <!-- chromosome label (only odd numbers 2 avoid clutter) -->
                {#if chr.chromosome % 2 === 1}
                  <text
                    x={getRadialPoint(chr.midAngle, 0).x}
                    y={getRadialPoint(chr.midAngle, 0).y}
                    text-anchor="middle"
                    dominant-baseline="middle"
                    font-size={8 * scale}
                    font-weight="600"
                    fill="white"
                    opacity="0.9"
                  >
                    {chr.chromosome}
                  </text>
                {/if}
              {/each}
            {/if}
          {/each}
        {/if}

        <!-- percentage markers -->
        {#if showPercentages}
          {#each segments as seg}
            {#if seg.showLabel}
              {#each getSegmentMarkers(seg) as marker}
                <g>
                  <line
                    x1={centerX + (radius + strokeWidth/2) * Math.cos((marker.angle * Math.PI) / 180)}
                    y1={centerY + (radius + strokeWidth/2) * Math.sin((marker.angle * Math.PI) / 180)}
                    x2={marker.x - 5 * scale * Math.cos((marker.angle * Math.PI) / 180)}
                    y2={marker.y - 5 * scale * Math.sin((marker.angle * Math.PI) / 180)}
                    stroke="#374151"
                    stroke-width={1.5 * scale}
                  />
                  <text
                    x={marker.x}
                    y={marker.y}
                    text-anchor="middle"
                    dominant-baseline="middle"
                    font-size={10 * scale}
                    font-weight="500"
                    fill="#374151"
                  >
                    {marker.label}
                  </text>
                </g>
              {/each}
            {/if}
          {/each}
        {/if}
      </svg>
    </div>
  </div>

  <!-- info sec (no scaling) -->
  <div class="info">
    <div class="section">
      <h2>Files</h2>
      {#each files as file, idx}
        <div class="file-item">
          <div class="color-box" style="background: {file.color}"></div>
          <span class="file-name">{file.name}</span>
          <input
            type="number"
            min="1"
            bind:value={files[idx].rows}
            style="width: 60px;"
            on:input={() => files[idx].rows = Math.max(1, files[idx].rows)}
          />
          <span class="file-pct">
            {#if segments[idx].showLabel}
              ({segments[idx].percentage}%)
            {/if}
          </span>
        </div>
      {/each}
    </div>

    {#if matches.length > 0}
      <div class="section">
        <h2>Data Matches</h2>
        {#each matches as match}
          <div class="match-item">
            <div class="match-detail">
              <strong>{files[match.from].name}</strong> ({(match.fromPct * 100).toFixed(0)}%) 
              → <strong>{files[match.to].name}</strong> ({(match.toPct * 100).toFixed(0)}%)
            </div>
          </div>
        {/each}
      </div>
    {/if}
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

  .info {
    flex: 1;
    min-width: 0;
    transform: none !important;
  }

  .section {
    margin-bottom: 1.5rem;
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
  }

  .file-name {
    font-weight: 500;
  }

  .file-pct {
    color: #6b7280;
  }

  input[type="number"] {
    padding: 2px 4px;
    border: 1px solid #d1d5db;
    border-radius: 0.25rem;
    font-size: 0.875rem;
  }

  .match-item {
    font-size: 0.75rem;
    margin-bottom: 0.5rem;
    padding: 0.5rem;
    background: #f3f4f6;
    border-radius: 0.375rem;
  }

  .match-detail {
    color: #374151;
  }
</style>