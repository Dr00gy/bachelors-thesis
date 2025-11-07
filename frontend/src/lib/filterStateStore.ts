import { writable } from 'svelte/store';

/**
 * Interface for donut chart filter state
 */
export interface DonutFilterState {
  selectedQueryContigId: string;
  selectedGenome1: string;
  selectedGenome2: string;
  selectedChromosome: string;
  selectedGenomeForChromosome: string;
  showDuplicates: boolean;
  scale: number;
}

/**
 * Create a persistent filter state store
 */
function createFilterStateStore() {
  const defaultState: DonutFilterState = {
    selectedQueryContigId: '',
    selectedGenome1: '',
    selectedGenome2: '',
    selectedChromosome: '',
    selectedGenomeForChromosome: '',
    showDuplicates: false,
    scale: 1.0
  };

  const { subscribe, set, update } = writable<DonutFilterState>(defaultState);

  return {
    subscribe,
    set,
    update,
    reset: () => set(defaultState)
  };
}

export const donutFilterState = createFilterStateStore();