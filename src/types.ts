export type Locale = 'zh-CN' | 'en' | 'ja';
export type DataKind = 'bytes' | 'text';
export interface TransformationStep {
  operation: 'encode' | 'decode';
  encoding: string;
  inputType: DataKind;
  outputType: DataKind;
  inputPreview: string;
  outputPreview: string;
  inputSize: number;
  outputSize: number;
  lossy: boolean;
  replacementCount: number;
}
export interface ScoreBreakdown {
  base: number;
  reversibility: number;
  unicodeValidity: number;
  replacementPenalty: number;
  controlPenalty: number;
  scriptCoherence: number;
  mojibakePenalty: number;
  roundTrip: number;
  depthPenalty: number;
  preservation: number;
}
export interface RecoveryCandidate {
  id: string;
  preview: string;
  fullText: string;
  score: number;
  scoreBreakdown: ScoreBreakdown;
  reversible: boolean;
  lossy: boolean;
  depth: number;
  transformations: TransformationStep[];
  warnings: string[];
  reasons: string[];
  alternativePaths: number;
}
export interface EncodingEvidence {
  bom: string | null;
  validUtf8: boolean | null;
  nullBytePattern: string;
  replacementCount: number;
  controlRatio: number;
  asciiRatio: number;
  whitespaceRatio: number;
  lineCount: number;
  scriptDistribution: Record<string, number>;
  detectorSuggestion: string | null;
  hexPrefix: string;
  sampled: boolean;
  sampleSize: number;
  binary: boolean;
}
export interface CaseAnalysis {
  caseId: string;
  sourceType: 'file' | 'paste' | 'sample';
  fileName: string | null;
  fileSize: number | null;
  originalText: string;
  evidence: EncodingEvidence;
  candidates: RecoveryCandidate[];
  warnings: string[];
  exploredStates: number;
  elapsedMs: number;
}
export interface SampleCase {
  id: string;
  name: string;
  description: string;
}
export interface HexRow {
  offset: number;
  hex: string;
  ascii: string;
}
