import { defineStore } from "pinia"
import type { Sequence } from "~/utils/sequence"
import { fetchSequences } from "~/utils/dbQueries" // [Neu] Import für Mock-Daten

const STORAGE_KEY = "sequences_v1"

type RawSequence = Omit<Sequence, "startTime" | "endTime"> & {
  startTime: string
  endTime: string | null
}

function reviveSequences(raw: RawSequence[] = []): Sequence[] {
  return raw.map((s) => ({
    ...s,
    startTime: new Date(s.startTime),
    endTime: s.endTime ? new Date(s.endTime) : null,
  }))
}

export const useSequencesStore = defineStore("sequences", {
  state: () => ({
    sequences: [] as Sequence[],
    _inited: false,
  }),

  getters: {
    byEntry: (state) => {
      return (entryID: number) => state.sequences.filter(s => s.entryID === entryID)
    },
  },

  actions: {
    init() {
      if (this._inited) return
      this._inited = true

      if (!process.client) return

      const saved = localStorage.getItem(STORAGE_KEY)

      if (saved) {
        try {
          // [Geändert] Keine "revive"-Funktion mehr nötig, da Zahlen (Sekunden)
          // beim Parsen Zahlen bleiben.
          this.sequences = JSON.parse(saved)
        } catch {
          this.sequences = []
        }
      } else {
        // [Neu] Wenn LocalStorage leer ist (erster Besuch), lade Mock-Daten!
        console.log("Lade Mock-Sequenzen...")
        const seq1 = fetchSequences(1)
        const seq2 = fetchSequences(2)
        this.sequences = [...seq1, ...seq2]
      }

      // [Beibehalten] Automatisch speichern bei Änderungen
      this.$subscribe((_mutation, state) => {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(state.sequences))
      })
    },

    add(payload: Omit<Sequence, "id">) {
      // [Beibehalten] Generiert eine einfache ID
      const id = Date.now() + Math.floor(Math.random() * 1000)

      this.sequences.push({
        id,
        ...payload,
        tags: payload.tags || []
      })
    },

    // [Neu] Wichtig für den Bearbeiten-Button
    update(updatedSeq: Sequence) {
      const index = this.sequences.findIndex(s => s.id === updatedSeq.id)
      if (index !== -1) {
        // Ersetzt das alte Objekt durch das bearbeitete
        this.sequences[index] = updatedSeq
      }
    },

    remove(id: number) {
      this.sequences = this.sequences.filter(s => s.id !== id)
    },

    clearAll() {
      this.sequences = []
    },
  },
})