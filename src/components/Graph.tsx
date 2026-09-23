import { ArrowDown, Binary, Check, FileText, GitBranch } from 'lucide-react';
import type { CaseAnalysis, RecoveryCandidate, TransformationStep } from '../types';
import type { Translate } from '../lib/i18n';

function Node({
  kind,
  value,
  title,
  final,
  size,
  t,
}: {
  kind: 'bytes' | 'text';
  value: string;
  title: string;
  final?: boolean;
  size?: number;
  t: Translate;
}) {
  return (
    <div className={`graph-node ${kind} ${final ? 'final-node' : ''}`}>
      <div className="node-header">
        <span>
          {kind === 'bytes' ? <Binary size={13} /> : <FileText size={13} />}
          {t(kind)}
        </span>
        <span>{title}</span>
        {final && <Check size={13} />}
      </div>
      <pre>{value.slice(0, 180) || '∅'}</pre>
      {size !== undefined && (
        <span className="node-size">
          {size.toLocaleString()} {kind === 'bytes' ? 'B' : 'UTF-8 B'}
        </span>
      )}
    </div>
  );
}

export function PathGraph({
  analysis,
  candidate,
  onSelect,
  t,
}: {
  analysis: CaseAnalysis;
  candidate: RecoveryCandidate;
  onSelect: (id: string) => void;
  t: Translate;
}) {
  const steps = candidate.transformations;
  const sourceKind = steps[0]?.inputType ?? (analysis.sourceType === 'file' ? 'bytes' : 'text');
  const sourceValue =
    steps[0]?.inputPreview ??
    (sourceKind === 'bytes' ? analysis.evidence.hexPrefix : analysis.originalText);
  const others = analysis.candidates.filter((c) => c.id !== candidate.id);
  return (
    <section className="panel graph-panel">
      <div className="panel-heading">
        <h2>
          <GitBranch size={15} />
          {t('graph')}
        </h2>
        <div className="graph-legend">
          <span className="legend-square bytes-legend" />
          {t('bytes')}
          <span className="legend-square text-legend" />
          {t('text')}
        </div>
      </div>
      <div className={`graph-canvas ${steps.length >= 3 ? 'long-path' : ''}`}>
        <div className="graph-active-path">
          <Node
            kind={sourceKind}
            value={sourceValue}
            title={t('observed')}
            size={steps[0]?.inputSize}
            t={t}
          />
          {steps.map((step: TransformationStep, index) => (
            <div className="graph-stage" key={`${index}-${step.encoding}`}>
              <div className={`graph-edge ${step.lossy ? 'lossy-edge' : ''}`}>
                <span>
                  <span>{t(step.operation)}</span>
                  <strong>{step.encoding}</strong>
                  {step.lossy && <span className="lossy-dot" />}
                </span>
                <ArrowDown size={13} />
              </div>
              <Node
                kind={step.outputType}
                value={step.outputPreview}
                title={index === steps.length - 1 ? t('recovered') : `${t('step')} ${index + 1}`}
                final={index === steps.length - 1}
                size={step.outputSize}
                t={t}
              />
            </div>
          ))}
          {steps.length === 0 && (
            <div className="unchanged-path">
              <Check size={16} />
              <span>
                {t('unchanged')}
                <small>{t('unchangedDetail')}</small>
              </span>
            </div>
          )}
        </div>
        {others.length > 0 && (
          <div className="graph-alternatives">
            <div className="alternative-heading">{t('alternatives')}</div>
            {others.slice(0, 5).map((other) => (
              <button
                key={other.id}
                className="alternative-node"
                onClick={() => onSelect(other.id)}
                title={`${t('selectCandidate')} ${analysis.candidates.indexOf(other) + 1}`}
              >
                <span className="alternative-label">
                  <FileText size={11} />
                  <span>0{analysis.candidates.indexOf(other) + 1}</span>
                  <span>{Math.round(other.score)}</span>
                </span>
                <pre>{other.preview.slice(0, 48)}</pre>
                <span className="alternative-chain">
                  {other.transformations.map((s) => s.encoding).join(' → ') || t('unchanged')}
                </span>
              </button>
            ))}
          </div>
        )}
      </div>
      <div className="panel-footnote">{t('historyNote')}</div>
    </section>
  );
}
