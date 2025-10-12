<script lang="ts">
  interface Point {
    x: number;
    y: number;
    color?: string;
    label?: string;
  }

  const lines = [
    { id: 0, color: '#3B82F6', y: 200 }, // blue
    { id: 1, color: '#EF4444', y: 400 }  // red
  ];

  export let points: Point[] = [
    { x: 150, y: 200, color: '#3B82F6', label: 'X1' },
    { x: 300, y: 200, color: '#3B82F6', label: 'X2' },
    { x: 500, y: 400, color: '#EF4444', label: 'Y1' },
    { x: 700, y: 400, color: '#EF4444', label: 'Y2' }
  ];

  let width = 800;
  let height = 500;

  const tickStep = 100;
  $: ticks = Array.from({ length: Math.floor(width / tickStep) + 1 }, (_, i) => i * tickStep);

  // random noise spikes
  const numSamples = 80; // more = finer noise
  function generateNoise(lineY: number, color: string) {
    const step = width / numSamples;
    const baseY = lineY;
    const noisePoints = Array.from({ length: numSamples + 1 }, (_, i) => {
      const x = i * step;
      const spike = Math.random() * 40 + 10;
      return { x, y: baseY - spike };
    });
    return noisePoints;
  }

  $: noiseData = lines.map(line => ({
    line,
    path: generateNoise(line.y, line.color)
  }));

  // noise array to SVG path
  function pointsToPath(points: { x: number; y: number }[], baseY: number) {
    let d = `M ${points[0].x},${baseY}`;
    for (const p of points) d += ` L ${p.x},${p.y}`;
    d += ` L ${(points.at(-1)?.x ?? 0)},${baseY} Z`;
    return d;
  }
</script>

<svg {width} {height} class="lines-svg">
  <g class="scale">
    {#each ticks as x}
      <line x1={x} x2={x} y1="20" y2="30" stroke="#9CA3AF" stroke-width="1" />
      <text x={x} y="10" text-anchor="middle" font-size="10" fill="#6B7280">
        {x}
      </text>
    {/each}
  </g>

  <!-- noise overlays -->
  {#each noiseData as { line, path }}
    <path
      d={pointsToPath(path, line.y)}
      fill={line.color}
      opacity="0.15"
    />
  {/each}

  {#each lines as line}
    <line
      x1="0"
      x2={width}
      y1={line.y}
      y2={line.y}
      stroke={line.color}
      stroke-width="3"
    />
  {/each}

  {#each points as p}
    <g transform={`translate(${p.x}, ${p.y})`}>
      <circle r="6" fill={p.color ?? '#000'} />
      {#if p.label}
        <text x="10" y="4" font-size="12" fill="#374151">{p.label}</text>
      {/if}
    </g>
  {/each}
</svg>

<style>
  .lines-svg {
    width: 100%;
    height: auto;
    background: #f9fafb;
    border-radius: 0.5rem;
    display: block;
    padding: 1rem;
  }

  .scale text {
    font-family: system-ui, sans-serif;
  }
</style>
