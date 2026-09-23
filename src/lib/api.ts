import { invoke, isTauri } from '@tauri-apps/api/core';
import { open, save } from '@tauri-apps/plugin-dialog';
import type { CaseAnalysis, HexRow } from '../types';

export const native = isTauri();

function requireNative() {
  if (!native) throw new Error('desktop_required');
}

export async function openEvidence(): Promise<string | null> {
  requireNative();
  const path = await open({ multiple: false, directory: false });
  return typeof path === 'string' ? path : null;
}

export async function analyzeFile(path: string): Promise<CaseAnalysis> {
  requireNative();
  return invoke('analyze_file', { path });
}

export async function analyzeText(text: string): Promise<CaseAnalysis> {
  requireNative();
  return invoke('analyze_text', { text });
}

export async function analyzeSample(id: string): Promise<CaseAnalysis> {
  requireNative();
  return invoke('analyze_sample', { id });
}

export async function saveCandidate(analysis: CaseAnalysis, candidateId: string): Promise<boolean> {
  requireNative();
  const source = analysis.fileName?.split(/[\\/]/).pop() ?? 'text';
  const path = await save({
    defaultPath: `${source}.recovered.txt`,
    filters: [{ name: 'UTF-8 text', extensions: ['txt'] }],
  });
  if (!path) return false;
  await invoke('save_candidate', { caseId: analysis.caseId, candidateId, path });
  return true;
}

export async function copyCandidate(caseId: string, candidateId: string): Promise<void> {
  requireNative();
  const text = await invoke<string>('copy_candidate', { caseId, candidateId });
  await navigator.clipboard.writeText(text);
}

export async function readHex(caseId: string, offset: number): Promise<HexRow[]> {
  requireNative();
  return invoke('read_hex', { caseId, offset });
}
