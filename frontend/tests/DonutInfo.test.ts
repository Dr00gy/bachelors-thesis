import { render } from '@testing-library/svelte';
import { describe, it, expect } from 'vitest';
import DonutInfo from '../src/routes/DonutInfo.svelte';
import { mockBackendMatch } from './__mocks__/bincodeMock';

describe('DonutInfo.svelte', () => {
  it('shows genome count based on files and renders core sections', () => {
    const files = [
      { name: 'genome1.xmap', rows: 100, color: '#3b82f6' },
      { name: 'genome2.xmap', rows: 200, color: '#10b981' }
    ];

    const { getByText } = render(DonutInfo, {
      props: {
        files,
        matches: [mockBackendMatch], // mock has file_indices [0,1]
      }
    });

    expect(getByText(/Genomes\s*\(2\)/i)).toBeTruthy();

    // Sanity sections that are always in DOM dump
    expect(getByText('Filters')).toBeTruthy();
    expect(getByText('Debug Info')).toBeTruthy();
  });
});
