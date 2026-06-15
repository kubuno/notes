import { create } from 'zustand'
import { notesApi, Note, Notebook, Label } from './api'

interface NotesState {
  notes: Note[]
  notebooks: Notebook[]
  labels: Label[]
  activeNotebook: string | null
  activeLabel: string | null
  searchQuery: string
  view: 'all' | 'archived' | 'trashed' | 'pinned'
  isLoading: boolean
  error: string | null

  fetchNotes: () => Promise<void>
  fetchNotebooks: () => Promise<void>
  fetchLabels: () => Promise<void>
  setActiveNotebook: (id: string | null) => void
  setActiveLabel: (id: string | null) => void
  setSearchQuery: (q: string) => void
  setView: (view: NotesState['view']) => void
  createNote: (data?: Partial<Parameters<typeof notesApi.createNote>[0]>) => Promise<Note>
  updateNote: (id: string, data: Parameters<typeof notesApi.updateNote>[1]) => Promise<void>
  trashNote: (id: string) => Promise<void>
  restoreNote: (id: string) => Promise<void>
  deleteNote: (id: string) => Promise<void>
  emptyTrash: () => Promise<void>
}

export const useNotesStore = create<NotesState>((set, get) => ({
  notes: [],
  notebooks: [],
  labels: [],
  activeNotebook: null,
  activeLabel: null,
  searchQuery: '',
  view: 'all',
  isLoading: false,
  error: null,

  fetchNotes: async () => {
    set({ isLoading: true, error: null })
    try {
      const { view, activeNotebook, activeLabel, searchQuery } = get()
      const params: Parameters<typeof notesApi.listNotes>[0] = {}
      if (view === 'archived') params.archived = true
      else if (view === 'trashed') params.trashed = true
      else if (view === 'pinned') params.pinned = true
      if (activeNotebook) params.notebook_id = activeNotebook
      if (activeLabel) params.label_id = activeLabel
      if (searchQuery) params.q = searchQuery
      const notes = await notesApi.listNotes(params)
      set({ notes: notes.notes, isLoading: false })
    } catch (e: unknown) {
      set({ error: (e as Error).message, isLoading: false })
    }
  },

  fetchNotebooks: async () => {
    const notebooks = await notesApi.listNotebooks()
    set({ notebooks })
  },

  fetchLabels: async () => {
    const labels = await notesApi.listLabels()
    set({ labels })
  },

  setActiveNotebook: (id) => {
    set({ activeNotebook: id, activeLabel: null, view: 'all' })
    get().fetchNotes()
  },

  setActiveLabel: (id) => {
    set({ activeLabel: id, activeNotebook: null, view: 'all' })
    get().fetchNotes()
  },

  setSearchQuery: (q) => {
    set({ searchQuery: q })
    get().fetchNotes()
  },

  setView: (view) => {
    set({ view, activeNotebook: null, activeLabel: null })
    get().fetchNotes()
  },

  createNote: async (data = {}) => {
    const { activeNotebook } = get()
    const note = await notesApi.createNote({
      color: 'default',
      notebook_id: activeNotebook ?? undefined,
      ...data,
    })
    set(s => ({ notes: [note, ...s.notes] }))
    return note
  },

  updateNote: async (id, data) => {
    const updated = await notesApi.updateNote(id, data)
    set(s => ({ notes: s.notes.map(n => n.id === id ? updated : n) }))
  },

  trashNote: async (id) => {
    await notesApi.trashNote(id)
    set(s => ({ notes: s.notes.filter(n => n.id !== id) }))
  },

  restoreNote: async (id) => {
    await notesApi.restoreNote(id)
    set(s => ({ notes: s.notes.filter(n => n.id !== id) }))
  },

  deleteNote: async (id) => {
    await notesApi.deleteNote(id)
    set(s => ({ notes: s.notes.filter(n => n.id !== id) }))
  },

  emptyTrash: async () => {
    await notesApi.emptyTrash()
    set({ notes: [] })
  },
}))
