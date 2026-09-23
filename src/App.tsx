import { useCallback, useEffect, useRef, useState } from 'react';
import {
  ArrowRight,
  Braces,
  Check,
  ChevronDown,
  Clipboard,
  FileText,
  FolderOpen,
  FlaskConical,
  Languages,
  LoaderCircle,
  LockKeyhole,
  TriangleAlert,
  Upload,
  X,
} from 'lucide-react';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import type { CaseAnalysis, Locale } from './types';
import {
  analyzeFile,
  analyzeSample,
  analyzeText,
  copyCandidate,
  native,
  openEvidence,
  saveCandidate,
} from './lib/api';
import { dictionaries, initialLocale, message, sampleKeys, type Translate } from './lib/i18n';
import { Candidates } from './components/Candidates';
import { Inspector, type InspectorTab } from './components/Inspector';
import { TransformationStrip } from './components/TransformationStrip';

declare global {
  interface Window {
    __MOJIBAKE_PREVIEW__?: CaseAnalysis;
  }
}

export default function App() {
  const [locale, setLocale] = useState<Locale>(initialLocale);
  const [analysis, setAnalysis] = useState<CaseAnalysis | null>(() =>
    import.meta.env.DEV ? (window.__MOJIBAKE_PREVIEW__ ?? null) : null,
  );
  const [selectedId, setSelectedId] = useState('');
  const [tab, setTab] = useState<InspectorTab>('preview');
  const [draft, setDraft] = useState('');
  const [busy, setBusy] = useState(false);
  const [saving, setSaving] = useState(false);
  const [dragging, setDragging] = useState(false);
  const [error, setError] = useState('');
  const [toast, setToast] = useState('');
  const [copied, setCopied] = useState(false);
  const busyRef = useRef(false);
  const editor = useRef<HTMLTextAreaElement>(null);
  const t = useCallback<Translate>((key) => dictionaries[locale][key], [locale]);
  const candidate =
    analysis?.candidates.find((c) => c.id === selectedId) ?? analysis?.candidates[0];

  useEffect(() => {
    localStorage.setItem('mojibake.locale', locale);
    document.documentElement.lang = locale;
    document.title = 'Mojibake Lab';
  }, [locale]);
  useEffect(() => {
    // Clear the retired appearance preference when upgrading an existing profile.
    localStorage.removeItem('mojibake.theme');
    document.documentElement.removeAttribute('data-theme');
  }, []);
  useEffect(() => {
    if (!analysis) editor.current?.focus();
  }, [analysis]);
  useEffect(() => {
    if (!import.meta.env.DEV || !new URLSearchParams(window.location.search).has('preview')) return;
    void fetch('/__preview__/case.json')
      .then((response) => {
        if (!response.ok) throw new Error('Preview fixture unavailable');
        return response.json();
      })
      .then((value: CaseAnalysis) => setAnalysis(value))
      .catch(() => {});
  }, []);
  useEffect(() => {
    if (!toast) return;
    const timer = window.setTimeout(() => {
      setToast('');
      setCopied(false);
    }, 2800);
    return () => window.clearTimeout(timer);
  }, [toast]);

  const run = useCallback(async (operation: () => Promise<CaseAnalysis>) => {
    if (busyRef.current) return;
    busyRef.current = true;
    setBusy(true);
    setError('');
    try {
      const result = await operation();
      setAnalysis(result);
      setSelectedId(result.candidates[0]?.id ?? '');
      setTab('preview');
    } catch (reason) {
      setError(String(reason).replace(/^Error: /, ''));
    } finally {
      busyRef.current = false;
      setBusy(false);
    }
  }, []);
  const openFile = useCallback(async () => {
    if (busyRef.current) return;
    try {
      const path = await openEvidence();
      if (path) await run(() => analyzeFile(path));
    } catch (reason) {
      setError(String(reason).replace(/^Error: /, ''));
    }
  }, [run]);
  const newCase = useCallback(() => {
    if (busyRef.current) return;
    setAnalysis(null);
    setSelectedId('');
    setError('');
    window.setTimeout(() => editor.current?.focus(), 0);
  }, []);
  async function pasteText() {
    newCase();
    try {
      if (navigator.clipboard?.readText) setDraft(await navigator.clipboard.readText());
    } catch {
      /* The focused editor still accepts Ctrl+V if clipboard access is denied. */
    }
    editor.current?.focus();
  }

  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'o') {
        event.preventDefault();
        void openFile();
      }
      if ((event.ctrlKey || event.metaKey) && event.key === 'Enter' && !busyRef.current) {
        event.preventDefault();
        if (!analysis && draft.length) void run(() => analyzeText(draft));
        else if (analysis) newCase();
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [openFile, analysis, draft, run, newCase]);
  useEffect(() => {
    if (!native) return;
    let dispose: (() => void) | undefined,
      cancelled = false;
    void getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type === 'enter' || event.payload.type === 'over') setDragging(true);
        if (event.payload.type === 'leave') setDragging(false);
        if (event.payload.type === 'drop') {
          setDragging(false);
          const path = event.payload.paths[0];
          if (path) void run(() => analyzeFile(path));
        }
      })
      .then((unlisten) => {
        if (cancelled) unlisten();
        else dispose = unlisten;
      })
      .catch((reason) => setError(String(reason)));
    return () => {
      cancelled = true;
      dispose?.();
    };
  }, [run]);

  async function onCopy() {
    if (!analysis || !candidate) return;
    try {
      if (analysis.evidence.sampled) await navigator.clipboard.writeText(candidate.fullText);
      else await copyCandidate(analysis.caseId, candidate.id);
      setCopied(true);
      setToast(t(analysis.evidence.sampled ? 'previewCopied' : 'copied'));
    } catch (reason) {
      setError(String(reason).replace(/^Error: /, ''));
    }
  }
  async function onSave() {
    if (!analysis || !candidate || saving) return;
    setSaving(true);
    try {
      if (await saveCandidate(analysis, candidate.id)) setToast(t('saved'));
    } catch (reason) {
      setError(String(reason).replace(/^Error: /, ''));
    } finally {
      setSaving(false);
    }
  }

  const importantWarnings =
    analysis?.warnings.filter(
      (warning) =>
        ![
          'original_bytes_unavailable',
          'heuristic_not_probability',
          'no_obvious_corruption',
          'sampled_input',
          'sampled_file',
          'ambiguous_result',
        ].includes(warning),
    ) ?? [];
  const fileName =
    analysis?.fileName?.split(/[\\/]/).pop() ||
    t(analysis?.sourceType === 'sample' ? 'sample' : 'pastedText');

  return (
    <div
      className="app"
      onDragOver={(event) => event.preventDefault()}
      onDrop={(event) => event.preventDefault()}
    >
      <header className="app-toolbar">
        <button className="brand" onClick={newCase} title={t('newCase')} aria-label={t('newCase')}>
          <Braces size={20} strokeWidth={1.8} />
        </button>
        <span className="toolbar-divider" />
        <button
          className="toolbar-button"
          onClick={() => void openFile()}
          disabled={busy}
          title="Ctrl / ⌘ O"
        >
          <FolderOpen size={15} />
          {t('open')}
        </button>
        <button className="toolbar-button" onClick={() => void pasteText()} disabled={busy}>
          <Clipboard size={14} />
          {t('paste')}
        </button>
        <label className="toolbar-select sample-select">
          <FlaskConical size={14} />
          <select
            aria-label={t('samples')}
            value=""
            disabled={busy}
            onChange={(event) => {
              if (event.target.value) void run(() => analyzeSample(event.target.value));
            }}
          >
            <option value="">{t('samples')}</option>
            {sampleKeys.map((sample) => (
              <option key={sample.id} value={sample.id}>
                {t(sample.key)}
              </option>
            ))}
          </select>
          <ChevronDown size={11} />
        </label>
        <span className="toolbar-space" />
        <label className="toolbar-select language-select">
          <Languages size={14} />
          <select
            aria-label={t('language')}
            value={locale}
            onChange={(event) => setLocale(event.target.value as Locale)}
          >
            <option value="zh-CN">中文</option>
            <option value="en">EN</option>
            <option value="ja">日本語</option>
          </select>
          <ChevronDown size={10} />
        </label>
      </header>

      {error && (
        <div className="error-banner" role="alert">
          <TriangleAlert size={14} />
          <span>{message(error, t)}</span>
          <button
            className="icon-button"
            onClick={() => setError('')}
            title={t('dismiss')}
            aria-label={t('dismiss')}
          >
            <X size={14} />
          </button>
        </div>
      )}

      {!analysis ? (
        <main className="input-workspace">
          <div className="input-heading">
            <h1>{t('textInput')}</h1>
            <button
              className="example-button"
              onClick={() => void run(() => analyzeSample('cp1252'))}
              disabled={busy}
            >
              <span>ä¸­æ–‡</span>
              <ArrowRight size={12} />
              <span>中文</span>
            </button>
          </div>
          <div className="input-editor">
            <textarea
              ref={editor}
              aria-label={t('pastedText')}
              value={draft}
              onChange={(event) => setDraft(event.target.value)}
              placeholder={t('editorPlaceholder')}
              spellCheck={false}
              disabled={busy}
            />
            <div className="editor-bottom">
              <span>Unicode</span>
              <span>{t('shortcut')}</span>
            </div>
          </div>
          <div className="input-actions">
            <button className="drop-hint" onClick={() => void openFile()} disabled={busy}>
              <Upload size={14} />
              {t('dropHint')}
            </button>
            <button
              className="button primary"
              onClick={() => void run(() => analyzeText(draft))}
              disabled={!draft.length || busy}
            >
              {busy ? <LoaderCircle size={14} className="spin" /> : <ArrowRight size={14} />}
              {t('analyze')}
            </button>
          </div>
        </main>
      ) : (
        <main className="result-workspace">
          <div className="document-heading">
            <FileText size={14} />
            <h1>{fileName}</h1>
            {analysis.evidence.sampled && <span className="sampled-label">{t('sampled')}</span>}
            <span className="toolbar-space" />
            <button
              className="icon-button"
              onClick={newCase}
              aria-label={t('close')}
              title={t('close')}
            >
              <X size={15} />
            </button>
          </div>
          {importantWarnings.length > 0 && (
            <div className="case-warnings">
              <TriangleAlert size={13} />
              <span>{importantWarnings.map((warning) => message(warning, t)).join(' ')}</span>
            </div>
          )}
          {candidate ? (
            <div className="result-shell">
              <div className="result-body">
                <Inspector
                  analysis={analysis}
                  candidate={candidate}
                  tab={tab}
                  setTab={setTab}
                  onSelect={setSelectedId}
                  onCopy={() => void onCopy()}
                  onSave={() => void onSave()}
                  saving={saving}
                  copied={copied}
                  t={t}
                />
                <Candidates
                  candidates={analysis.candidates}
                  selectedId={candidate.id}
                  onSelect={setSelectedId}
                  t={t}
                />
              </div>
              <TransformationStrip
                analysis={analysis}
                candidate={candidate}
                onExpand={() => setTab('graph')}
                t={t}
              />
            </div>
          ) : (
            <div className="no-candidates">
              <TriangleAlert size={27} strokeWidth={1.4} />
              <h2>{t(analysis.evidence.binary ? 'binary' : 'noCandidates')}</h2>
              <p>{t(analysis.evidence.binary ? 'binaryDetail' : 'emptyInputHint')}</p>
              <button className="button" onClick={() => void openFile()}>
                <FolderOpen size={14} />
                {t('open')}
              </button>
            </div>
          )}
        </main>
      )}

      <footer className="app-footer">
        <span>
          <LockKeyhole size={10} />
          {t('local')}
        </span>
        <span>
          {busy
            ? t('analyzing')
            : analysis?.warnings.includes('no_obvious_corruption')
              ? t('cleanNotice')
              : ''}
        </span>
      </footer>
      {busy && (
        <div className="busy-indicator" role="status">
          <LoaderCircle className="spin" size={15} />
          {t('analyzing')}
        </div>
      )}
      {dragging && (
        <div className="drag-overlay">
          <Upload size={28} strokeWidth={1.4} />
          <span>{t('dropOverlay')}</span>
        </div>
      )}
      {toast && (
        <div className="toast" role="status">
          <Check size={14} />
          {toast}
        </div>
      )}
    </div>
  );
}
