/** Format Windows extended-length paths for display only; never use as a file identity. */
export function displayPath(path: string): string {
  const extendedPrefix = '\\\\?\\'
  if (!path.startsWith(extendedPrefix)) return path

  const rest = path.slice(extendedPrefix.length)
  if (rest.startsWith('UNC\\')) return `\\\\${rest.slice(4)}`
  if (/^[A-Za-z]:\\/.test(rest)) return rest
  return path
}
