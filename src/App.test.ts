// @vitest-environment happy-dom

import { config, flushPromises, mount } from '@vue/test-utils'
import { Dialog, Notify, Quasar } from 'quasar'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import App from './App.vue'

const { invoke, writeText, openDirectory } = vi.hoisted(() => ({ invoke: vi.fn(), writeText: vi.fn(), openDirectory: vi.fn() }))

vi.mock('@tauri-apps/api/core', () => ({ invoke }))
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: openDirectory }))
vi.mock('@tauri-apps/plugin-clipboard-manager', () => ({ writeText }))
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({
    close: vi.fn(),
    onCloseRequested: vi.fn(async () => vi.fn()),
  }),
}))

class ResizeObserverStub {
  observe() {}
  unobserve() {}
  disconnect() {}
}

describe('parcours principal portable', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.stubGlobal('ResizeObserver', ResizeObserverStub)
    invoke.mockImplementation(async (command: string) => {
      if (command === 'get_catalog') {
        return {
          sections: [{
            id: 'travail',
            name: 'Travail',
            expanded: true,
            projects: [],
            children: [{
              id: 'local',
              name: 'Local',
              expanded: true,
              children: [],
              projects: [{
                id: '/fixture/project',
                name: 'projet-fixture',
                path: '/fixture/project',
                environment: 'development',
                expanded: true,
                files: [{ id: '/fixture/project/.env', name: '.env', path: '/fixture/project/.env', variables: [] }],
              }],
            }],
          }],
          unsectionedProjects: [],
        }
      }
      if (command === 'open_env_file') {
        return {
          sessionId: 'fixture-session',
          variables: [{ id: 'line-0', lineIndex: 0, key: 'APP_URL', value: 'https://example.test', comment: '', sensitive: false }],
          diagnostics: [], unknownLines: [], duplicateKeys: [],
        }
      }
      if (command === 'save_env_file') {
        return {
          variables: [{ id: 'line-0', lineIndex: 0, key: 'APP_URL', value: 'changed', comment: '', sensitive: false }],
          diagnostics: [], unknownLines: [], duplicateKeys: [],
        }
      }
      throw new Error(`Commande inattendue : ${command}`)
    })
    writeText.mockResolvedValue(undefined)
    config.global.plugins = [[Quasar, { plugins: { Dialog, Notify } }]]
  })

  it('charge un projet puis affiche les variables du fichier sélectionné', async () => {
    const wrapper = mount(App, { attachTo: document.body })
    await flushPromises()
    expect(wrapper.text()).not.toContain('projet-fixture')
    await wrapper.findAll('.section-row')[0]!.trigger('click')
    await flushPromises()
    await wrapper.findAll('.section-row')[1]!.trigger('click')
    await flushPromises()
    const project = wrapper.find('.project-row')
    expect(project.exists()).toBe(true)
    await project.trigger('click')
    await flushPromises()
    const file = wrapper.findAll('.file-row').find((item) => item.text().includes('.env'))
    expect(file).toBeDefined()
    await file!.trigger('click')
    await flushPromises()
    expect(invoke).toHaveBeenCalledWith('open_env_file', { path: '/fixture/project/.env' })
    expect(wrapper.findAll('input').some((input) => input.element.value === 'APP_URL')).toBe(true)
    expect(wrapper.text()).toContain('1 variable(s) affichée(s)')
    expect(wrapper.text()).toContain('v0.4.4')
    wrapper.unmount()
  })

  it('ouvre les paramètres du projet local et transmet les modifications', async () => {
    const original = invoke.getMockImplementation()!
    invoke.mockImplementation(async (command: string, args?: unknown) => {
      if (command === 'update_project') return { sections: [], unsectionedProjects: [] }
      return original(command, args)
    })
    const wrapper = mount(App, { attachTo: document.body })
    await flushPromises()
    await wrapper.findAll('.section-row')[0]!.trigger('click')
    await wrapper.findAll('.section-row')[1]!.trigger('click')
    await wrapper.find('.project-row .tree-actions').trigger('click')
    await flushPromises()
    const edit = [...document.body.querySelectorAll<HTMLElement>('.project-menu .q-item')]
      .find((item) => item.textContent?.includes('Modifier le projet'))
    expect(edit).toBeDefined()
    edit!.click()
    await flushPromises()
    const name = document.body.querySelector<HTMLInputElement>('.project-local-content input')!
    name.value = 'Nouveau nom'
    name.dispatchEvent(new Event('input', { bubbles: true }))
    await flushPromises()
    const save = [...document.body.querySelectorAll<HTMLButtonElement>('.project-dialog button')]
      .find((button) => button.textContent?.trim() === 'Enregistrer le projet')
    save!.click()
    await flushPromises()
    expect(invoke).toHaveBeenCalledWith('update_project', {
      projectId: '/fixture/project', name: 'Nouveau nom', path: '/fixture/project', environment: 'development',
      alias: null, user: null, port: null,
    })
    wrapper.unmount()
  })

  it('ouvre un projet local de production en lecture seule', async () => {
    const original = invoke.getMockImplementation()!
    invoke.mockImplementation(async (command: string, args?: unknown) => {
      const result = await original(command, args)
      if (command === 'get_catalog') result.sections[0].children[0].projects[0].environment = 'production'
      return result
    })
    const wrapper = mount(App, { attachTo: document.body })
    await flushPromises()
    await wrapper.findAll('.section-row')[0]!.trigger('click')
    await wrapper.findAll('.section-row')[1]!.trigger('click')
    await wrapper.find('.project-row').trigger('click')
    await wrapper.find('.file-row').trigger('click')
    await flushPromises()
    expect(wrapper.text()).toContain('LOCAL · PRODUCTION')
    expect(wrapper.find('input[aria-label="Valeur"]').attributes('readonly')).toBeDefined()
    expect(wrapper.find('[aria-label="Supprimer la variable"]').attributes('disabled')).toBeDefined()
    wrapper.unmount()
  })

  it('compare les clés et ajoute une clé manquante sans enregistrer automatiquement', async () => {
    const original = invoke.getMockImplementation()!
    invoke.mockImplementation(async (command: string, args?: unknown) => {
      if (command === 'get_env_template_keys') return { keys: ['APP_URL', 'DB_HOST'], comments: { DB_HOST: 'Serveur de base de données' }, unknownLines: 0, duplicateKeys: [] }
      const result = await original(command, args)
      if (command === 'get_catalog') result.sections[0].children[0].projects[0].files.push({
        id: '/fixture/project/.env.example', name: '.env.example', path: '/fixture/project/.env.example', variables: [],
      })
      return result
    })
    const wrapper = mount(App, { attachTo: document.body })
    await flushPromises()
    await wrapper.findAll('.section-row')[0]!.trigger('click')
    await wrapper.findAll('.section-row')[1]!.trigger('click')
    await wrapper.find('.project-row').trigger('click')
    await wrapper.findAll('.file-row')[0]!.trigger('click')
    await flushPromises()
    expect(invoke).toHaveBeenCalledWith('get_env_template_keys', {
      projectId: '/fixture/project', templateName: '.env.example',
    })
    expect(wrapper.text()).toContain('1 clé manquante · 0 supplémentaire')
    await wrapper.find('[aria-label="Voir la comparaison"]').trigger('click')
    expect(wrapper.text()).toContain('DB_HOST')
    await wrapper.find('[aria-label="Ajouter DB_HOST"]').trigger('click')
    await flushPromises()
    expect(wrapper.text()).toContain('Toutes les clés sont présentes')
    expect(wrapper.findAll('input[aria-label="Nom de variable"]').some((input) => (input.element as HTMLInputElement).value === 'DB_HOST')).toBe(true)
    expect(wrapper.find('[aria-label="Serveur de base de données"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('Valeur vide')
    expect(invoke).not.toHaveBeenCalledWith('save_env_file', expect.anything())
    const save = wrapper.findAll('button').find((button) => button.text().includes('Enregistrer'))
    await save!.trigger('click')
    await flushPromises()
    expect(document.body.textContent).toContain('Valeurs vides à enregistrer')
    expect(document.body.textContent).toContain('DB_HOST')
    expect(invoke).not.toHaveBeenCalledWith('save_env_file', expect.anything())
    const confirm = [...document.body.querySelectorAll<HTMLButtonElement>('.q-dialog button')]
      .find((button) => button.textContent?.includes('Enregistrer quand même'))
    confirm!.click()
    await flushPromises()
    expect(invoke).toHaveBeenCalledWith('save_env_file', expect.objectContaining({
      variables: expect.arrayContaining([expect.objectContaining({ key: 'DB_HOST', value: '', comment: 'Serveur de base de données' })]),
    }))
    wrapper.unmount()
  })

  it('ne redemande pas confirmation pour une valeur déjà vide avant édition', async () => {
    const original = invoke.getMockImplementation()!
    invoke.mockImplementation(async (command: string, args?: unknown) => {
      if (command === 'open_env_file') return {
        sessionId: 'fixture-session', diagnostics: [], unknownLines: [], duplicateKeys: [],
        variables: [
          { id: 'line-0', lineIndex: 0, key: 'OPTIONAL', value: '', comment: '', sensitive: false },
          { id: 'line-1', lineIndex: 1, key: 'APP_URL', value: 'old', comment: '', sensitive: false },
        ],
      }
      return original(command, args)
    })
    const wrapper = mount(App, { attachTo: document.body })
    await flushPromises()
    await wrapper.findAll('.section-row')[0]!.trigger('click')
    await wrapper.findAll('.section-row')[1]!.trigger('click')
    await wrapper.find('.project-row').trigger('click')
    await wrapper.find('.file-row').trigger('click')
    await flushPromises()
    expect(wrapper.text()).toContain('Valeur vide')
    const values = wrapper.findAll('input[aria-label="Valeur"]')
    await values[1]!.setValue('new')
    await wrapper.findAll('button').find((button) => button.text().includes('Enregistrer'))!.trigger('click')
    await flushPromises()
    expect(invoke).toHaveBeenCalledWith('save_env_file', expect.objectContaining({
      variables: expect.arrayContaining([expect.objectContaining({ key: 'OPTIONAL', value: '' })]),
    }))
    expect(document.body.textContent).not.toContain('Valeurs vides à enregistrer')
    wrapper.unmount()
  })

  it('avertit si une valeur initialement vide a été remplie puis vidée', async () => {
    const original = invoke.getMockImplementation()!
    invoke.mockImplementation(async (command: string, args?: unknown) => {
      if (command === 'open_env_file') return {
        sessionId: 'fixture-session', diagnostics: [], unknownLines: [], duplicateKeys: [],
        variables: [{ id: 'line-0', lineIndex: 0, key: 'OPTIONAL', value: '', comment: '', sensitive: false }],
      }
      return original(command, args)
    })
    const wrapper = mount(App, { attachTo: document.body })
    await flushPromises()
    await wrapper.findAll('.section-row')[0]!.trigger('click')
    await wrapper.findAll('.section-row')[1]!.trigger('click')
    await wrapper.find('.project-row').trigger('click')
    await wrapper.find('.file-row').trigger('click')
    await flushPromises()
    const input = wrapper.find('input[aria-label="Valeur"]').element as HTMLInputElement
    input.value = 'temporaire'
    input.dispatchEvent(new Event('input', { bubbles: true }))
    input.value = ''
    input.dispatchEvent(new Event('input', { bubbles: true }))
    await flushPromises()
    expect(wrapper.text()).toContain('Valeur vide')
    await wrapper.findAll('button').find((button) => button.text().includes('Enregistrer'))!.trigger('click')
    await flushPromises()
    expect(document.body.textContent).toContain('Valeurs vides à enregistrer')
    expect(document.body.textContent).toContain('OPTIONAL')
    expect(invoke).not.toHaveBeenCalledWith('save_env_file', expect.anything())
    const confirm = [...document.body.querySelectorAll<HTMLButtonElement>('.q-dialog button')]
      .find((button) => button.textContent?.includes('Enregistrer quand même'))
    confirm!.click()
    await flushPromises()
    expect(invoke).toHaveBeenCalledWith('save_env_file', expect.objectContaining({
      variables: expect.arrayContaining([expect.objectContaining({ key: 'OPTIONAL', value: '' })]),
    }))
    wrapper.unmount()
  })

  it('conserve un rail et permet de replier puis déplier le panneau', async () => {
    const wrapper = mount(App, { attachTo: document.body })
    await flushPromises()
    const toggle = wrapper.find('[aria-label="Replier le panneau des projets"]')
    expect(toggle.exists()).toBe(true)
    await toggle.trigger('click')
    expect(wrapper.find('[aria-label="Déplier le panneau des projets"]').exists()).toBe(true)
    expect(wrapper.find('[aria-label="Ouvrir les projets"]').exists()).toBe(true)
    expect(wrapper.find('.rail-count text').text()).toBe('1')
    expect(wrapper.find('.file-row').exists()).toBe(false)
    await wrapper.find('[aria-label="Ouvrir les projets"]').trigger('click')
    expect(wrapper.find('.section-row').exists()).toBe(true)
    expect(wrapper.find('.file-row').exists()).toBe(false)
    wrapper.unmount()
  })

  it('affiche les projets regroupés dans leur section', async () => {
    const wrapper = mount(App, { attachTo: document.body })
    await flushPromises()
    expect(wrapper.text()).toContain('Travail')
    expect(wrapper.text()).not.toContain('Local')
    await wrapper.find('.section-row').trigger('click')
    await flushPromises()
    expect(wrapper.text()).toContain('Local')
    expect(wrapper.text()).not.toContain('projet-fixture')
    wrapper.unmount()
  })

  it('copie une valeur sans modifier le document', async () => {
    const wrapper = mount(App, { attachTo: document.body })
    await flushPromises()
    await wrapper.findAll('.section-row')[0]!.trigger('click')
    await wrapper.findAll('.section-row')[1]!.trigger('click')
    await wrapper.find('.project-row').trigger('click')
    await wrapper.find('.file-row').trigger('click')
    await flushPromises()
    await wrapper.find('[aria-label="Copier la valeur"]').trigger('click')
    await flushPromises()
    expect(writeText).toHaveBeenCalledWith('https://example.test')
    const save = wrapper.findAll('button').find((button) => button.text().includes('Enregistrer'))
    expect(save?.attributes()).toHaveProperty('disabled')
    wrapper.unmount()
  })

  it('ouvre un fichier de production SSH en lecture seule', async () => {
    invoke.mockImplementation(async (command: string) => {
      if (command === 'get_catalog') return { sections: [], unsectionedProjects: [{
        id: 'ssh://prod/srv/app', name: 'production', path: 'ssh://prod/srv/app', expanded: false,
        ssh: { alias: 'prod', environment: 'production' },
        files: [],
      }] }
      if (command === 'discover_project_files') return [
        { id: 'ssh://prod/srv/app/.env', name: '.env', path: 'ssh://prod/srv/app/.env', variables: [] },
      ]
      if (command === 'open_env_file') return { sessionId: 'ssh-session',
        variables: [{ id: 'line-0', lineIndex: 0, key: 'API_TOKEN', value: 'fixture-only', comment: '', sensitive: true }],
        diagnostics: [], unknownLines: [], duplicateKeys: [] }
      throw new Error(`Commande inattendue : ${command}`)
    })
    const wrapper = mount(App, { attachTo: document.body })
    await flushPromises()
    expect(invoke).not.toHaveBeenCalledWith('discover_project_files', { projectId: 'ssh://prod/srv/app' })
    await wrapper.find('.section-row').trigger('click')
    await wrapper.find('.project-row').trigger('click')
    await flushPromises()
    expect(invoke).toHaveBeenCalledWith('discover_project_files', { projectId: 'ssh://prod/srv/app' })
    await wrapper.find('.file-row').trigger('click')
    await flushPromises()
    expect(wrapper.text()).toContain('PRODUCTION')
    expect(wrapper.text()).toContain('Lecture seule')
    expect(wrapper.find('[aria-label="Valeur"]').attributes()).toHaveProperty('readonly')
    expect(wrapper.find('[aria-label="Supprimer la variable"]').attributes()).toHaveProperty('disabled')
    wrapper.unmount()
  })

  it('garde le projet visible si la découverte SSH échoue', async () => {
    invoke.mockImplementation(async (command: string) => {
      if (command === 'get_catalog') return { sections: [], unsectionedProjects: [{
        id: 'ssh://offline/srv/app', name: 'serveur indisponible', path: 'ssh://offline/srv/app',
        expanded: false, ssh: { alias: 'offline', environment: 'production' }, files: [],
      }] }
      if (command === 'discover_project_files') throw new Error('Serveur SSH inaccessible')
      throw new Error(`Commande inattendue : ${command}`)
    })
    const wrapper = mount(App, { attachTo: document.body })
    await flushPromises()
    await wrapper.find('.section-row').trigger('click')
    await wrapper.find('.project-row').trigger('click')
    await flushPromises()
    expect(wrapper.text()).toContain('serveur indisponible')
    expect(wrapper.text()).toContain('Serveur SSH inaccessible')
    wrapper.unmount()
  })

  it('transmet utilisateur et port du formulaire SSH', async () => {
    invoke.mockImplementation(async (command: string) => {
      if (command === 'get_catalog' || command === 'add_ssh_project') return { sections: [], unsectionedProjects: [] }
      throw new Error(`Commande inattendue : ${command}`)
    })
    const wrapper = mount(App, { attachTo: document.body })
    await flushPromises()
    await wrapper.find('[aria-label="Ajouter un projet"]').trigger('click')
    await flushPromises()
    const tabs = [...document.body.querySelectorAll<HTMLElement>('.project-mode-tabs .q-tab')]
    expect(tabs).toHaveLength(2)
    tabs[1]!.click()
    await flushPromises()
    const inputs = [...document.body.querySelectorAll<HTMLInputElement>('.project-ssh-content input')]
    expect(inputs.length).toBeGreaterThanOrEqual(4)
    for (const [index, value] of ['Test', 'server.test', 'deploy', '/srv/app'].entries()) {
      inputs[index]!.value = value
      inputs[index]!.dispatchEvent(new Event('input', { bubbles: true }))
    }
    document.body.querySelector<HTMLElement>('.project-ssh-options .q-item')!.click()
    await flushPromises()
    const portInput = [...document.body.querySelectorAll<HTMLInputElement>('.project-ssh-options input')][0]!
    portInput.value = '2222'
    portInput.dispatchEvent(new Event('input', { bubbles: true }))
    await flushPromises()
    const add = [...document.body.querySelectorAll<HTMLButtonElement>('.project-dialog button')]
      .find((button) => button.textContent?.trim() === 'Ajouter le projet')
    expect(add).toBeDefined()
    add!.click()
    await flushPromises()
    expect(invoke).toHaveBeenCalledWith('add_ssh_project', {
      name: 'Test', alias: 'server.test', user: 'deploy', port: 2222,
      directory: '/srv/app', environment: 'preproduction', sectionId: null,
    })
    wrapper.unmount()
  })

  it('ajoute un dossier local depuis la même modale', async () => {
    openDirectory.mockResolvedValue('/fixture/new-project')
    invoke.mockImplementation(async (command: string) => {
      if (command === 'get_catalog' || command === 'add_project') return { sections: [], unsectionedProjects: [] }
      throw new Error(`Commande inattendue : ${command}`)
    })
    const wrapper = mount(App, { attachTo: document.body })
    await flushPromises()
    await wrapper.find('[aria-label="Ajouter un projet"]').trigger('click')
    await flushPromises()
    expect(document.body.querySelectorAll('.project-mode-tabs .q-tab')).toHaveLength(2)
    const choose = [...document.body.querySelectorAll<HTMLButtonElement>('.project-dialog button')]
      .find((button) => button.textContent?.includes('Choisir un dossier'))
    choose!.click()
    await flushPromises()
    expect(openDirectory).toHaveBeenCalledWith({ directory: true, multiple: false, title: 'Choisir le dossier du projet' })
    expect(document.body.querySelector('.project-local-path')?.textContent).toBe('/fixture/new-project')
    const add = [...document.body.querySelectorAll<HTMLButtonElement>('.project-dialog button')]
      .find((button) => button.textContent?.trim() === 'Ajouter le projet')
    add!.click()
    await flushPromises()
    expect(invoke).toHaveBeenCalledWith('add_project', { path: '/fixture/new-project', name: 'new-project', environment: 'development', sectionId: null })
    wrapper.unmount()
  })

  it('ajoute le projet dans la section d’origine', async () => {
    openDirectory.mockResolvedValue('/fixture/section-project')
    invoke.mockImplementation(async (command: string) => {
      if (command === 'get_catalog' || command === 'add_project') return {
        sections: [{ id: 'travail', name: 'Travail', expanded: false, projects: [], children: [] }],
        unsectionedProjects: [],
      }
      throw new Error(`Commande inattendue : ${command}`)
    })
    const wrapper = mount(App, { attachTo: document.body })
    await flushPromises()
    await wrapper.find('.section-row .tree-actions').trigger('click')
    await flushPromises()
    const menuItem = [...document.body.querySelectorAll<HTMLElement>('.project-menu .q-item')]
      .find((item) => item.textContent?.includes('Ajouter un projet'))
    expect(menuItem).toBeDefined()
    menuItem!.click()
    await flushPromises()
    const choose = [...document.body.querySelectorAll<HTMLButtonElement>('.project-dialog button')]
      .find((button) => button.textContent?.includes('Choisir un dossier'))
    choose!.click()
    await flushPromises()
    const add = [...document.body.querySelectorAll<HTMLButtonElement>('.project-dialog button')]
      .find((button) => button.textContent?.trim() === 'Ajouter le projet')
    add!.click()
    await flushPromises()
    expect(invoke).toHaveBeenCalledWith('add_project', { path: '/fixture/section-project', name: 'section-project', environment: 'development', sectionId: 'travail' })
    wrapper.unmount()
  })
})
