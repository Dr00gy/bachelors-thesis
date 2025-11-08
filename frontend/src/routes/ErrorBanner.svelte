<script lang="ts">
  /**
   * Displays a styled error message
   * @prop error - The error message to display
   */
  export let error: string;
</script>

<div class="error">{error}</div>

<style>
  .error {
    margin-top: 1rem;
    padding: 0.75rem;
    background: #fef2f2;
    color: #dc2626;
    border-radius: 0.375rem;
    border: 1px solid #fecaca;
  }
</style>
