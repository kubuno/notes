interface NotesLogoProps {
  size?:      number
  className?: string
  title?:     string
}

/** Notes logo (designer artwork, raster). Served by the host from
 *  `/notes-logo.png`; rendered as a square image so it weighs the same as its
 *  neighbours in the waffle menu. */
export function NotesLogo({ size = 24, className, title = 'Notes' }: NotesLogoProps) {
  return (
    <img
      src="/notes-logo.png"
      width={size}
      height={size}
      alt={title}
      className={className}
      style={{ display: 'block', objectFit: 'contain' }}
    />
  )
}

export default NotesLogo
