import { describe, expect, it } from 'vitest'
import { compareEnvKeys, filterVariables, isBlankValue, isSensitiveKey, newlyBlankKeys, type EnvVariable } from './model'

const variables: EnvVariable[] = [
  { id: '1', key: 'APP_URL', value: 'x', comment: 'Adresse publique', sensitive: false },
  { id: '2', key: 'DB_PASSWORD', value: 'fixture', comment: 'Connexion', sensitive: true },
]

describe('modèle frontend', () => {
  it('détecte les noms sensibles sans inspecter les valeurs', () => {
    expect(isSensitiveKey('db_password')).toBe(true)
    expect(isSensitiveKey('PUBLIC_URL')).toBe(false)
  })

  it('filtre par clé ou commentaire', () => {
    expect(filterVariables(variables, 'app')).toEqual([variables[0]])
    expect(filterVariables(variables, 'connexion')).toEqual([variables[1]])
  })

  it('compare les ensembles de clés sans utiliser les valeurs ni compter les doublons plusieurs fois', () => {
    expect(compareEnvKeys(['A', 'A', 'ONLY_CURRENT'], ['A', 'MISSING', 'MISSING'])).toEqual({
      missing: ['MISSING'], extra: ['ONLY_CURRENT'], common: ['A'],
    })
  })

  it('signale seulement les valeurs nouvellement vides, y compris les espaces', () => {
    const original: EnvVariable[] = [
      { id: 'old-empty', key: 'OPTIONAL', value: '', comment: '', sensitive: false },
      { id: 'old-spaces', key: 'SPACES', value: '  ', comment: '', sensitive: false },
      { id: 'changed', key: 'DB_HOST', value: 'localhost', comment: '', sensitive: false },
      { id: 'unchanged', key: 'ENABLED', value: 'false', comment: '', sensitive: false },
    ]
    const current: EnvVariable[] = [
      ...original.slice(0, 2),
      { ...original[2]!, value: ' \t' },
      original[3]!,
      { id: 'new', key: 'API_TOKEN', value: '', comment: '', sensitive: true },
      { id: 'new-filled', key: 'PORT', value: '0', comment: '', sensitive: false },
    ]
    expect(newlyBlankKeys(original, current)).toEqual(['DB_HOST', 'API_TOKEN'])
    expect(newlyBlankKeys(original, current, new Set(['old-empty']))).toEqual(['OPTIONAL', 'DB_HOST', 'API_TOKEN'])
    expect(newlyBlankKeys(original, original, new Set(['old-empty']))).toEqual(['OPTIONAL'])
    expect(newlyBlankKeys(original, [{ ...original[0]!, value: 'renseignée' }], new Set(['old-empty']))).toEqual([])
    expect(isBlankValue('')).toBe(true)
    expect(isBlankValue(' \t')).toBe(true)
    expect(isBlankValue('false')).toBe(false)
    expect(isBlankValue('0')).toBe(false)
  })
})
