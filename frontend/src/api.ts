import { api as apiClient } from '@kubuno/sdk'

export interface Note {
  id: string
  owner_id: string
  title: string | null
  content: string | null
  content_html: string | null
  note_type: 'text' | 'checklist' | 'drawing' | 'voice'
  color: string
  is_pinned: boolean
  is_archived: boolean
  is_trashed: boolean
  trashed_at: string | null
  notebook_id: string | null
  mentions: string[]
  mentioned_by: string[]
  word_count: number
  checklist: ChecklistItem[] | null
  created_at: string
  updated_at: string
}

export interface ChecklistItem {
  id: string
  text: string
  checked: boolean
}

export interface Notebook {
  id: string
  owner_id: string
  name: string
  description: string | null
  color: string
  icon: string | null
  parent_id: string | null
  note_count: number
  created_at: string
  updated_at: string
}

export interface Label {
  id: string
  owner_id: string
  name: string
  color: string
  created_at: string
}

export interface Reminder {
  id: string
  note_id: string
  owner_id: string
  fire_at: string
  method: string
  recurrence: string | null
  sent_at: string | null
  created_at: string
}

export interface NoteShare {
  id: string
  note_id: string
  created_by: string
  token: string
  permission: string
  expires_at: string | null
  view_count: number
  is_active: boolean
  last_accessed_at: string | null
  created_at: string
}

export interface GraphData {
  nodes: GraphNode[]
  edges: GraphEdge[]
}

export interface GraphNode {
  id: string
  label: string
}

export interface GraphEdge {
  source: string
  target: string
}

export const notesApi = {
  // Notes
  listNotes: (params?: {
    notebook_id?: string
    label_id?: string
    archived?: boolean
    trashed?: boolean
    pinned?: boolean
    q?: string
  }) => apiClient.get<{ notes: Note[] }>('/notes/notes', { params }).then(r => r.data),

  getNote: (id: string) =>
    apiClient.get<{ note: Note }>(`/notes/notes/${id}`).then(r => r.data.note),

  openByFile: (fileId: string) =>
    apiClient.post<{ note: Note }>('/notes/notes/open-by-file', { file_id: fileId }).then(r => r.data.note),

  createNote: (data: {
    title?: string
    content?: string
    note_type?: string
    color?: string
    notebook_id?: string
    checklist?: ChecklistItem[]
  }) => apiClient.post<{ note: Note }>('/notes/notes', data).then(r => r.data.note),

  updateNote: (id: string, data: Partial<{
    title: string | null
    content: string
    color: string
    is_pinned: boolean
    is_archived: boolean
    notebook_id: string | null
    checklist: ChecklistItem[]
  }>) => apiClient.patch<{ note: Note }>(`/notes/notes/${id}`, data).then(r => r.data.note),

  trashNote: (id: string) =>
    apiClient.post(`/notes/notes/${id}/trash`),

  restoreNote: (id: string) =>
    apiClient.post(`/notes/notes/${id}/restore`),

  deleteNote: (id: string) =>
    apiClient.delete(`/notes/notes/${id}/delete`),

  emptyTrash: () =>
    apiClient.delete<{ deleted: number }>('/notes/notes/trash').then(r => r.data),

  duplicateNote: (id: string) =>
    apiClient.post<{ note: Note }>(`/notes/notes/${id}/duplicate`).then(r => r.data.note),

  getBacklinks: (id: string) =>
    apiClient.get<{ backlinks: Note[] }>(`/notes/notes/${id}/backlinks`).then(r => r.data.backlinks),

  // Notebooks
  listNotebooks: () =>
    apiClient.get<{ notebooks: Notebook[] }>('/notes/notebooks').then(r => r.data.notebooks),

  createNotebook: (data: { name: string; description?: string; color?: string; parent_id?: string }) =>
    apiClient.post<{ notebook: Notebook }>('/notes/notebooks', data).then(r => r.data.notebook),

  updateNotebook: (id: string, data: Partial<{ name: string; description: string; color: string }>) =>
    apiClient.patch<{ notebook: Notebook }>(`/notes/notebooks/${id}`, data).then(r => r.data.notebook),

  deleteNotebook: (id: string) =>
    apiClient.delete(`/notes/notebooks/${id}`),

  // Labels
  listLabels: () =>
    apiClient.get<{ labels: Label[] }>('/notes/labels').then(r => r.data.labels),

  createLabel: (data: { name: string; color?: string }) =>
    apiClient.post<{ label: Label }>('/notes/labels', data).then(r => r.data.label),

  updateLabel: (id: string, data: Partial<{ name: string; color: string }>) =>
    apiClient.patch<{ label: Label }>(`/notes/labels/${id}`, data).then(r => r.data.label),

  deleteLabel: (id: string) =>
    apiClient.delete(`/notes/labels/${id}`),

  assignLabel: (noteId: string, labelId: string) =>
    apiClient.post(`/notes/notes/${noteId}/labels/${labelId}`),

  removeLabel: (noteId: string, labelId: string) =>
    apiClient.delete(`/notes/notes/${noteId}/labels/${labelId}`),

  // Reminders
  listReminders: (noteId: string) =>
    apiClient.get<{ reminders: Reminder[] }>(`/notes/notes/${noteId}/reminders`).then(r => r.data.reminders),

  createReminder: (noteId: string, data: { fire_at: string; method?: string; recurrence?: string }) =>
    apiClient.post<{ reminder: Reminder }>(`/notes/notes/${noteId}/reminders`, data).then(r => r.data.reminder),

  deleteReminder: (noteId: string, reminderId: string) =>
    apiClient.delete(`/notes/notes/${noteId}/reminders/${reminderId}`),

  // Shares
  listShares: (noteId: string) =>
    apiClient.get<{ shares: NoteShare[] }>(`/notes/notes/${noteId}/shares`).then(r => r.data.shares),

  createShare: (noteId: string, data: { expires_in_days?: number }) =>
    apiClient.post<{ share: NoteShare }>(`/notes/notes/${noteId}/shares`, data).then(r => r.data.share),

  deleteShare: (noteId: string, shareId: string) =>
    apiClient.delete(`/notes/notes/${noteId}/shares/${shareId}`),

  // Search
  search: (q: string, limit?: number) =>
    apiClient.get<{ notes: Note[] }>('/notes/search', { params: { q, limit } }).then(r => r.data.notes),

  // Knowledge graph
  getGraph: () =>
    apiClient.get<GraphData>('/notes/graph').then(r => r.data),
}

export const NOTE_COLORS = [
  { id: 'default',  hex: '#ffffff', label: 'Défaut' },
  { id: 'red',      hex: '#f28b82', label: 'Tomate' },
  { id: 'orange',   hex: '#fbbc04', label: 'Orange' },
  { id: 'yellow',   hex: '#fff475', label: 'Banane' },
  { id: 'green',    hex: '#ccff90', label: 'Sauge' },
  { id: 'teal',     hex: '#a8f0cb', label: 'Menthe' },
  { id: 'blue',     hex: '#cbf0f8', label: 'Glacier' },
  { id: 'darkblue', hex: '#aecbfa', label: 'Bleuet' },
  { id: 'purple',   hex: '#d7aefb', label: 'Lavande' },
  { id: 'pink',     hex: '#fdcfe8', label: 'Pivoine' },
  { id: 'brown',    hex: '#e6c9a8', label: 'Sable' },
  { id: 'gray',     hex: '#e8eaed', label: 'Graphite' },
  { id: 'charcoal', hex: '#232427', label: 'Charbon' },
] as const

export type NoteColor = typeof NOTE_COLORS[number]['id']
