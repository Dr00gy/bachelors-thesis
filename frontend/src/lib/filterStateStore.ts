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
 * Interface for area analysis filter state
 */
export interface AreaAnalysisFilterState {
  selectedFiles: number[];
  selectedChromosome: number;
  windowSize: number;
  currentWindowIndex: number;
}

/**
 * Create a persistent filter state store for donut
 */
function createDonutFilterStateStore() {
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

/**
 * Create a persistent filter state store for area analysis
 */
function createAreaAnalysisFilterStateStore() {
  const defaultState: AreaAnalysisFilterState = {
    selectedFiles: [0],
    selectedChromosome: 1,
    windowSize: 100000,
    currentWindowIndex: 0
  };

  const { subscribe, set, update } = writable<AreaAnalysisFilterState>(defaultState);

  return {
    subscribe,
    set,
    update,
    reset: () => set(defaultState)
  };
}

export const donutFilterState = createDonutFilterStateStore();
export const areaAnalysisFilterState = createAreaAnalysisFilterStateStore();