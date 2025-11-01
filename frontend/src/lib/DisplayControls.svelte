<script lang="ts">
  /**
   * Controls for toggling visualization display options
   * @prop showDuplicates - Boolean controlling whether to show self-flow lines
   */
  export let showDuplicates = false;
  export let scale = 1.0;
</script>

<div class="display-controls">
  <div class="scale-control">
    <label for="scale-slider">
      Size: {Math.round(((scale - 1.0) / 1.2) * 100)}%
    </label>
    <input
      id="scale-slider"
      type="range"
      min="1.0"
      max="2.2"
      step="0.1"
      bind:value={scale}
      class="scale-slider"
    />
  </div>
  
  <label class="toggle-label">
    <input type="checkbox" bind:checked={showDuplicates} />
    <span class="toggle-slider"></span>
    Show self flow lines
  </label>
</div>

<style>
  .display-controls {
    margin-top: 1rem;
    gap: 2rem;
  }
  
  .scale-control {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  
  .scale-control label {
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--text-primary);
  }
  
  .scale-slider {
    width: 300px;
    margin-bottom: 0.8rem;
  }
  
  .toggle-label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
    font-weight: 400;
    color: var(--text-secondary);
    font-size: 0.875rem;
  }
  
  .toggle-label input { 
    display: none; 
  }
  
  .toggle-slider {
    width: 2.25rem;
    height: 1.125rem;
    background: var(--border-color-dark);
    border-radius: 1rem;
    position: relative;
    transition: background 0.2s;
    flex-shrink: 0;
  }
  
  .toggle-slider::before {
    content: '';
    position: absolute;
    width: 0.875rem;
    height: 0.875rem;
    background: var(--bg-primary);
    border-radius: 50%;
    top: 0.125rem;
    left: 0.125rem;
    transition: transform 0.2s;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }
  
  .toggle-label input:checked + .toggle-slider { 
    background: var(--accent-primary); 
  }
  
  .toggle-label input:checked + .toggle-slider::before { 
    transform: translateX(1.125rem);
    background: var(--accent-light);
  }
</style>