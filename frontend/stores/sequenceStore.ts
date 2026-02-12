import { defineStore } from "pinia"
import type { Sequence } from "~/utils/sequence"

const STORAGE_KEY = "sequences_v1"

function reviveSequences(raw: any[]): Sequence[] {
  // перетворюємо строки дат назад у Date
  return (raw ?? []).map((s: any) => ({
    ...s,
    startTime: new Date(s.startTime),
    endTime: new Date(s.endTime),
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
          this.sequences = reviveSequences(JSON.parse(saved))
        } catch {
          this.sequences = []
        }
      }

      // автоматично зберігати при будь-якій зміні
      this.$subscribe((_mutation, state) => {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(state.sequences))
      })
    },

    add(payload: Omit<Sequence, "id">) {
      // простий унікальний id
      const id = Date.now() + Math.floor(Math.random() * 1000)

      this.sequences.push({
        id,
        ...payload,
      })
    },

    remove(id: number) {
      this.sequences = this.sequences.filter(s => s.id !== id)
    },

    // (опційно) очистити все
    clearAll() {
      this.sequences = []
    },
  },
})
