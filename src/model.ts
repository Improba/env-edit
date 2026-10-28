export interface EnvVariable {
  id: string
  lineIndex?: number
  key: string
  value: string
  comment: string
  sensitive: boolean
}

export interface EnvFile {
  id: string
  name: string
  path: string
  variables: EnvVariable[]
  diagnostics?: string[]
}

export interface ParsedDocument {
  sessionId?: string
  variables: EnvVariable[]
  diagnostics: string[]
  unknownLines: number[]
  duplicateKeys: string[]
}

export type ProjectEnvironment = 'development' | 'preproduction' | 'production'

export interface Project {
  id: string
  name: string
  path: string
  environment: ProjectEnvironment
  expanded: boolean
  files: EnvFile[]
  ssh?: { alias: string; user?: string | null; port?: number | null; environment: ProjectEnvironment } | null
  connectionError?: string | null
  loadingFiles?: boolean
}

export interface ProjectSection {
  id: string
  name: string
  expanded: boolean
  projects: Project[]
  children: ProjectSection[]
}

export interface ProjectCatalog {
  sections: ProjectSection[]
  unsectionedProjects: Project[]
}

export interface TemplateKeys {
  keys: string[]
  comments: Record<string, string>
  unknownLines: number
  duplicateKeys: string[]
}

export function compareEnvKeys(currentKeys: string[], templateKeys: string[]) {
  const current = new Set(currentKeys)
  const template = new Set(templateKeys)
  return {
    missing: [...template].filter((key) => !current.has(key)).sort(),
    extra: [...current].filter((key) => !template.has(key)).sort(),
    common: [...current].filter((key) => template.has(key)).sort(),
  }
}

export function isBlankValue(value: string): boolean {
  return value.trim().length === 0
}

export function newlyBlankKeys(original: EnvVariable[], current: EnvVariable[], editedValueIds: ReadonlySet<string> = new Set()): string[] {
  const originals = new Map(original.map((variable) => [variable.id, variable]))
  return [...new Set(current
    .filter((variable) => isBlankValue(variable.value)
      && (!originals.has(variable.id) || !isBlankValue(originals.get(variable.id)!.value)
        || editedValueIds.has(variable.id)))
    .map((variable) => variable.key))]
}

const sensitiveNames = /PASSWORD|PASSWD|SECRET|TOKEN|API_KEY|PRIVATE_KEY|CREDENTIAL/i

export function isSensitiveKey(key: string): boolean {
  return sensitiveNames.test(key)
}

export function filterVariables(variables: EnvVariable[], query: string): EnvVariable[] {
  const term = query.trim().toLocaleLowerCase()
  if (!term) return variables
  return variables.filter((variable) =>
    variable.key.toLocaleLowerCase().includes(term)
    || variable.comment.toLocaleLowerCase().includes(term),
  )
}
