import { ArrowRight, Binary, Check, ChevronRight, Type } from 'lucide-react';
import type { CaseAnalysis, RecoveryCandidate } from '../types';
import type { Translate } from '../lib/i18n';

export function TransformationStrip({
  analysis,
  candidate,
  onExpand,
  t,
}: {
  analysis: CaseAnalysis;
  candidate: RecoveryCandidate;
  onExpand: () => void;
  t: Translate;
}) {
  const steps = candidate.transformations;
  const firstType = steps[0]?.inputType ?? (analysis.sourceType === 'file' ? 'bytes' : 'text');
  return (
    <section className="transformation-strip" aria-label={t('graph')}>
      <button className="strip-heading" onClick={onExpand}>
        {t('pathShort')}
        <ChevronRight size={12} />
      </button>
      <div className="strip-scroll">
        <span
          className={`strip-node ${firstType}`}
          title={steps[0]?.inputPreview ?? analysis.originalText}
        >
          {firstType === 'bytes' ? <Binary size={13} /> : <Type size={13} />}
          <span>{t(firstType)}</span>
        </span>
        {steps.map((step, index) => (
          <span className="strip-step" key={index}>
            <span className={`strip-edge ${step.lossy ? 'warning-text' : ''}`}>
              <span>
                {step.encoding}
                <small>{t(step.operation)}</small>
              </span>
              <ArrowRight size={13} />
            </span>
            <span
              className={`strip-node ${step.outputType} ${index === steps.length - 1 ? 'strip-final' : ''}`}
              title={step.outputPreview}
            >
              {step.outputType === 'bytes' ? <Binary size={13} /> : <Type size={13} />}
              <span>{t(step.outputType)}</span>
            </span>
          </span>
        ))}
        {steps.length === 0 && (
          <span className="strip-unchanged">
            <Check size={13} />
            {t('unchanged')}
          </span>
        )}
      </div>
    </section>
  );
}
