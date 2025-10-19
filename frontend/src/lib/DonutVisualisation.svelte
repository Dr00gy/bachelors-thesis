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
  import DonutChart from '$lib/DonutChart.svelte';
  import type { BackendMatch } from '$lib/bincodeDecoder';

  /**
   * Wrapper around DonutChart to pass visualization props
   * @prop files - File metadata array
   * @prop matches - Match data from backend
   * @prop showDuplicates - Whether to include intra-genome flows
   */
  export let files: FileData[] = [];
  export let matches: BackendMatch[] = [];
  export let showDuplicates = false;
</script>

<DonutChart {files} {matches} {showDuplicates} />
