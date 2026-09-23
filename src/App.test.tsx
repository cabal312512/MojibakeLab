import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import '@testing-library/jest-dom/vitest';
import App from './App';
import type { CaseAnalysis } from './types';
import * as api from './lib/api';

vi.mock('./lib/api', () => ({
  native: false,
  analyzeFile: vi.fn(),
  analyzeSample: vi.fn(),
  analyzeText: vi.fn(),
  copyCandidate: vi.fn(),
  openEvidence: vi.fn(),
  saveCandidate: vi.fn(),
  readHex: vi.fn(),
}));

function fixture(sampled = false): CaseAnalysis {
  const scoreBreakdown = {
    base: 100,
    reversibility: 12,
    unicodeValidity: 0,
    replacementPenalty: 0,
    controlPenalty: 0,
    scriptCoherence: 2,
    mojibakePenalty: 0,
    roundTrip: 5,
    depthPenalty: -2,
    preservation: 0,
  };
  return {
    caseId: 'case-1',
    sourceType: 'paste',
    fileName: null,
    fileSize: null,
    originalText: 'ä¸­æ–‡',
    exploredStates: 18,
    elapsedMs: 8,
    evidence: {
      bom: null,
      validUtf8: null,
      nullBytePattern: 'none',
      replacementCount: 0,
      controlRatio: 0,
      asciiRatio: 0,
      whitespaceRatio: 0,
      lineCount: 1,
      scriptDistribution: { Latin: 1 },
      detectorSuggestion: null,
      hexPrefix: '',
      sampled,
      sampleSize: 12,
      binary: false,
    },
    warnings: ['original_bytes_unavailable'],
    candidates: [
      {
        id: 'restored',
        preview: '中文…',
        fullText: '中文',
        score: 117,
        scoreBreakdown,
        reversible: true,
        lossy: false,
        depth: 1,
        warnings: [],
        reasons: ['reversible_path', 'no_replacement_characters'],
        alternativePaths: 0,
        transformations: [
          {
            operation: 'encode',
            encoding: 'Windows-1252',
            inputType: 'text',
            outputType: 'bytes',
            inputPreview: 'ä¸­æ–‡',
            outputPreview: 'E4 B8 AD E6 96 87',
            inputSize: 13,
            outputSize: 6,
            lossy: false,
            replacementCount: 0,
          },
          {
            operation: 'decode',
            encoding: 'UTF-8',
            inputType: 'bytes',
            outputType: 'text',
            inputPreview: 'E4 B8 AD E6 96 87',
            outputPreview: '中文',
            inputSize: 6,
            outputSize: 6,
            lossy: false,
            replacementCount: 0,
          },
        ],
      },
      {
        id: 'original',
        preview: 'ä¸­æ–‡',
        fullText: 'ä¸­æ–‡',
        score: 86,
        scoreBreakdown: { ...scoreBreakdown, mojibakePenalty: -31 },
        reversible: true,
        lossy: false,
        depth: 0,
        transformations: [],
        warnings: [],
        reasons: ['preserved_original'],
        alternativePaths: 0,
      },
    ],
  };
}

beforeEach(() => {
  localStorage.clear();
  localStorage.setItem('mojibake.locale', 'en');
  delete window.__MOJIBAKE_PREVIEW__;
  vi.clearAllMocks();
  Object.defineProperty(window, 'matchMedia', {
    configurable: true,
    value: vi
      .fn()
      .mockReturnValue({ matches: false, addEventListener: vi.fn(), removeEventListener: vi.fn() }),
  });
  Object.defineProperty(navigator, 'clipboard', {
    configurable: true,
    value: { writeText: vi.fn().mockResolvedValue(undefined) },
  });
});
afterEach(() => {
  cleanup();
  delete window.__MOJIBAKE_PREVIEW__;
});

describe('workspace', () => {
  it('starts directly in the text editor and removes the retired theme preference', () => {
    localStorage.setItem('mojibake.theme', 'dark');
    render(<App />);
    expect(screen.getByRole('textbox', { name: 'Pasted text' })).toHaveFocus();
    expect(screen.queryByRole('button', { name: 'Dark' })).not.toBeInTheDocument();
    expect(localStorage.getItem('mojibake.theme')).toBeNull();
    fireEvent.change(screen.getByRole('combobox', { name: 'Language' }), {
      target: { value: 'ja' },
    });
    expect(screen.getByRole('heading', { name: '調べるテキスト' })).toBeInTheDocument();
    expect(localStorage.getItem('mojibake.locale')).toBe('ja');
    expect(document.documentElement).not.toHaveAttribute('data-theme');
  });

  it('preserves the exact pasted Unicode text including whitespace', async () => {
    vi.mocked(api.analyzeText).mockResolvedValue(fixture());
    render(<App />);
    const textbox = screen.getByRole('textbox', { name: 'Pasted text' });
    const input = '  ä¸­æ–‡\n';
    fireEvent.change(textbox, { target: { value: input } });
    fireEvent.keyDown(textbox, { key: 'Enter', ctrlKey: true });
    await waitFor(() => expect(api.analyzeText).toHaveBeenCalledWith(input));
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    expect(screen.getByRole('region', { name: 'Transformation path' })).toBeInTheDocument();
  });

  it('retains the case and translates an action error', async () => {
    window.__MOJIBAKE_PREVIEW__ = fixture();
    vi.mocked(api.openEvidence).mockRejectedValue(new Error('permission_denied'));
    render(<App />);
    fireEvent.click(screen.getByRole('button', { name: 'Open file' }));
    await waitFor(() =>
      expect(screen.getByRole('alert')).toHaveTextContent(
        'Permission to access this file was denied.',
      ),
    );
    expect(screen.getByRole('region', { name: 'Transformation path' })).toBeInTheDocument();
  });

  it('loads bundled samples from the compact toolbar selector', async () => {
    vi.mocked(api.analyzeSample).mockResolvedValue(fixture());
    render(<App />);
    fireEvent.change(screen.getByRole('combobox', { name: 'Samples' }), {
      target: { value: 'cp1252' },
    });
    await waitFor(() => expect(api.analyzeSample).toHaveBeenCalledWith('cp1252'));
    expect(await screen.findByRole('tab', { name: 'Result' })).toHaveAttribute(
      'aria-selected',
      'true',
    );
  });
});

describe('candidate inspection', () => {
  it('changes the highlighted typed path when another candidate is selected', () => {
    window.__MOJIBAKE_PREVIEW__ = fixture();
    const { container } = render(<App />);
    expect(container.querySelectorAll('.strip-node.bytes')).toHaveLength(1);
    expect(container.querySelectorAll('.strip-node.text')).toHaveLength(2);
    expect(container.querySelector('.graph-panel')).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('tab', { name: 'Path' }));
    expect(container.querySelectorAll('.graph-active-path .graph-node.bytes')).toHaveLength(1);
    expect(container.querySelectorAll('.graph-active-path .graph-node.text')).toHaveLength(2);
    const candidateButtons = container.querySelectorAll<HTMLButtonElement>('.candidate-card');
    fireEvent.click(candidateButtons[1]);
    expect(candidateButtons[1]).toHaveAttribute('aria-pressed', 'true');
    expect(container.querySelectorAll('.graph-active-path .graph-node')).toHaveLength(1);
    expect(container.querySelector('.unchanged-path')).toHaveTextContent('Original text retained');
    expect(container.querySelectorAll('.alternative-node')).toHaveLength(1);
    expect(container.querySelector('.strip-unchanged')).toHaveTextContent('Original text retained');
  });

  it('copies exact sampled text rather than a preview containing display ellipsis', async () => {
    window.__MOJIBAKE_PREVIEW__ = fixture(true);
    render(<App />);
    fireEvent.click(screen.getByRole('button', { name: 'Copy preview' }));
    await waitFor(() => expect(navigator.clipboard.writeText).toHaveBeenCalledWith('中文'));
    expect(api.copyCandidate).not.toHaveBeenCalled();
    expect(screen.getByRole('status')).toHaveTextContent('Preview copied');
  });

  it('compares exact candidate text without synthetic preview punctuation', () => {
    const value = fixture();
    value.originalText = '中文';
    window.__MOJIBAKE_PREVIEW__ = value;
    const { container } = render(<App />);
    fireEvent.click(screen.getByRole('tab', { name: 'Compare' }));
    expect(container.querySelectorAll('.changed-line')).toHaveLength(0);
    expect(container.querySelector('.compare-grid')).not.toHaveTextContent('…');
  });

  it('localizes explanations and never renders scores as a probability', () => {
    localStorage.setItem('mojibake.locale', 'zh-CN');
    window.__MOJIBAKE_PREVIEW__ = fixture();
    const { container } = render(<App />);
    fireEvent.click(screen.getByRole('tab', { name: '评分' }));
    expect(screen.getByText('没有替换字符')).toBeInTheDocument();
    expect(container.querySelector('.candidate-score')).toHaveTextContent('117');
    expect(container.querySelector('.candidate-score')).not.toHaveTextContent('%');
    const panel = screen.getByRole('tabpanel');
    expect(within(panel).getByText('启发式评分')).toBeInTheDocument();
  });

  it('navigates details with arrow keys while keeping the conversion strip visible', () => {
    window.__MOJIBAKE_PREVIEW__ = fixture();
    render(<App />);
    const resultTab = screen.getByRole('tab', { name: 'Result' });
    resultTab.focus();
    fireEvent.keyDown(resultTab, { key: 'ArrowRight' });
    expect(screen.getByRole('tab', { name: 'Compare' })).toHaveFocus();
    expect(screen.getByRole('tab', { name: 'Compare' })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByRole('region', { name: 'Transformation path' })).toBeVisible();
  });
});
