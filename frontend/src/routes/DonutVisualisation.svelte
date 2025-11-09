<script lang="ts" context="module">
  /**
   * FileData interface represents file metadata used in visualization
   */
  export interface FileData {
    name: string;
    rows: number;
    color: string;
  }
</script>

<script lang="ts">
  import DonutChart from './DonutChart.svelte';
  import type { BackendMatch, ChromosomeInfo } from '$lib/bincodeDecoder';

  /**
   * Wrapper around DonutChart to pass visualization props
   * @prop files - File metadata array
   * @prop matches - Match data from backend
   * @prop chromosomeInfo - Chromosome length information
   * @prop showDuplicates - Whether to include intra-genome flows
   * @prop scale - Visualization scale factor
   */
  export let files: FileData[] = [];
  export let matches: BackendMatch[] = [];
  export let chromosomeInfo: ChromosomeInfo[][] = [];
  export let showDuplicates = false;
  export let scale = 1.0;
</script>

<DonutChart {files} {matches} {chromosomeInfo} {showDuplicates} {scale}/>