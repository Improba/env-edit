import { describe, expect, it } from 'vitest'
import { displayPath } from './displayPath'

describe('displayPath', () => {
  it('retire le préfixe étendu des chemins Windows locaux', () => {
    expect(displayPath('\\\\?\\C:\\Users\\Alice\\projet\\.env')).toBe('C:\\Users\\Alice\\projet\\.env')
  })

  it('affiche les chemins réseau étendus comme des chemins UNC usuels', () => {
    expect(displayPath('\\\\?\\UNC\\serveur\\partage\\.env')).toBe('\\\\serveur\\partage\\.env')
  })

  it('conserve les chemins Linux, Windows usuels et les autres espaces de noms', () => {
    expect(displayPath('/home/alice/projet/.env')).toBe('/home/alice/projet/.env')
    expect(displayPath('C:\\Users\\Alice\\projet\\.env')).toBe('C:\\Users\\Alice\\projet\\.env')
    expect(displayPath('\\\\?\\Volume{abc}\\.env')).toBe('\\\\?\\Volume{abc}\\.env')
  })
})
