import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link as RouterLink, useLocation } from 'react-router-dom'
import {
  BookOpen, Pin, Archive, Trash2, Tag, ChevronDown,
} from 'lucide-react'
import { useNotesStore } from './store'
import { SidebarNavItem } from '@kubuno/sdk'
import { hashTo, fromHash } from './hashRoute'

// Plain views reachable through a sidebar hash link (no id attached).
const HASH_VIEWS = ['all', 'pinned', 'archived', 'trashed'] as const
type HashView = (typeof HASH_VIEWS)[number]

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
  const { hash } = useLocation()

  const isAllActive = view === 'all' && !activeNotebook && !activeLabel

  // The hash is the source of truth for the selected view: a direct link, a
  // sidebar click and the Back button all end up here.
  useEffect(() => {
    const r = fromHash(hash)
    if (!r) return
    if (r.kind === 'notebook') { if (r.id) setActiveNotebook(r.id); return }
    if (r.kind === 'tag')      { if (r.id) setActiveLabel(r.id); return }
    if ((HASH_VIEWS as readonly string[]).includes(r.kind)) setView(r.kind as HashView)
  }, [hash, setView, setActiveNotebook, setActiveLabel])

  // Section header ("Notebooks", "Labels"): collapsing a section is a pure
  // in-page action, so it is an anchor-button — href="#", Space wired by hand,
  // Enter handled natively by the anchor. Never a <button> in the sidebar.
  const sectionCls =
    `flex items-center gap-2 w-full px-3 py-1 text-[10px] font-bold no-underline cursor-pointer
     text-text-tertiary uppercase tracking-widest hover:text-text-secondary
     outline-none focus-visible:ring-2 focus-visible:ring-primary`

  return (
    <nav className={`flex-1 overflow-y-auto py-1 space-y-0.5 px-2`}>
      <SidebarNavItem collapsed={collapsed}
        label={t('notes_nav_all')}
        icon={<BookOpen className="w-4 h-4 flex-shrink-0" />}
        active={isAllActive}
        to={hashTo('all')}
      />
      <SidebarNavItem collapsed={collapsed}
        label={t('notes_nav_pinned')}
        icon={<Pin className="w-4 h-4 flex-shrink-0" />}
        active={view === 'pinned'}
        to={hashTo('pinned')}
      />
      <SidebarNavItem collapsed={collapsed}
        label={t('notes_nav_archived')}
        icon={<Archive className="w-4 h-4 flex-shrink-0" />}
        active={view === 'archived'}
        to={hashTo('archived')}
      />
      <SidebarNavItem collapsed={collapsed}
        label={t('notes_nav_trash')}
        icon={<Trash2 className="w-4 h-4 flex-shrink-0" />}
        active={view === 'trashed'}
        to={hashTo('trashed')}
      />

      {!collapsed && notebooks.length > 0 && (
        <div className="pt-2">
          <a
            href="#"
            role="button"
            aria-expanded={notebooksOpen}
            onClick={e => { e.preventDefault(); setNotebooksOpen(v => !v) }}
            onKeyDown={e => { if (e.key === ' ') { e.preventDefault(); setNotebooksOpen(v => !v) } }}
            className={sectionCls}
          >
            <ChevronDown
              className={`w-3 h-3 transition-transform ${notebooksOpen ? '' : '-rotate-90'}`}
            />
            {t('notes_nav_notebooks')}
          </a>
          {notebooksOpen &&
            notebooks.map((nb) => (
              <SidebarNavItem collapsed={collapsed}
                key={nb.id}
                label={nb.name}
                icon={<BookOpen className="w-4 h-4 flex-shrink-0" />}
                active={activeNotebook === nb.id}
                to={hashTo('notebook', nb.id)}
              />
            ))}
        </div>
      )}

      {!collapsed && labels.length > 0 && (
        <div className="pt-2">
          <a
            href="#"
            role="button"
            aria-expanded={labelsOpen}
            onClick={e => { e.preventDefault(); setLabelsOpen(v => !v) }}
            onKeyDown={e => { if (e.key === ' ') { e.preventDefault(); setLabelsOpen(v => !v) } }}
            className={sectionCls}
          >
            <ChevronDown
              className={`w-3 h-3 transition-transform ${labelsOpen ? '' : '-rotate-90'}`}
            />
            {t('notes_nav_labels')}
          </a>
          {labelsOpen &&
            labels.map((lb) => (
              <RouterLink
                key={lb.id}
                to={hashTo('tag', lb.id)}
                className={`
                  flex items-center gap-3 w-full px-3 py-2 rounded-full text-sm transition-colors
                  no-underline cursor-pointer outline-none focus-visible:ring-2 focus-visible:ring-primary
                  ${activeLabel === lb.id
                    ? 'bg-primary-light text-primary font-medium'
                    : 'text-text-secondary hover:bg-surface-2'}
                `}
              >
                <Tag className="w-4 h-4 flex-shrink-0" style={{ color: lb.color }} />
                <span className="truncate">{lb.name}</span>
              </RouterLink>
            ))}
        </div>
      )}
    </nav>
  )
}
