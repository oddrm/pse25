export interface Sequence {
  id: number;
  entry_id: number;
  description: string;
  start_timestamp: number;
  end_timestamp: number;
  created_at: string;
  updated_at: string;

  // Local-only fields (not persisted in backend currently)
  name?: string;
  tags?: string[];
}

export interface SequenceWeb {
  description: string;
  start_timestamp: number;
  end_timestamp: number;
}

export type sequenceID = number;