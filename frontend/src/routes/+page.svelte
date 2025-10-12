<script lang="ts">
  import { onMount } from 'svelte';
  import DonutChart from '$lib/DonutChart.svelte';
  import TwoLines from '$lib/TwoLines.svelte';

  let files = [
    { name: "one.file", rows: 1000, color: "#3b82f6" },
    { name: "two.file", rows: 800, color: "#10b981" },
    { name: "three.file", rows: 600, color: "#f59e0b" }
  ];

  // sim matches
  let matches = [
    { from: 0, fromPct: 0.25, to: 1, toPct: 0.15, rows: 200 },
    { from: 0, fromPct: 0.60, to: 2, toPct: 0.80, rows: 150 },
    { from: 1, fromPct: 0.50, to: 2, toPct: 0.30, rows: 180 },
    { from: 0, fromPct: 0.85, to: 1, toPct: 0.70, rows: 100 }
  ];

  const customPoints = [
    { x: 150, y: 200, color: '#3B82F6', label: 'X1' },
    { x: 300, y: 200, color: '#3B82F6', label: 'X2' },
    { x: 500, y: 400, color: '#EF4444', label: 'Y1' },
    { x: 700, y: 400, color: '#EF4444', label: 'Y2' }
  ];

  let fileContent = '';

  onMount(async () => {
    try {
      const res = await fetch('http://127.0.0.1:8080/file');
      fileContent = await res.text();
      
    } catch (err) {
      fileContent = 'Failed to fetch file.';
      console.error(err);
    }
  });
</script>

<main>
  <h1>File Content</h1>
  <pre>{fileContent}</pre>
  
  <div class="page">
    <h1>File Data Flow Visualization</h1>
    <DonutChart {files} {matches} />
    <TwoLines points={customPoints} />
  </div>
</main>

<style>
  .page {
    padding: 2rem;
    max-width: 1000px;
    margin: 0 auto;
  }

  h1 {
    margin-bottom: 2rem;
  }
</style>