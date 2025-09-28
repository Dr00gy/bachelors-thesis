<script lang="ts">
  import { onMount } from 'svelte';
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
</main>
