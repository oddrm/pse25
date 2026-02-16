export interface Sequence {
  id: sequenceID
  name: string
  // WICHTIG: Änderung von Date zu number (Sekunden)
  startTime: number
  // WICHTIG: Änderung von Date | null zu number (oder number | null)
  endTime: number
  description: string
  entryID: number
  tags: string[] // <--- NEU
}

export type sequenceID = number;