<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  const dispatch = createEventDispatcher();

  /**
   * Props for FileUpload component
   * @property isLoading - Whether upload is currently in progress
   * @property matchCount - Number of processed matches (for progress display)
   */
  export let isLoading = false;
  export let matchCount = 0;
  let fileInput: HTMLInputElement;

  /**
   * Triggers hidden file input element
   */
  function triggerUpload() {
    fileInput.click();
  }

  /**
   * Handles file input change event and dispatches files to parent
   */
  function handleChange(e: Event) {
    const files = (e.target as HTMLInputElement).files;
    if (files) dispatch('upload', files);
  }

  /**
   * Cancels ongoing upload
   */
  function handleCancel() {
    dispatch('cancel');
  }
</script>

<div class="upload-section">
  <div class="upload-controls">
    <input bind:this={fileInput} type="file" multiple accept=".xmap" on:change={handleChange} />
    <button on:click={triggerUpload} disabled={isLoading}>Upload Files</button>
    {#if isLoading}
      <button class="cancel-button" on:click={handleCancel}>Cancel</button>
    {/if}
  </div>

  <div class="status">
    {#if isLoading}
      Uploading... Processed {matchCount} matches
    {:else}
      ...
    {/if}
  </div>
</div>

<style>
  .upload-section {
    margin-bottom: 2rem;
    padding: 1.5rem;
    background: #f9fafb;
    border-radius: 0.5rem;
    border: 2px dashed #d1d5db;
  }
  .upload-controls {
    display: flex;
    gap: 1rem;
    align-items: center;
  }
  input[type="file"] { display: none; }
  button {
    padding: 0.75rem 1.5rem;
    background: #3b82f6;
    color: white;
    border: none;
    border-radius: 0.5rem;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.2s;
  }
  button:hover:not(:disabled) { background: #2563eb; }
  button:disabled { background: #9ca3af; cursor: not-allowed; }
  .cancel-button { background: #ef4444; }
  .cancel-button:hover { background: #dc2626; }
  .status { margin-top: 1rem; color: #6b7280; font-style: italic; }
</style>
