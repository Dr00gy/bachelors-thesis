import { writable } from 'svelte/store';

interface SearchState {
  overviewSearchQuery: string;
  matchesSearchQuery: string;
  areaSearchQuery: string;
  overviewSearchType: 'contig' | 'chromosome' | 'confidence';
  matchesSearchType: 'contig' | 'chromosome' | 'confidence';
}

const initialState: SearchState = {
  overviewSearchQuery: '',
  matchesSearchQuery: '',
  areaSearchQuery: '',
  overviewSearchType: 'contig',
  matchesSearchType: 'contig'
};

export const searchStore = writable<SearchState>(initialState);

export function resetSearchStore() {
  searchStore.set(initialState);
}