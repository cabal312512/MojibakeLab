import { describe, expect, it, vi } from 'vitest';
import { analyzeFile, analyzeSample, analyzeText } from './api';
import { invoke } from '@tauri-apps/api/core';

vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => false, invoke: vi.fn() }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn(), save: vi.fn() }));

describe('native evidence boundary', () => {
  it('does not silently run a browser substitute for the encoding engine', async () => {
    await expect(analyzeText('ä¸­æ–‡')).rejects.toThrow('desktop_required');
    await expect(analyzeFile('evidence.txt')).rejects.toThrow('desktop_required');
    await expect(analyzeSample('cp1252')).rejects.toThrow('desktop_required');
    expect(invoke).not.toHaveBeenCalled();
  });
});
