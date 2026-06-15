import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  BookOpen, Pin, Archive, Trash2, Tag, ChevronDown,
} from 'lucide-react'
import { useNotesStore } from './store'
import { SidebarNavItem } from '@kubuno/sdk'

export default function NotesSidebarBody({ collapsed = false }: { collapsed?: boolean }) {
  const { t } = useTranslation('notes')
  const {
    view, setView,
    notebooks, labels,
    activeNotebook, activeLabel,
    setActiveNotebook, setActiveLabel,
  } = useNotesStore()

  const [notebooksOpen, setNotebooksOpen] = useState(true)
  const [labelsOpen,    setLabelsOpen]    = useState(true)

  const isAllActive = view === 'all' && !activeNotebook && !activeLabel

  return (
    <nav className={`flex-1 overflow-y-auto py-1 space-y-0.5 ${collapsed ? "px-2" : "px-3"}`}>
      <SidebarNavItem collapsed={collapsed}
        label={t('notes_nav_all')}
        icon={<BookOpen className="w-4 h-4 flex-shrink-0" />}
        active={isAllActive}
        onClick={() => { setView('all'); setActiveNotebook(null); setActiveLabel(null) }}
      />
      <SidebarNavItem collapsed={collapsed}
        label={t('notes_nav_pinned')}
        icon={<Pin className="w-4 h-4 flex-shrink-0" />}
        active={view === 'pinned'}
        onClick={() => setView('pinned')}
      />
      <SidebarNavItem collapsed={collapsed}
        label={t('notes_nav_archived')}
        icon={<Archive className="w-4 h-4 flex-shrink-0" />}
        active={view === 'archived'}
        onClick={() => setView('archived')}
      />
      <SidebarNavItem collapsed={collapsed}
        label={t('notes_nav_trash')}
        icon={<Trash2 className="w-4 h-4 flex-shrink-0" />}
        active={view === 'trashed'}
        onClick={() => setView('trashed')}
      />

      {!collapsed && notebooks.length > 0 && (
        <div className="pt-2">
          <button
            onClick={() => setNotebooksOpen((v) => !v)}
            className="flex items-center gap-2 w-full px-3 py-1 text-[10px] font-bold
                       text-text-tertiary uppercase tracking-widest hover:text-text-secondary"
          >
            <ChevronDown
              className={`w-3 h-3 transition-transform ${notebooksOpen ? '' : '-rotate-90'}`}
            />
            {t('notes_nav_notebooks')}
          </button>
          {notebooksOpen &&
            notebooks.map((nb) => (
              <SidebarNavItem collapsed={collapsed}
                key={nb.id}
                label={nb.name}
                icon={<BookOpen className="w-4 h-4 flex-shrink-0" />}
                active={activeNotebook === nb.id}
                onClick={() => setActiveNotebook(nb.id)}
              />
            ))}
        </div>
      )}

      {!collapsed && labels.length > 0 && (
        <div className="pt-2">
          <button
            onClick={() => setLabelsOpen((v) => !v)}
            className="flex items-center gap-2 w-full px-3 py-1 text-[10px] font-bold
                       text-text-tertiary uppercase tracking-widest hover:text-text-secondary"
          >
            <ChevronDown
              className={`w-3 h-3 transition-transform ${labelsOpen ? '' : '-rotate-90'}`}
            />
            {t('notes_nav_labels')}
          </button>
          {labelsOpen &&
            labels.map((lb) => (
              <button
                key={lb.id}
                onClick={() => setActiveLabel(lb.id)}
                className={`
                  flex items-center gap-3 w-full px-3 py-2 rounded-full text-sm transition-colors
                  ${activeLabel === lb.id
                    ? 'bg-primary-light text-primary font-medium'
                    : 'text-text-secondary hover:bg-surface-2'}
                `}
              >
                <Tag className="w-4 h-4 flex-shrink-0" style={{ color: lb.color }} />
                <span className="truncate">{lb.name}</span>
              </button>
            ))}
        </div>
      )}
    </nav>
  )
}
