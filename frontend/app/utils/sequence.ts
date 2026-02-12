export interface Sequence {
  id: sequenceID
  name: string
  startTime: Date
  endTime: Date | null
  description: string
  entryID: number
}

export type sequenceID = number;