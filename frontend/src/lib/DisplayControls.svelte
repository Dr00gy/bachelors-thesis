<script lang="ts">
  /**
   * Controls for toggling visualization display options
   * @prop showDuplicates - Boolean controlling whether to show self-flow lines
   */
  export let showDuplicates = false;
</script>

<div class="display-controls">
  <label class="toggle-label">
    <input type="checkbox" bind:checked={showDuplicates} />
    <span class="toggle-slider"></span>
    Show self-flow lines (same genome)
  </label>

  <div class="toggle-description">
    {#if showDuplicates}
      Showing self-flow lines within the same genome
    {:else}
      Showing only flow lines between different genomes
    {/if}
  </div>
</div>

<style>
  .display-controls {
    margin-top: 1rem;
    padding: 1rem;
    background: white;
    border-radius: 0.375rem;
    border: 1px solid #e5e7eb;
  }
  .toggle-label {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    cursor: pointer;
    font-weight: 500;
    color: #374151;
  }
  .toggle-label input { display: none; }
  .toggle-slider {
    width: 3rem;
    height: 1.5rem;
    background: #d1d5db;
    border-radius: 1rem;
    position: relative;
    transition: background 0.2s;
  }
  .toggle-slider::before {
    content: '';
    position: absolute;
    width: 1.25rem;
    height: 1.25rem;
    background: white;
    border-radius: 50%;
    top: 0.125rem;
    left: 0.125rem;
    transition: transform 0.2s;
  }
  .toggle-label input:checked + .toggle-slider { background: #3b82f6; }
  .toggle-label input:checked + .toggle-slider::before { transform: translateX(1.5rem); }
  .toggle-description {
    margin-top: 0.5rem;
    font-size: 0.875rem;
    color: #6b7280;
    font-style: italic;
  }
</style>
