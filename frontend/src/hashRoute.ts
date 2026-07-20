// Notes views (all, pinned, a notebook, a label…) are client-side state rather
// than routes of their own. They still deserve a real, shareable link, so the
// left sidebar encodes them in the URL HASH: /notes/#pinned,
// /notes/#notebook/<id>, /notes/#tag/<id>. Every sidebar entry is then an <a>
// carrying a genuine href, and direct links plus the browser Back button keep
// working because the view is read back from the location hash.

/** Route the module is mounted on. */
export const NOTES_BASE = '/notes'

/** `to=` value for a view link: /notes/#<kind>[/<id>]. */
export function hashTo(kind: string, id?: string | null): string {
  return `${NOTES_BASE}/#${kind}${id ? `/${encodeURIComponent(id)}` : ''}`
}

/**
 * Parse a location hash produced by `hashTo`.
 * Returns null when the hash holds nothing this module owns.
 */
export function fromHash(hash: string): { kind: string; id: string | null } | null {
  const m = /^#([a-z]+)(?:\/([^/#?]+))?$/.exec(hash)
  if (!m) return null
  return { kind: m[1], id: m[2] ? decodeURIComponent(m[2]) : null }
}
