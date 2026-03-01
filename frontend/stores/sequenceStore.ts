import { defineStore } from "pinia"
import type { Sequence, SequenceWeb } from "~/utils/sequence"
import { fetchSequences, fetchEntries } from "~/utils/dbQueries"
import { addSequence, updateSequence, removeSequence } from "~/utils/dbQueries"
import { Sorting } from "~/utils/entryColumns"

const STORAGE_KEY = "sequences_v1"

export const useSequencesStore = defineStore("sequences", {
  state: () => ({
    sequences: [] as Sequence[],
    _inited: false,
  }),

  getters: {
    byEntry: (state) => {
      return (entry_id: number) => state.sequences.filter(s => s.entry_id === entry_id)
    },
  },

  actions: {
    async init() {
      if (this._inited) return
      this._inited = true

      if (!process.client) return

      // Load from backend as primary source of truth
      try {
        // TODO match with rest of search
        const [entries, num_pages] = await fetchEntries('', Sorting.Name, true, 0, 50000)
        let allSeqs: Sequence[] = []
        for (const e of entries) {
          const map = await fetchSequences(e.id)
          const values = Object.values(map) as Sequence[]
          values.forEach((s) => {
            if (!allSeqs.some(existing => existing.id === s.id)) {
              allSeqs.push(s)
            }
          })
        }
        this.sequences = allSeqs
      } catch (err) {
        console.error("Error loading sequences from backend:", err)
        // Fallback to localStorage if backend fails
        const saved = localStorage.getItem(STORAGE_KEY)
        if (saved) {
          try {
            const parsed = JSON.parse(saved)
            this.sequences = parsed.map((s: any) => ({
              ...s,
              entry_id: s.entry_id || s.entryID,
              id: s.id ?? 0
            }))
          } catch {
            this.sequences = []
          }
        }
      }

      // Automatically save to localStorage when changes occur
      this.$subscribe((_mutation, state) => {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(state.sequences))
      })
    },

    async add(payload: Omit<Sequence, "id" | "created_at" | "updated_at">) {
      try {
        const webPayload: SequenceWeb = {
          description: payload.description,
          start_timestamp: payload.start_timestamp,
          end_timestamp: payload.end_timestamp,
          tags: payload.tags || []
        }
        const newId = await addSequence(payload.entry_id, webPayload)

        // Prevent duplicate IDs in store
        if (this.sequences.some(s => s.id === newId)) {
          console.warn(`[SequenceStore] Sequence with ID ${newId} already exists in store.`);
          return
        }

        this.sequences.push({
          id: newId,
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
          ...payload
        })
      } catch (err) {
        console.error('Error adding sequence:', err)
      }
    },

    // [Neu] Wichtig für den Bearbeiten-Button
    async update(updatedSeq: Sequence) {
      const index = this.sequences.findIndex(s => s.id === updatedSeq.id)
      if (index === -1) return
      try {
        const webPayload: SequenceWeb = {
          description: updatedSeq.description,
          start_timestamp: updatedSeq.start_timestamp,
          end_timestamp: updatedSeq.end_timestamp,
          tags: updatedSeq.tags || []
        }
        await updateSequence(updatedSeq.entry_id, updatedSeq.id, webPayload)
        this.sequences[index] = {
          ...updatedSeq,
          updated_at: new Date().toISOString()
        }
      } catch (err) {
        console.error('Error updating sequence:', err)
      }
    },

    async remove(id: number) {
      if (id === undefined || id === null) {
        return
      }
      const seq = this.sequences.find(s => s.id === id)
      if (!seq) {
        return
      }

      try {
        await removeSequence(seq.entry_id, id)
        this.sequences = this.sequences.filter(s => s.id !== id)
      } catch (err) {
        console.error('[SequenceStore] Error removing sequence:', err)
      }
    },

    clearAll() {
      this.sequences = []
    },
  },
})