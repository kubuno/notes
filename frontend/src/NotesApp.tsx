import { useEffect, useState, useRef, useCallback } from 'react'
import { useParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useConfirm } from '@kubuno/sdk'
import { ConfirmDialog } from '@ui'
import { FloatCheckbox } from '@ui'
import { MenuDropdown, type MenuDropdownPos } from '@ui'
import DOMPurify from 'dompurify'
import { useDraggable } from '@kubuno/sdk'
import {
  Pin, Archive, BookOpen,
  MoreHorizontal, Share2, Copy, Trash, Check,
  Palette, Bell, UserPlus, Image, Type, Undo2, Redo2,
  Pencil, MoreVertical, Clock, X, Tag,
} from 'lucide-react'
import { useNotesStore } from './store'
import { Note, Label, NOTE_COLORS, NoteColor } from './api'
import { format } from 'date-fns'
import { getDateLocale } from '@kubuno/sdk'

// ── CheckSquare icon ──────────────────────────────────────────────────────────
function CheckSquareIcon({ className = 'w-4 h-4' }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <rect x="3" y="3" width="18" height="18" rx="2" strokeWidth="2" />
      <polyline points="9,11 12,14 20,6" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  )
}

// ── IconBtn ───────────────────────────────────────────────────────────────────
function IconBtn({
  icon, title, dark, onClick, active,
}: {
  icon: React.ReactNode; title: string; dark?: boolean; onClick: () => void; active?: boolean
}) {
  return (
    <button
      title={title}
      className={`p-1.5 rounded-full transition-colors
        ${active
          ? 'bg-black/15 text-text-primary'
          : dark
          ? 'text-white/70 hover:bg-black/15 hover:text-white'
          : 'text-text-secondary hover:bg-black/8 hover:text-text-primary'}`}
      onClick={onClick}
    >
      {icon}
    </button>
  )
}

// ── ColorPicker ───────────────────────────────────────────────────────────────
function ColorPicker({
  current, onChange, onClose, direction = 'up',
}: {
  current: string
  onChange: (c: NoteColor) => void
  onClose: () => void
  direction?: 'up' | 'down'
}) {
  return (
    <div className={`absolute left-0 ${direction === 'up' ? 'bottom-full mb-1' : 'top-full mt-1'}
                     bg-white rounded-lg shadow-lg border border-border z-30 p-2`}>
      <div className="flex flex-wrap gap-1" style={{ width: 176 }}>
        {NOTE_COLORS.map(c => (
          <button
            key={c.id}
            title={c.label}
            className={`w-7 h-7 rounded-full border-2 hover:scale-110 transition-transform ${current === c.id ? 'border-primary' : 'border-transparent'}`}
            style={{ backgroundColor: c.hex, boxShadow: c.id === 'default' ? 'inset 0 0 0 1px #dadce0' : undefined }}
            onClick={() => { onChange(c.id as NoteColor); onClose() }}
          />
        ))}
      </div>
    </div>
  )
}

// ── LabelPicker ───────────────────────────────────────────────────────────────
function LabelPicker({ labels, onToggle }: {
  labels: Label[]
  onToggle: (labelId: string) => void
}) {
  const { t } = useTranslation('notes')
  return (
    <div className="absolute right-0 top-full mt-1 bg-white border border-border rounded-xl shadow-xl z-30 py-1
                    min-w-[180px] max-h-56 overflow-y-auto">
      {labels.length === 0 ? (
        <div className="px-3 py-2 text-sm text-text-tertiary">{t('no_labels')}</div>
      ) : (
        labels.map(label => (
          <button
            key={label.id}
            onClick={() => onToggle(label.id)}
            className="w-full flex items-center gap-2.5 px-3 py-2 text-sm text-text-primary
                       hover:bg-surface-1 text-left transition-colors"
          >
            <span
              className="w-3 h-3 rounded-full flex-shrink-0"
              style={{ backgroundColor: label.color || '#5f6368' }}
            />
            {label.name}
          </button>
        ))
      )}
    </div>
  )
}

// ── SelectionBar ──────────────────────────────────────────────────────────────
function SelectionBar({
  count, notes, selectedIds, labels, onClear, onBulkPin, onBulkColor, onBulkTrash, onBulkLabel,
}: {
  count: number
  notes: Note[]
  selectedIds: Set<string>
  labels: Label[]
  onClear: () => void
  onBulkPin: () => void
  onBulkColor: (c: NoteColor) => void
  onBulkTrash: () => void
  onBulkLabel: (labelId: string) => void
}) {
  const { t } = useTranslation('notes')
  const [colorOpen, setColorOpen] = useState(false)
  const [labelOpen, setLabelOpen] = useState(false)
  const colorRef = useRef<HTMLDivElement>(null)
  const labelRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (colorRef.current && !colorRef.current.contains(e.target as Node)) setColorOpen(false)
      if (labelRef.current && !labelRef.current.contains(e.target as Node)) setLabelOpen(false)
    }
    document.addEventListener('mousedown', handler)
    return () => document.removeEventListener('mousedown', handler)
  }, [])

  const selectedNotes = notes.filter(n => selectedIds.has(n.id))
  const allPinned = selectedNotes.length > 0 && selectedNotes.every(n => n.is_pinned)

  return (
    <div className="sticky top-0 z-20 -mx-6 -mt-6 mb-6 px-6 py-2.5
                    bg-white/95 backdrop-blur border-b border-border
                    flex items-center gap-1 no-print">
      <button
        onClick={onClear}
        title={t('tip_cancel_sel')}
        className="p-1.5 rounded-full text-text-secondary hover:bg-surface-2 transition-colors mr-1"
      >
        <X size={16} />
      </button>

      <span className="text-sm font-medium text-text-primary mr-auto">
        {t('notes_selected_count', { count })}
      </span>

      {/* Pin */}
      <IconBtn
        icon={<Pin size={16} style={{ fill: allPinned ? 'currentColor' : 'none' }} />}
        title={allPinned ? t('unpin') : t('pin')}
        onClick={onBulkPin}
      />

      {/* Color */}
      <div className="relative" ref={colorRef}>
        <IconBtn
          icon={<Palette size={16} />}
          title={t('tip_bg')}
          onClick={() => setColorOpen(v => !v)}
          active={colorOpen}
        />
        {colorOpen && (
          <ColorPicker
            current=""
            direction="down"
            onChange={c => { onBulkColor(c); setColorOpen(false) }}
            onClose={() => setColorOpen(false)}
          />
        )}
      </div>

      {/* Labels */}
      <div className="relative" ref={labelRef}>
        <IconBtn
          icon={<Tag size={16} />}
          title={t('tip_label')}
          onClick={() => setLabelOpen(v => !v)}
          active={labelOpen}
        />
        {labelOpen && (
          <LabelPicker labels={labels} onToggle={id => { onBulkLabel(id); setLabelOpen(false) }} />
        )}
      </div>

      {/* Trash */}
      <IconBtn
        icon={<Trash size={16} />}
        title={t('tip_delete')}
        onClick={onBulkTrash}
      />
    </div>
  )
}

// ── NoteCard ──────────────────────────────────────────────────────────────────
interface NoteCardProps {
  note: Note
  onClick: () => void
  onTrash: () => void
  onPin: () => void
  onArchive: () => void
  onColorChange: (color: NoteColor) => void
  selected?: boolean
  onToggle?: () => void
  isDragging?: boolean
  isDragOver?: boolean
  onDragStart?: (e: React.DragEvent) => void
  onDragOver?: (e: React.DragEvent) => void
  onDragLeave?: () => void
  onDrop?: (e: React.DragEvent) => void
  onDragEnd?: () => void
  selectionMode?: boolean
}

function NoteCard({
  note, onClick, onTrash, onPin, onArchive, onColorChange,
  selected, onToggle, isDragging, isDragOver,
  onDragStart, onDragOver, onDragLeave, onDrop, onDragEnd,
  selectionMode,
}: NoteCardProps) {
  const { t, i18n } = useTranslation('notes')
  const [menuPos, setMenuPos] = useState<MenuDropdownPos | null>(null)
  const [colorPickerOpen, setColorPickerOpen] = useState(false)
  const colorRef = useRef<HTMLDivElement>(null)
  const menuBtnRef = useRef<HTMLDivElement>(null)

  const colorEntry = NOTE_COLORS.find(c => c.id === note.color) ?? NOTE_COLORS[0]
  const isDark = note.color === 'charcoal'

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (colorRef.current && !colorRef.current.contains(e.target as Node)) setColorPickerOpen(false)
    }
    document.addEventListener('mousedown', handler)
    return () => document.removeEventListener('mousedown', handler)
  }, [])

  return (
    <div
      draggable={!selectionMode}
      onDragStart={onDragStart}
      onDragOver={onDragOver}
      onDragLeave={onDragLeave}
      onDrop={onDrop}
      onDragEnd={onDragEnd}
      className={`relative rounded-lg border cursor-pointer group transition-all
        ${selected
          ? 'ring-2 ring-primary border-primary shadow-md'
          : isDragOver
          ? 'ring-2 ring-primary/60 border-primary/40 shadow-md scale-[1.01]'
          : 'border-border hover:shadow-md'}
        ${isDragging ? 'opacity-40' : ''}`}
      style={{ backgroundColor: colorEntry.hex }}
      onClick={onClick}
    >
      {/* Floating checkbox */}
      <FloatCheckbox
        selected={!!selected}
        onToggle={() => onToggle?.()}
        className="absolute top-2 left-2 z-10"
      />

      {note.is_pinned && (
        <div className="absolute top-2 right-2">
          <Pin className={`w-3.5 h-3.5 ${isDark ? 'text-white/60' : 'text-text-secondary'}`} />
        </div>
      )}

      <div className="p-4 pt-4">
        {note.title && (
          <h3 className={`font-medium text-sm mb-2 pr-5 leading-snug ${isDark ? 'text-white' : 'text-text-primary'}`}>
            {note.title}
          </h3>
        )}
        {note.note_type === 'checklist' && note.checklist ? (
          <ul className="space-y-1">
            {note.checklist.slice(0, 5).map(item => (
              <li key={item.id} className={`flex items-center gap-2 text-xs ${isDark ? 'text-white/80' : 'text-text-secondary'}`}>
                {item.checked
                  ? <Check className="w-3 h-3 flex-shrink-0" />
                  : <div className="w-3 h-3 border border-current rounded-sm flex-shrink-0" />}
                <span className={item.checked ? 'line-through opacity-60' : ''}>{item.text}</span>
              </li>
            ))}
            {note.checklist.length > 5 && (
              <li className={`text-xs ${isDark ? 'text-white/50' : 'text-text-tertiary'}`}>
                {t('notes_more_items', { count: note.checklist.length - 5 })}
              </li>
            )}
          </ul>
        ) : note.content_html ? (
          <div
            className={`text-xs leading-relaxed line-clamp-6 ${isDark ? 'text-white/80' : 'text-text-secondary'}`}
            dangerouslySetInnerHTML={{ __html: DOMPurify.sanitize(note.content_html) }}
          />
        ) : null}
      </div>

      {/* Toolbar on hover */}
      <div
        className="flex items-center justify-between px-2 pb-2 opacity-0 group-hover:opacity-100 transition-opacity"
        onClick={e => e.stopPropagation()}
      >
        <div className="flex items-center gap-0.5">
          <IconBtn icon={<Clock className="w-3.5 h-3.5" />} title={t('tip_reminder')} dark={isDark} onClick={() => {}} />
          <IconBtn icon={<Pin className="w-3.5 h-3.5" />} title={note.is_pinned ? t('unpin') : t('pin')} dark={isDark} onClick={onPin} />
          <IconBtn icon={<Archive className="w-3.5 h-3.5" />} title={note.is_archived ? t('unarchive') : t('tip_archive')} dark={isDark} onClick={onArchive} />
          <div className="relative" ref={colorRef}>
            <IconBtn icon={<Palette className="w-3.5 h-3.5" />} title={t('tip_color')} dark={isDark} onClick={() => setColorPickerOpen(v => !v)} />
            {colorPickerOpen && (
              <ColorPicker current={note.color} onChange={onColorChange} onClose={() => setColorPickerOpen(false)} />
            )}
          </div>
        </div>
        <div className="relative" ref={menuBtnRef}>
          <IconBtn icon={<MoreVertical className="w-3.5 h-3.5" />} title={t('tip_more')} dark={isDark} onClick={() => {
            const el = menuBtnRef.current
            if (!el) return
            const r = el.getBoundingClientRect()
            setMenuPos(p => p ? null : { top: r.bottom + 4, left: r.right - 180 })
          }} />
          {menuPos && (
            <MenuDropdown
              pos={menuPos}
              onClose={() => setMenuPos(null)}
              items={[
                { type: 'action', label: t('duplicate'), icon: <Copy className="w-4 h-4" />, onClick: () => {} },
                { type: 'action', label: t('share'), icon: <Share2 className="w-4 h-4" />, onClick: () => {} },
                { type: 'action', label: t('common_delete'), icon: <Trash className="w-4 h-4" />, danger: true, onClick: () => onTrash() },
              ]}
            />
          )}
        </div>
      </div>

      <div className={`px-4 pb-2 text-[10px] ${isDark ? 'text-white/40' : 'text-text-tertiary'}`}>
        {format(new Date(note.updated_at), 'd MMM', { locale: getDateLocale(i18n.language) })}
      </div>
    </div>
  )
}

// ── NoteEditorToolbar ─────────────────────────────────────────────────────────
function NoteEditorToolbar({
  dark, color, onColorChange, onArchive, onClose,
}: {
  dark: boolean; color: string; onColorChange: (c: NoteColor) => void; onArchive: () => void; onClose: () => void
}) {
  const { t } = useTranslation('notes')
  const [colorOpen, setColorOpen] = useState(false)
  const [menuPos, setMenuPos] = useState<MenuDropdownPos | null>(null)
  const colorRef = useRef<HTMLDivElement>(null)
  const moreRef  = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (colorRef.current && !colorRef.current.contains(e.target as Node)) setColorOpen(false)
    }
    document.addEventListener('mousedown', handler)
    return () => document.removeEventListener('mousedown', handler)
  }, [])

  const btnClass = dark
    ? 'text-white/70 hover:bg-black/15 hover:text-white'
    : 'text-text-secondary hover:bg-black/8 hover:text-text-primary'

  return (
    <div
      className={`flex items-center px-2 py-1.5 border-t ${dark ? 'border-white/10' : 'border-black/6'}`}
      onClick={e => e.stopPropagation()}
    >
      <div className="flex items-center gap-0.5 flex-1">
        <IconBtn icon={<Type className="w-4 h-4" />}     title={t('tip_format')} dark={dark} onClick={() => {}} />
        <div className="relative" ref={colorRef}>
          <IconBtn icon={<Palette className="w-4 h-4" />} title={t('tip_note_color')} dark={dark} onClick={() => setColorOpen(v => !v)} active={colorOpen} />
          {colorOpen && (
            <ColorPicker current={color} onChange={c => { onColorChange(c); setColorOpen(false) }} onClose={() => setColorOpen(false)} />
          )}
        </div>
        <IconBtn icon={<Bell className="w-4 h-4" />}     title={t('tip_add_reminder')} dark={dark} onClick={() => {}} />
        <IconBtn icon={<UserPlus className="w-4 h-4" />} title={t('tip_collab')}       dark={dark} onClick={() => {}} />
        <IconBtn icon={<Image className="w-4 h-4" />}    title={t('tip_add_image')}    dark={dark} onClick={() => {}} />
        <IconBtn icon={<Archive className="w-4 h-4" />}  title={t('tip_archive')}      dark={dark} onClick={onArchive} />
        <div className="relative" ref={moreRef}>
          <IconBtn icon={<MoreHorizontal className="w-4 h-4" />} title={t('tip_more_options')} dark={dark} onClick={() => {
            const el = moreRef.current
            if (!el) return
            const r = el.getBoundingClientRect()
            setMenuPos(p => p ? null : { top: r.bottom + 4, left: r.right - 180 })
          }} active={!!menuPos} />
          {menuPos && (
            <MenuDropdown
              pos={menuPos}
              onClose={() => setMenuPos(null)}
              items={[
                { type: 'action', label: t('duplicate'), icon: <Copy className="w-4 h-4" />, onClick: () => {} },
                { type: 'action', label: t('share'), icon: <Share2 className="w-4 h-4" />, onClick: () => {} },
              ]}
            />
          )}
        </div>
        <div className={`w-px h-4 mx-1 ${dark ? 'bg-white/15' : 'bg-border'}`} />
        <IconBtn icon={<Undo2 className="w-4 h-4" />} title={t('tip_undo')}  dark={dark} onClick={() => document.execCommand('undo')} />
        <IconBtn icon={<Redo2 className="w-4 h-4" />} title={t('tip_redo')} dark={dark} onClick={() => document.execCommand('redo')} />
      </div>
      <button
        onClick={onClose}
        className={`px-4 py-1.5 text-sm font-medium rounded-full transition-colors ml-2
          ${dark ? 'text-white/80 hover:bg-black/15' : `${btnClass} hover:bg-surface-2`}`}
      >
        {t('common_close')}
      </button>
    </div>
  )
}

// ── NoteEditor ────────────────────────────────────────────────────────────────
type NoteUpdateData = Partial<{
  title: string | null; content: string; color: string; is_pinned: boolean
  is_archived: boolean; notebook_id: string | null
}>

interface NoteEditorProps {
  note: Note; onClose: () => void
  onUpdate: (id: string, data: NoteUpdateData) => Promise<void>
  onArchive: () => void
}

function NoteEditor({ note, onClose, onUpdate, onArchive }: NoteEditorProps) {
  const { t } = useTranslation('notes')
  const { dialogRef, startDrag } = useDraggable()
  const [title,   setTitle]   = useState(note.title ?? '')
  const [content, setContent] = useState(note.content ?? '')
  const [pinned,  setPinned]  = useState(note.is_pinned)
  const [color,   setColor]   = useState(note.color)
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null)

  useEffect(() => {
    if (dialogRef.current) {
      const rect = dialogRef.current.getBoundingClientRect()
      dialogRef.current.style.left = `${(window.innerWidth  - rect.width)  / 2}px`
      dialogRef.current.style.top  = `${(window.innerHeight - rect.height) / 2}px`
    }
  }, [])

  const colorEntry = NOTE_COLORS.find(c => c.id === color) ?? NOTE_COLORS[0]
  const isDark = color === 'charcoal'

  const pendingRef = useRef(false)
  const titleRef   = useRef(title);   titleRef.current = title
  const contentRef = useRef(content); contentRef.current = content

  const scheduleSave = useCallback((t: string, c: string) => {
    pendingRef.current = true
    if (saveTimer.current) clearTimeout(saveTimer.current)
    saveTimer.current = setTimeout(() => {
      pendingRef.current = false
      onUpdate(note.id, { title: t || null, content: c })
    }, 800)
  }, [note.id, onUpdate])

  // Vide la sauvegarde différée (avant fermeture/navigation/démontage) pour ne
  // pas perdre les saisies non encore envoyées.
  const flushSave = useCallback(() => {
    if (saveTimer.current) { clearTimeout(saveTimer.current); saveTimer.current = null }
    if (pendingRef.current) { pendingRef.current = false; onUpdate(note.id, { title: titleRef.current || null, content: contentRef.current }) }
  }, [note.id, onUpdate])

  useEffect(() => {
    window.addEventListener('pagehide', flushSave)
    window.addEventListener('beforeunload', flushSave)
    return () => {
      window.removeEventListener('pagehide', flushSave)
      window.removeEventListener('beforeunload', flushSave)
      flushSave()
    }
  }, [flushSave])

  const handleClose = useCallback(() => {
    if (saveTimer.current) { clearTimeout(saveTimer.current); saveTimer.current = null }
    pendingRef.current = false
    onUpdate(note.id, { title: title || null, content })
    onClose()
  }, [note.id, onUpdate, onClose, title, content])

  const handlePin = async () => {
    const next = !pinned; setPinned(next)
    await onUpdate(note.id, { is_pinned: next })
  }

  const handleColorChange = async (c: NoteColor) => {
    setColor(c); await onUpdate(note.id, { color: c })
  }

  return (
    <div className="fixed inset-0 z-50 bg-black/40" onClick={handleClose}>
      <div
        ref={dialogRef}
        className="fixed w-full max-w-lg rounded-xl shadow-2xl overflow-hidden"
        style={{ backgroundColor: colorEntry.hex, top: 0, left: 0 }}
        onClick={e => e.stopPropagation()}
      >
        <div className="flex items-start px-5 pt-5 pb-1 gap-2 cursor-move select-none" onMouseDown={startDrag}>
          <input
            className={`flex-1 text-base font-medium bg-transparent outline-none placeholder:text-text-tertiary
              ${isDark ? 'text-white' : 'text-text-primary'}`}
            placeholder={t('ph_title')}
            value={title}
            onChange={e => { setTitle(e.target.value); scheduleSave(e.target.value, content) }}
          />
          <button
            title={pinned ? t('unpin') : t('pin')}
            onClick={handlePin}
            className={`mt-0.5 p-1 rounded-full transition-colors flex-shrink-0
              ${pinned
                ? isDark ? 'text-white' : 'text-text-primary'
                : isDark ? 'text-white/50 hover:text-white' : 'text-text-tertiary hover:text-text-primary'}`}
          >
            <Pin className="w-4 h-4" style={{ fill: pinned ? 'currentColor' : 'none' }} />
          </button>
        </div>
        <textarea
          className={`w-full px-5 py-2 text-sm bg-transparent outline-none resize-none placeholder:text-text-tertiary
            ${isDark ? 'text-white/90' : 'text-text-primary'}`}
          placeholder={t('ph_create')}
          rows={8}
          value={content}
          onChange={e => { setContent(e.target.value); scheduleSave(title, e.target.value) }}
        />
        <NoteEditorToolbar
          dark={isDark} color={color}
          onColorChange={handleColorChange}
          onArchive={() => { onArchive(); handleClose() }}
          onClose={handleClose}
        />
      </div>
    </div>
  )
}

// ── QuickNoteBar ──────────────────────────────────────────────────────────────
function QuickNoteBar({ onCreate }: { onCreate: (data: { title?: string; content?: string; is_pinned?: boolean }) => void }) {
  const { t } = useTranslation('notes')
  const [open,    setOpen]    = useState(false)
  const [title,   setTitle]   = useState('')
  const [content, setContent] = useState('')
  const [pinned,  setPinned]  = useState(false)
  const [color,   setColor]   = useState<NoteColor>('default')
  const containerRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!open) return
    const handler = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) handleClose()
    }
    document.addEventListener('mousedown', handler)
    return () => document.removeEventListener('mousedown', handler)
  }, [open, title, content]) // eslint-disable-line react-hooks/exhaustive-deps

  const handleClose = () => {
    if (content.trim() || title.trim()) {
      onCreate({ title: title || undefined, content: content || undefined, is_pinned: pinned || undefined })
    }
    setOpen(false); setTitle(''); setContent(''); setPinned(false); setColor('default')
  }

  const colorEntry = NOTE_COLORS.find(c => c.id === color) ?? NOTE_COLORS[0]

  return (
    <div className="w-full max-w-full sm:max-w-xl mx-auto mb-8 no-print" ref={containerRef}>
      {!open ? (
        <div
          className="flex items-center gap-3 px-4 h-12 bg-white border border-border rounded-xl shadow-sm cursor-text hover:shadow transition-shadow"
          onClick={() => setOpen(true)}
        >
          <span className="flex-1 text-sm text-text-tertiary select-none">{t('ph_create')}</span>
          <div className="flex items-center gap-1" onClick={e => e.stopPropagation()}>
            <button title={t('new_list')}  className="p-1.5 rounded-full text-text-secondary hover:bg-surface-2 transition-colors" onClick={() => setOpen(true)}>
              <CheckSquareIcon className="w-5 h-5" />
            </button>
            <button title={t('new_drawing')}  className="p-1.5 rounded-full text-text-secondary hover:bg-surface-2 transition-colors" onClick={() => setOpen(true)}>
              <Pencil className="w-5 h-5" />
            </button>
            <button title={t('new_image')} className="p-1.5 rounded-full text-text-secondary hover:bg-surface-2 transition-colors" onClick={() => setOpen(true)}>
              <Image className="w-5 h-5" />
            </button>
          </div>
        </div>
      ) : (
        <div className="rounded-xl border border-border shadow-md overflow-hidden" style={{ backgroundColor: colorEntry.hex }}>
          <div className="flex items-start px-4 pt-4 pb-1 gap-2">
            <input
              autoFocus
              className="flex-1 text-sm font-medium text-text-primary bg-transparent outline-none placeholder:text-text-tertiary"
              placeholder={t('ph_title')}
              value={title}
              onChange={e => setTitle(e.target.value)}
            />
            <button
              title={pinned ? t('unpin') : t('pin')}
              onClick={() => setPinned(v => !v)}
              className={`mt-0.5 p-1 rounded-full transition-colors flex-shrink-0
                ${pinned ? 'text-text-primary' : 'text-text-tertiary hover:text-text-primary'}`}
            >
              <Pin className="w-4 h-4" style={{ fill: pinned ? 'currentColor' : 'none' }} />
            </button>
          </div>
          <textarea
            className="w-full px-4 py-2 text-sm text-text-primary bg-transparent outline-none resize-none placeholder:text-text-tertiary"
            placeholder={t('ph_create')}
            rows={3}
            value={content}
            onChange={e => setContent(e.target.value)}
          />
          <NoteEditorToolbar dark={false} color={color} onColorChange={setColor} onArchive={handleClose} onClose={handleClose} />
        </div>
      )}
    </div>
  )
}

// ── Main App ──────────────────────────────────────────────────────────────────

export default function NotesApp() {
  const { t } = useTranslation('notes')
  const {
    notes, view, activeNotebook, activeLabel, searchQuery, labels,
    fetchNotes, fetchNotebooks, fetchLabels,
    createNote, updateNote, trashNote, restoreNote, emptyTrash,
  } = useNotesStore()

  const { id: routeId } = useParams<{ id: string }>()
  const [activeNote,    setActiveNote]    = useState<Note | null>(null)
  const [selectedIds,   setSelectedIds]   = useState<Set<string>>(new Set())
  const [orderedNotes,  setOrderedNotes]  = useState<Note[]>([])
  const [dragId,        setDragId]        = useState<string | null>(null)
  const [overId,        setOverId]        = useState<string | null>(null)
  const { confirm, confirmState, handleConfirm, handleCancel } = useConfirm()

  useEffect(() => {
    fetchNotes(); fetchNotebooks(); fetchLabels()
  }, []) // eslint-disable-line react-hooks/exhaustive-deps

  // Ouverture par URL (/notes/:id) — ex. double-clic d'un .kbnot dans files.
  useEffect(() => {
    if (!routeId) return
    let alive = true
    import('./api').then(({ notesApi }) =>
      notesApi.getNote(routeId).then(n => { if (alive) setActiveNote(n) }).catch(() => {}))
    return () => { alive = false }
  }, [routeId])

  // Sync orderedNotes with store, preserving drag-and-drop order
  useEffect(() => {
    setOrderedNotes(prev => {
      const current = new Map(notes.map(n => [n.id, n]))
      const kept    = prev.filter(n => current.has(n.id)).map(n => current.get(n.id)!)
      const keptIds = new Set(kept.map(n => n.id))
      const newOnes = notes.filter(n => !keptIds.has(n.id))
      return [...newOnes, ...kept]
    })
  }, [notes])

  // ESC clears selection
  useEffect(() => {
    const handler = (e: KeyboardEvent) => { if (e.key === 'Escape') clearSelection() }
    document.addEventListener('keydown', handler)
    return () => document.removeEventListener('keydown', handler)
  }, [])

  // ── Selection ─────────────────────────────────────────────────────────────
  const toggleSelect = useCallback((id: string) => {
    setSelectedIds(prev => {
      const next = new Set(prev)
      if (next.has(id)) next.delete(id); else next.add(id)
      return next
    })
  }, [])

  const clearSelection = () => setSelectedIds(new Set())

  const handleUpdateNote = async (id: string, data: Parameters<typeof updateNote>[1]) => {
    await updateNote(id, data)
    if (activeNote?.id === id) setActiveNote(n => n ? { ...n, ...data } : n)
  }

  // ── Bulk actions ──────────────────────────────────────────────────────────
  const handleBulkPin = async () => {
    const ids = [...selectedIds]
    const allPinned = ids.every(id => notes.find(n => n.id === id)?.is_pinned)
    await Promise.all(ids.map(id => updateNote(id, { is_pinned: !allPinned })))
    clearSelection()
  }

  const handleBulkColor = async (color: NoteColor) => {
    await Promise.all([...selectedIds].map(id => updateNote(id, { color })))
  }

  const handleBulkTrash = async () => {
    const ok = await confirm({
      title:        t('notes_bulk_trash_title', { count: selectedIds.size }),
      message:      t('notes_bulk_trash_msg'),
      confirmLabel: t('common_delete'),
      cancelLabel:  t('common_cancel'),
      variant:      'danger',
    })
    if (!ok) return
    await Promise.all([...selectedIds].map(id => trashNote(id)))
    clearSelection()
  }

  const handleBulkLabel = async (labelId: string) => {
    const { notesApi } = await import('./api')
    await Promise.all([...selectedIds].map(id => notesApi.assignLabel(id, labelId).catch(() => {})))
  }

  // ── Single note actions ───────────────────────────────────────────────────
  const handleColorChange = async (noteId: string, color: NoteColor) => {
    await updateNote(noteId, { color })
  }

  const handlePin = async (note: Note) => {
    await updateNote(note.id, { is_pinned: !note.is_pinned })
  }

  const handleArchive = async (note: Note) => {
    await updateNote(note.id, { is_archived: !note.is_archived })
    await fetchNotes()
  }

  const handleTrash = async (note: Note) => { await trashNote(note.id) }

  // ── Drag & drop ───────────────────────────────────────────────────────────
  const handleDragStart = useCallback((e: React.DragEvent, id: string) => {
    e.dataTransfer.effectAllowed = 'move'
    setDragId(id)
  }, [])

  const handleDragOver = useCallback((e: React.DragEvent, id: string) => {
    e.preventDefault()
    e.dataTransfer.dropEffect = 'move'
    setOverId(id)
  }, [])

  const handleDragLeave = useCallback(() => {
    setOverId(null)
  }, [])

  const handleDrop = useCallback((e: React.DragEvent, targetId: string) => {
    e.preventDefault()
    setOverId(null)
    const fromId = dragId
    if (!fromId || fromId === targetId) { setDragId(null); return }
    setOrderedNotes(prev => {
      const next    = [...prev]
      const fromIdx = next.findIndex(n => n.id === fromId)
      const toIdx   = next.findIndex(n => n.id === targetId)
      if (fromIdx < 0 || toIdx < 0) return prev
      const [item] = next.splice(fromIdx, 1)
      next.splice(toIdx, 0, item)
      return next
    })
    setDragId(null)
  }, [dragId])

  const handleDragEnd = useCallback(() => {
    setDragId(null)
    setOverId(null)
  }, [])

  // ── Card click handler (selection mode vs open) ───────────────────────────
  const getCardClick = (note: Note) => {
    if (selectedIds.size > 0) return () => toggleSelect(note.id)
    return () => setActiveNote(note)
  }

  // ── Split notes ───────────────────────────────────────────────────────────
  const pinned   = orderedNotes.filter(n => n.is_pinned  && !n.is_trashed)
  const unpinned = orderedNotes.filter(n => !n.is_pinned && !n.is_trashed)

  const selectionMode  = selectedIds.size > 0
  const isDraggable    = view !== 'trashed' && view !== 'archived'

  const renderCard = (note: Note) => (
    <NoteCard
      key={note.id}
      note={note}
      selected={selectedIds.has(note.id)}
      onToggle={() => toggleSelect(note.id)}
      isDragging={dragId === note.id}
      isDragOver={overId === note.id}
      selectionMode={selectionMode || !isDraggable}
      onClick={getCardClick(note)}
      onTrash={() => handleTrash(note)}
      onPin={() => handlePin(note)}
      onArchive={() => handleArchive(note)}
      onColorChange={c => handleColorChange(note.id, c)}
      onDragStart={isDraggable ? e => handleDragStart(e, note.id) : undefined}
      onDragOver={isDraggable ? e => handleDragOver(e, note.id) : undefined}
      onDragLeave={isDraggable ? handleDragLeave : undefined}
      onDrop={isDraggable ? e => handleDrop(e, note.id) : undefined}
      onDragEnd={isDraggable ? handleDragEnd : undefined}
    />
  )

  return (
    <div className="h-full overflow-y-auto">
      <div className="px-6 py-6">

        {/* Selection bar */}
        {selectionMode && (
          <SelectionBar
            count={selectedIds.size}
            notes={notes}
            selectedIds={selectedIds}
            labels={labels}
            onClear={clearSelection}
            onBulkPin={handleBulkPin}
            onBulkColor={handleBulkColor}
            onBulkTrash={handleBulkTrash}
            onBulkLabel={handleBulkLabel}
          />
        )}

        {/* Trash: empty + confirm */}
        {view === 'trashed' && notes.length > 0 && (
          <div className="flex justify-end mb-4">
            <button
              onClick={async () => {
                const ok = await confirm({
                  title:        t('notes_empty_trash_title'),
                  message:      t('notes_empty_trash_msg'),
                  confirmLabel: t('notes_empty_trash_confirm'),
                  variant:      'danger',
                })
                if (ok) emptyTrash()
              }}
              className="text-sm text-danger hover:underline"
            >
              {t('notes_empty_trash_btn')}
            </button>
          </div>
        )}

        {/* Quick create */}
        {view !== 'trashed' && view !== 'archived' && !searchQuery && !activeNotebook && !activeLabel && (
          <QuickNoteBar onCreate={data => createNote(data)} />
        )}

        {/* Empty state */}
        {notes.length === 0 && (
          <div className="flex flex-col items-center justify-center py-24 text-text-tertiary">
            <BookOpen className="w-16 h-16 mb-4 opacity-30" />
            <p className="text-sm">
              {view === 'trashed'  ? t('empty_trash')
                : view === 'archived' ? t('empty_archived')
                : searchQuery         ? t('empty_search')
                :                      t('empty_none')}
            </p>
          </div>
        )}

        {/* Pinned */}
        {pinned.length > 0 && view === 'all' && !searchQuery && (
          <>
            <p className="text-xs font-semibold text-text-tertiary uppercase tracking-wider mb-3">{t('pinned')}</p>
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-3 items-start mb-8">
              {pinned.map(renderCard)}
            </div>
            {unpinned.length > 0 && (
              <p className="text-xs font-semibold text-text-tertiary uppercase tracking-wider mb-3">{t('others')}</p>
            )}
          </>
        )}

        {/* Main grid */}
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-3 items-start">
          {(view === 'all' && !searchQuery && pinned.length > 0 ? unpinned : orderedNotes)
            .filter(n => view === 'trashed' ? true : !n.is_trashed)
            .map(note =>
              view === 'trashed' ? (
                <div key={note.id} className="relative">
                  {renderCard(note)}
                  <div className="absolute top-2 left-8 z-10" onClick={e => e.stopPropagation()}>
                    <button
                      onClick={() => restoreNote(note.id)}
                      className="text-xs bg-white border border-border rounded px-2 py-0.5 text-text-secondary hover:text-text-primary shadow-sm"
                    >
                      {t('restore')}
                    </button>
                  </div>
                </div>
              ) : renderCard(note)
            )
          }
        </div>
      </div>

      {/* Confirm dialogs */}
      {confirmState && (
        <ConfirmDialog {...confirmState} onConfirm={handleConfirm} onCancel={handleCancel} />
      )}

      {/* Note editor modal */}
      {activeNote && (
        <NoteEditor
          note={activeNote}
          onClose={() => setActiveNote(null)}
          onUpdate={handleUpdateNote}
          onArchive={() => handleArchive(activeNote)}
        />
      )}
    </div>
  )
}
