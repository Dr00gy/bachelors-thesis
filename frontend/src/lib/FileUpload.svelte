<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  const dispatch = createEventDispatcher();
  
  let fileInput: HTMLInputElement;
  let isDragging = false;
  let dragCounter = 0;

  /**
   * Triggers hidden file input element
   */
  function triggerUpload(e?: Event) {
    if (e) {
      e.preventDefault();
      e.stopPropagation();
    }
    fileInput.click();
  }

  /**
   * Handles file input change event and dispatches files to parent
   */
  function handleChange(e: Event) {
    const files = (e.target as HTMLInputElement).files;
    if (files) {
      dispatch('upload', files);
      if (fileInput) fileInput.value = '';
    }
  }

  /**
   * Handles drag enter event
   */
  function handleDragEnter(e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();
    dragCounter++;
    if (e.dataTransfer?.items) {
      isDragging = true;
    }
  }

  /**
   * Handles drag over event
   */
  function handleDragOver(e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();
    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = 'copy';
    }
  }

  /**
   * Handles drag leave event
   */
  function handleDragLeave(e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();
    dragCounter--;
    if (dragCounter === 0) {
      isDragging = false;
    }
  }

  /**
   * Handles file drop event
   */
  function handleDrop(e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();
    isDragging = false;
    dragCounter = 0;

    const files = e.dataTransfer?.files;
    if (files && files.length > 0) {
      dispatch('upload', files);
    }
  }
</script>

<div 
  class="upload-area"
  class:dragging={isDragging}
  on:dragenter={handleDragEnter}
  on:dragover={handleDragOver}
  on:dragleave={handleDragLeave}
  on:drop={handleDrop}
  role="button"
  tabindex="0"
  on:keydown={(e) => e.key === 'Enter' && triggerUpload()}
>
  <input 
    bind:this={fileInput} 
    type="file" 
    multiple 
    accept=".xmap" 
    on:change={handleChange}
    style="display: none;"
  />

  <div class="upload-content">
    <svg class="upload-icon" width="64" height="64" viewBox="0 0 64 64" fill="none">
      <path d="M32 20v24M20 32l12-12 12 12" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M20 48h24" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
    </svg>
    
    <h3>Drop XMAP files here</h3>
    <p class="upload-hint">or</p>
    
    <button type="button" class="upload-button" on:click={triggerUpload}>
      Select Files from Computer
    </button>
    
    <p class="upload-requirement">2-3 XMAP files required</p>
  </div>
</div>

<style>
  .upload-area {
    position: relative;
    padding: 4rem 2rem;
    background: #f9fafb;
    border-radius: 0.75rem;
    border: 3px dashed #d1d5db;
    cursor: pointer;
    transition: all 0.3s ease;
    min-height: 300px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .upload-area:hover {
    border-color: #3b82f6;
    background: #f0f9ff;
  }

  .upload-area.dragging {
    border-color: #3b82f6;
    background: #dbeafe;
    border-style: solid;
    transform: scale(1.02);
  }

  .upload-area:focus {
    outline: 2px solid #3b82f6;
    outline-offset: 2px;
  }

  .upload-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
    pointer-events: none;
  }

  .upload-icon {
    color: #9ca3af;
    transition: all 0.3s ease;
  }

  .upload-area:hover .upload-icon,
  .upload-area.dragging .upload-icon {
    color: #3b82f6;
  }

  h3 {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 600;
    color: #374151;
  }

  .upload-hint {
    margin: 0;
    font-size: 0.875rem;
    color: #6b7280;
  }

  .upload-requirement {
    margin: 0;
    font-size: 0.75rem;
    color: #9ca3af;
  }

  .upload-button {
    margin-top: 0.5rem;
    padding: 0.75rem 2rem;
    background: #3b82f6;
    color: white;
    border: none;
    border-radius: 0.5rem;
    font-weight: 600;
    font-size: 0.875rem;
    cursor: pointer;
    transition: all 0.2s;
    pointer-events: auto;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }

  .upload-button:hover {
    background: #2563eb;
    transform: translateY(-1px);
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
  }

  .upload-button:active {
    transform: translateY(0);
  }
</style>