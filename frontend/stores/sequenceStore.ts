import { defineStore } from "pinia"
import type { Sequence, SequenceWeb } from "~/utils/sequence"
import { fetchSequences, fetchEntries } from "~/utils/dbQueries"
import { addSequence, updateSequence, removeSequence } from "~/utils/dbQueries"
import { Sorting } from "~/utils/entryColumns"

export const useSequencesStore = defineStore("sequences", {
  state: () => ({
    sequences: [] as Sequence[],
    _inited: false,
    // track which entry IDs we've already loaded sequences for
    loaded_entry_ids: [] as number[],
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
    },

    // Load sequences only for the provided entry IDs. If `force` is true,
    // refetch even when we've loaded them before. For multi-user edits we
    // prefer fresh data and avoid long-lived caching.
    async loadForEntries(entryIDs: number[], force = false) {
      if (!process.client) return
      const uniqueIDs = Array.from(new Set(entryIDs))
      const toLoad = force ? uniqueIDs : uniqueIDs.filter(id => !this.loaded_entry_ids.includes(id))
      if (toLoad.length === 0) return

      for (const id of toLoad) {
        try {
          const map = await fetchSequences(id)
          const values = Object.values(map) as Sequence[]
          // Replace any sequences for this entry to avoid stale duplicates
          this.sequences = this.sequences.filter(s => s.entry_id !== id)
          for (const s of values) {
            this.sequences.push(s)
          }
          // mark as loaded
          if (!this.loaded_entry_ids.includes(id)) this.loaded_entry_ids.push(id)
        } catch (err) {
          console.error(`[SequenceStore] Error loading sequences for entry ${id}:`, err)
        }
      }
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
        // Refresh authoritative state for this entry (multi-user safety)
        await this.loadForEntries([payload.entry_id], true)
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
        // Refresh authoritative state for this entry (multi-user safety)
        await this.loadForEntries([updatedSeq.entry_id], true)
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
        // Refresh authoritative state for this entry (multi-user safety)
        await this.loadForEntries([seq.entry_id], true)
      } catch (err) {
        console.error('[SequenceStore] Error removing sequence:', err)
      }
    },

    clearAll() {
      this.sequences = []
    },
  },
})