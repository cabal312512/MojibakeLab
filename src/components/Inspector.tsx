import { useEffect, useState } from 'react';
import {
  ArrowLeft,
  ArrowRight,
  Check,
  Copy,
  Download,
  Info,
  LoaderCircle,
  ShieldCheck,
  TriangleAlert,
} from 'lucide-react';
import type { CaseAnalysis, HexRow, RecoveryCandidate } from '../types';
import { readHex } from '../lib/api';
import { message, type MessageKey, type Translate } from '../lib/i18n';
import { Evidence } from './Evidence';
import { PathGraph } from './Graph';
import { CandidateStatus } from './Candidates';

export type InspectorTab = 'preview' | 'compare' | 'graph' | 'evidence' | 'why' | 'hex';

function CompareText({
  text,
  other,
  t,
  title,
}: {
  text: string;
  other: string;
  t: Translate;
  title: 'original' | 'restored';
}) {
  const lines = text.split('\n'),
    otherLines = other.split('\n');
  return (
    <div className="compare-side">
      <div className="compare-heading">{t(title)}</div>
      <pre className="numbered-text">
        {lines.map((line, index) => (
          <span
            className={`text-line ${line !== otherLines[index] ? 'changed-line' : ''}`}
            key={index}
          >
            <span className="line-number">{index + 1}</span>
            <span>{line || '\u00a0'}</span>
          </span>
        ))}
      </pre>
    </div>
  );
}

function HexInspector({ analysis, t }: { analysis: CaseAnalysis; t: Translate }) {
  const [offset, setOffset] = useState(0);
  const [rows, setRows] = useState<HexRow[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  useEffect(() => {
    if (analysis.sourceType !== 'file') return;
    let active = true;
    setLoading(true);
    setError('');
    readHex(analysis.caseId, offset)
      .then((value) => {
        if (active) setRows(value);
      })
      .catch((reason) => {
        if (active) setError(String(reason));
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [analysis.caseId, analysis.sourceType, offset]);
  if (analysis.sourceType === 'paste') return <div className="inspector-empty">{t('noHex')}</div>;
  if (analysis.sourceType === 'sample')
    return (
      <div className="sample-hex">
        <span>
          {t('observed')} · {t('truncated')}
        </span>
        <pre>{analysis.evidence.hexPrefix || t('noHex')}</pre>
      </div>
    );
  const canNext =
    analysis.fileSize === null ? rows.length >= 256 : offset + 4096 < analysis.fileSize;
  return (
    <div className="hex-inspector">
      <div className="hex-toolbar">
        <span className="mono">0x{offset.toString(16).toUpperCase().padStart(8, '0')}</span>
        <span>4 KB</span>
        <button
          className="icon-button"
          disabled={offset === 0 || loading}
          title={t('previous')}
          aria-label={t('previous')}
          onClick={() => setOffset(Math.max(0, offset - 4096))}
        >
          <ArrowLeft size={14} />
        </button>
        <button
          className="icon-button"
          disabled={!canNext || loading}
          title={t('next')}
          aria-label={t('next')}
          onClick={() => setOffset(offset + 4096)}
        >
          <ArrowRight size={14} />
        </button>
      </div>
      {loading ? (
        <div className="inspector-empty">
          <LoaderCircle size={18} className="spin" />
        </div>
      ) : error ? (
        <div className="inline-error">{message(error.replace(/^Error: /, ''), t)}</div>
      ) : (
        <div className="hex-scroll">
          <table className="hex-table">
            <thead>
              <tr>
                <th>{t('offset')}</th>
                <th>00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F</th>
                <th>ASCII</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((row) => (
                <tr key={row.offset}>
                  <td>{row.offset.toString(16).toUpperCase().padStart(8, '0')}</td>
                  <td>{row.hex}</td>
                  <td>{row.ascii}</td>
                </tr>
              ))}
            </tbody>
          </table>
          {rows.length === 0 && <p className="inspector-empty">{t('noData')}</p>}
        </div>
      )}
    </div>
  );
}

export function Inspector({
  analysis,
  candidate,
  tab,
  setTab,
  onSelect,
  onCopy,
  onSave,
  saving,
  copied,
  t,
}: {
  analysis: CaseAnalysis;
  candidate: RecoveryCandidate;
  tab: InspectorTab;
  setTab: (tab: InspectorTab) => void;
  onSelect: (id: string) => void;
  onCopy: () => void;
  onSave: () => void;
  saving: boolean;
  copied: boolean;
  t: Translate;
}) {
  const breakdown = Object.entries(candidate.scoreBreakdown).filter(([, value]) => value !== 0);
  const tabs: Array<{ id: InspectorTab; label: MessageKey }> = [
    { id: 'preview', label: 'resultShort' },
    { id: 'compare', label: 'compare' },
    { id: 'graph', label: 'pathShort' },
    { id: 'evidence', label: 'evidenceShort' },
    { id: 'why', label: 'score' },
    { id: 'hex', label: 'hexShort' },
  ];
  return (
    <section className="inspector-panel">
      <div
        className="inspector-tabs"
        role="tablist"
        aria-label={t('details')}
        onKeyDown={(event) => {
          const index = tabs.findIndex((item) => item.id === tab);
          const next =
            event.key === 'ArrowRight'
              ? (index + 1) % tabs.length
              : event.key === 'ArrowLeft'
                ? (index + tabs.length - 1) % tabs.length
                : -1;
          if (next >= 0) {
            event.preventDefault();
            setTab(tabs[next].id);
            event.currentTarget.querySelectorAll<HTMLButtonElement>('button')[next].focus();
          }
        }}
      >
        {tabs.map((item) => (
          <button
            role="tab"
            id={`tab-${item.id}`}
            aria-controls={`panel-${item.id}`}
            aria-selected={tab === item.id}
            tabIndex={tab === item.id ? 0 : -1}
            className={tab === item.id ? 'active' : ''}
            key={item.id}
            onClick={() => setTab(item.id)}
          >
            {t(item.label)}
          </button>
        ))}
      </div>
      <div
        className={`inspector-content content-${tab}`}
        role="tabpanel"
        id={`panel-${tab}`}
        aria-labelledby={`tab-${tab}`}
      >
        {tab === 'preview' && (
          <>
            <pre className="recovered-text">{candidate.preview || '∅'}</pre>
            {analysis.evidence.sampled && (
              <div className="preview-notice">
                <Info size={13} />
                {t('fullCopyUnavailable')}
              </div>
            )}
            {candidate.lossy && (
              <div className="preview-warning">
                <TriangleAlert size={13} />
                {t('lossWarning')}
              </div>
            )}
          </>
        )}
        {tab === 'compare' && (
          <div className="compare-grid">
            <CompareText
              text={analysis.originalText}
              other={candidate.fullText}
              t={t}
              title="original"
            />
            <CompareText
              text={candidate.fullText}
              other={analysis.originalText}
              t={t}
              title="restored"
            />
          </div>
        )}
        {tab === 'graph' && (
          <PathGraph analysis={analysis} candidate={candidate} onSelect={onSelect} t={t} />
        )}
        {tab === 'evidence' && <Evidence analysis={analysis} t={t} />}
        {tab === 'hex' && <HexInspector key={analysis.caseId} analysis={analysis} t={t} />}
        {tab === 'why' && (
          <div className="why-content">
            <div className="why-reasons">
              <h3>{t('whyTitle')}</h3>
              <ul>
                {candidate.reasons.map((reason, index) => (
                  <li key={index}>
                    <ShieldCheck size={13} />
                    {message(reason, t)}
                  </li>
                ))}
              </ul>
              {[...new Set([...candidate.warnings, ...analysis.warnings])]
                .filter(
                  (warning) =>
                    ![
                      'heuristic_not_probability',
                      'original_bytes_unavailable',
                      'sampled_input',
                      'sampled_file',
                      'no_obvious_corruption',
                    ].includes(warning),
                )
                .map((warning, index) => (
                  <p className="warning-text" key={index}>
                    <Info size={13} />
                    {message(warning, t)}
                  </p>
                ))}
              <p className="muted">{t('roundTripNote')}</p>
            </div>
            <div className="score-breakdown">
              {breakdown.map(([key, value]) => (
                <div className="score-row" key={key}>
                  <span>{t(key as MessageKey)}</span>
                  <strong className={value < 0 ? 'warning-text' : ''}>
                    {value > 0 ? '+' : ''}
                    {Number(value.toFixed(1))}
                  </strong>
                </div>
              ))}
              <div className="score-total">
                <span>{t('heuristic')}</span>
                <strong>{Number(candidate.score.toFixed(1))}</strong>
              </div>
              <p>{t('scoreNote')}</p>
            </div>
          </div>
        )}
      </div>
      <div className="result-actions">
        <span title={t('roundTripNote')}>
          <CandidateStatus candidate={candidate} t={t} />
        </span>
        <span className="toolbar-space" />
        <button
          className="button quiet copy-button"
          onClick={onCopy}
          title={t(analysis.evidence.sampled ? 'copyPreview' : 'copy')}
        >
          {copied ? <Check size={13} /> : <Copy size={13} />}
          {t(analysis.evidence.sampled ? 'copyPreview' : 'copy')}
        </button>
        <button className="button primary" onClick={onSave} disabled={saving} title={t('safeSave')}>
          {saving ? <LoaderCircle size={13} className="spin" /> : <Download size={13} />}
          {t('save')}
        </button>
      </div>
    </section>
  );
}
