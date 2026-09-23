import { Binary, FileSearch, Info } from 'lucide-react';
import type { CaseAnalysis } from '../types';
import type { Translate, MessageKey } from '../lib/i18n';

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

const knownScripts = new Set([
  'Han',
  'Hiragana',
  'Katakana',
  'Hangul',
  'Latin',
  'Cyrillic',
  'Common',
  'Other',
]);

export function Evidence({ analysis, t }: { analysis: CaseAnalysis; t: Translate }) {
  const e = analysis.evidence;
  const scripts = Object.entries(e.scriptDistribution)
    .filter(([, value]) => value > 0)
    .sort((a, b) => b[1] - a[1])
    .slice(0, 7);
  const ratio = (n: number) => `${(n * 100).toFixed(1)}%`;
  return (
    <aside className="panel evidence-panel">
      <div className="panel-heading">
        <h2>
          <FileSearch size={15} />
          {t('evidence')}
        </h2>
        <span className="section-index">01</span>
      </div>
      <div className="evidence-scroll">
        <div className="evidence-section">
          <span className="eyebrow">{t('summary')}</span>
          <dl className="evidence-list">
            <div>
              <dt>{t('source')}</dt>
              <dd>
                {t(
                  analysis.sourceType === 'paste'
                    ? 'pastedText'
                    : analysis.sourceType === 'sample'
                      ? 'sample'
                      : 'file',
                )}
              </dd>
            </div>
            {analysis.fileSize !== null && (
              <div>
                <dt>{t('size')}</dt>
                <dd className="mono">{formatBytes(analysis.fileSize)}</dd>
              </div>
            )}
            <div>
              <dt>{t('lineCount')}</dt>
              <dd className="mono">{e.lineCount.toLocaleString()}</dd>
            </div>
          </dl>
        </div>
        <div className="evidence-section">
          <span className="eyebrow">{t('bytes')}</span>
          <dl className="evidence-list">
            <div>
              <dt>{t('bom')}</dt>
              <dd className="mono">{e.bom ?? t('none')}</dd>
            </div>
            <div>
              <dt>{t('utf8')}</dt>
              <dd className={e.validUtf8 ? 'positive' : ''}>
                {e.validUtf8 === null ? '—' : t(e.validUtf8 ? 'yes' : 'no')}
              </dd>
            </div>
            <div>
              <dt>{t('detector')}</dt>
              <dd className="mono">{e.detectorSuggestion ?? '—'}</dd>
            </div>
            {e.nullBytePattern &&
              e.nullBytePattern !== 'none' &&
              e.nullBytePattern !== 'unavailable' && (
                <div className="long-evidence">
                  <dt>{t('nullPattern')}</dt>
                  <dd>{e.nullBytePattern}</dd>
                </div>
              )}
          </dl>
          {e.hexPrefix && <pre className="hex-prefix">{e.hexPrefix.slice(0, 95)}</pre>}
        </div>
        <div className="evidence-section">
          <span className="eyebrow">{t('text')}</span>
          <dl className="evidence-list">
            <div>
              <dt>{t('replacements')}</dt>
              <dd className={`mono ${e.replacementCount > 0 ? 'warning-text' : ''}`}>
                {e.replacementCount.toLocaleString()}
              </dd>
            </div>
            <div>
              <dt>{t('controls')}</dt>
              <dd className="mono">{ratio(e.controlRatio)}</dd>
            </div>
            <div>
              <dt>{t('ascii')}</dt>
              <dd className="mono">{ratio(e.asciiRatio)}</dd>
            </div>
            <div>
              <dt>{t('whitespace')}</dt>
              <dd className="mono">{ratio(e.whitespaceRatio)}</dd>
            </div>
          </dl>
        </div>
        {scripts.length > 0 && (
          <div className="evidence-section script-section">
            <span className="eyebrow">{t('scripts')}</span>
            <div className="script-bar">
              {scripts.map(([name, value], i) => (
                <span
                  key={name}
                  title={`${name}: ${ratio(value)}`}
                  style={{ flex: value, opacity: 1 - i * 0.12 }}
                />
              ))}
            </div>
            <div className="script-list">
              {scripts.map(([name, value]) => (
                <div key={name}>
                  <span>{knownScripts.has(name) ? t(name as MessageKey) : name}</span>
                  <span className="mono">{ratio(value)}</span>
                </div>
              ))}
            </div>
          </div>
        )}
        {(analysis.sourceType === 'paste' || e.validUtf8 === null) && (
          <div className="evidence-note">
            <Info size={14} />
            <p>{t('noRaw')}</p>
          </div>
        )}
        {e.sampled && (
          <div className="evidence-note">
            <Binary size={14} />
            <p>
              <strong>{t('sampled')}</strong>
              {t('sampledDetail')}
            </p>
          </div>
        )}
      </div>
    </aside>
  );
}
