import { Check, ShieldCheck, TriangleAlert } from 'lucide-react';
import type { RecoveryCandidate } from '../types';
import type { Translate } from '../lib/i18n';

export function CandidateStatus({ candidate, t }: { candidate: RecoveryCandidate; t: Translate }) {
  return (
    <span className={`candidate-status ${candidate.lossy ? 'status-loss' : ''}`}>
      {candidate.lossy ? (
        <TriangleAlert size={12} />
      ) : candidate.reversible ? (
        <ShieldCheck size={12} />
      ) : null}
      {t(candidate.lossy ? 'lossy' : candidate.reversible ? 'reversible' : 'uncertain')}
    </span>
  );
}

export function Candidates({
  candidates,
  selectedId,
  onSelect,
  t,
}: {
  candidates: RecoveryCandidate[];
  selectedId: string;
  onSelect: (id: string) => void;
  t: Translate;
}) {
  return (
    <aside className="candidates-panel">
      <div className="candidates-heading">
        <h2>{t('candidates')}</h2>
        <span>{candidates.length}</span>
      </div>
      <div className="candidate-list">
        {candidates.map((candidate) => (
          <button
            className={`candidate-card ${selectedId === candidate.id ? 'active' : ''}`}
            key={candidate.id}
            onClick={() => onSelect(candidate.id)}
            aria-pressed={selectedId === candidate.id}
          >
            <span className="candidate-preview">{candidate.preview || '∅'}</span>
            <span className="candidate-meta">
              <span>{candidate.transformations.at(-1)?.encoding ?? t('unchangedShort')}</span>
              <span className="candidate-score" title={t('heuristic')}>
                {Math.round(candidate.score)}
              </span>
              {selectedId === candidate.id && <Check size={11} />}
            </span>
            {candidate.lossy && (
              <span className="candidate-loss">
                <TriangleAlert size={10} />
                {t('lossy')}
              </span>
            )}
          </button>
        ))}
      </div>
      <p className="candidate-note" title={t('scoreNote')}>
        {t('scoreShort')}
      </p>
    </aside>
  );
}
