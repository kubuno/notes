/**
 * Point d'entrée du bundle MODULE notes, chargé à l'exécution. Buildé séparément
 * via `vite.module.config.ts` ; specifiers partagés résolus au runtime par
 * l'import map du host. Le host importe ce fichier puis appelle `register()` ;
 * `sdkVersion` permet de rejeter une incompatibilité de contrat.
 */
import { lazy } from 'react'
import {
  RouteRegistry,
  ModuleSettingsRegistry,
  WidgetRegistry,
  WaffleAppRegistry,
  FileTypeRegistry,
  FaviconRegistry,
  useSidebarStore,
  useToolbarStore,
  useSearchStore,
  useRightPanelStore,
  SDK_VERSION,
} from '@kubuno/sdk'
import { FileText } from 'lucide-react'
import './index.css'
import './i18n'
import { useNotesStore } from './store'
import NotesLogo from './NotesLogo'
import NotesNewActions from './NotesNewActions'
import NotesSidebarBody from './NotesSidebarBody'
import NotesToolbar from './NotesToolbar'
import NotesMiniPanel from './NotesMiniPanel'
import NotesFilterPanel from './NotesFilterPanel'
import NotesRecentWidget from './NotesRecentWidget'

export const sdkVersion = SDK_VERSION

export function register() {
  FaviconRegistry.register('notes', '/notes-logo.svg')

  WaffleAppRegistry.register('notes', 'Notes', [
    { id: 'notes', label: 'Notes', Icon: NotesLogo, path: '/notes' },
  ])

  // The header gear button opens the per-user Notes settings while in /notes.
  ModuleSettingsRegistry.register('notes')

  // Type de fichier Kubuno produit par Notes (.kbnot) — filtrage + icône + ouverture.
  FileTypeRegistry.register({
    moduleId: 'notes', label: 'Notes', icon: 'StickyNote',
    mimeTypes: ['application/vnd.kubuno.note+json'],
    extensions: ['kbnot'],
    open: (f, nav) => { import('./api').then(({ notesApi }) => notesApi.openByFile(f.id).then(note => nav(`/notes/${note.id}`)).catch(() => {})) },
  })

  WidgetRegistry.register({ id: 'notes-recent', moduleId: 'notes', Component: NotesRecentWidget, size: 'small', order: 40 })

  useSidebarStore.getState().register({
    moduleId:    'notes',
    routePrefix: '/notes',
    NewActions:  NotesNewActions,
    SidebarBody: NotesSidebarBody,
    collapsedBody: true,
  })

  useToolbarStore.getState().register({
    moduleId:         'notes',
    routePrefix:      '/notes',
    ToolbarComponent: NotesToolbar,
    noPadding:        true,
  })

  useToolbarStore.getState().register({
    moduleId:    'notes-settings',
    routePrefix: '/notes/settings',
  })

  useSearchStore.getState().register({
    moduleId:    'notes',
    routePrefix: '/notes',
    placeholder: 'Rechercher dans les notes…',
    placeholderKey: 'notes:notes_search_ph',
    onSearch:    (q) => useNotesStore.getState().setSearchQuery(q),
    FilterPanel: NotesFilterPanel,
  })

  useRightPanelStore.getState().registerEntry({
    moduleId:       'notes',
    icon:           FileText,
    label:          'Notes',
    panelComponent: NotesMiniPanel,
  })

  // Routes
  const NotesApp          = lazy(() => import('./NotesApp'))
  const NotesSettingsPage = lazy(() => import('./NotesSettingsPage'))

  RouteRegistry.register('notes',          NotesApp)
  RouteRegistry.register('notes/:id',      NotesApp)
  RouteRegistry.register('notes/settings', NotesSettingsPage)
}
