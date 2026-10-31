/**
 * RetinaCare Clinical Screening Suite Contracts
 */

export type DiabeticRetinopathyGrade =
  | 'No DR'
  | 'Mild DR'
  | 'Moderate DR'
  | 'Severe DR'
  | 'Proliferative DR';

export interface DoctorProfile {
  name: string;
  role: string;
  avatarUrl: string;
}

export interface DiagnosticMetrics {
  microaneurysms: number;
  hemorrhages: number;
  exudates: number;
}

export interface RetinalExaminationRecord {
  id: string;
  date: string;
  patientName: string;
  patientId: string;
  eye: 'OD' | 'OS';
  severityGrade: DiabeticRetinopathyGrade;
  confidence: number;
  clinicalSummary: string;
  metrics: DiagnosticMetrics;
  thumbnail: string;
}

export interface FundusAnalysisMode {
  id: string;
  title: string;
  subtitle?: string;
  badge?: string;
  type: 'original' | 'lesion' | 'vessel' | 'gradcam';
  imageUrl: string;
}
