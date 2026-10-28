<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { open } from '@tauri-apps/plugin-dialog'
import { writeText } from '@tauri-apps/plugin-clipboard-manager'
import { useQuasar } from 'quasar'
import { compareEnvKeys, filterVariables, isBlankValue, isSensitiveKey, newlyBlankKeys, type EnvFile, type EnvVariable, type ParsedDocument, type Project, type ProjectCatalog, type ProjectSection, type ProjectEnvironment, type TemplateKeys } from './model'
import packageInfo from '../package.json'
import ProjectTree from './ProjectTree.vue'
import SectionTree from './SectionTree.vue'

const $q = useQuasar()
const catalog = ref<ProjectCatalog>({ sections: [], unsectionedProjects: [] })
const unsectionedExpanded = ref(true)
const selectedFile = ref<EnvFile | null>(null)
const query = ref('')
const dirty = ref(false)
const revealed = ref(new Set<string>())
const loadingProjects = ref(false)
const loadingFile = ref(false)
const saving = ref(false)
const sessionId = ref<string | null>(null)
const projectDialog = ref(false)
const editingProject = ref<Project | null>(null)
const projectMode = ref<'local' | 'ssh'>('local')
const projectSectionId = ref<string | null>(null)
const localDirectory = ref('')
const localName = ref('')
const localEnvironment = ref<ProjectEnvironment>('development')
const projectAdding = ref(false)
const sshName = ref('')
const sshAlias = ref('')
const sshUser = ref('')
const sshPort = ref('')
const sshDirectory = ref('')
const sshEnvironment = ref<'development' | 'preproduction' | 'production'>('preproduction')
const environmentOptions = [{ label: 'Développement', value: 'development' }, { label: 'Préproduction', value: 'preproduction' }, { label: 'Production', value: 'production' }]
const appVersion = packageInfo.version
const editingUnlocked = ref(false)
const originalVariables = ref<EnvVariable[]>([])
const editedValueIds = ref(new Set<string>())
const copiedVariableId = ref<string | null>(null)
const selectedTemplateName = ref<string | null>(null)
const templateKeys = ref<TemplateKeys | null>(null)
const templateError = ref('')
const templateLoading = ref(false)
const comparisonOpen = ref(false)
let templateRequestId = 0
const drawerMini = ref($q.screen.width < 900)
let closingApproved = false
let unlistenClose: (() => void) | null = null
let copyFeedbackTimer: ReturnType<typeof setTimeout> | null = null

const displayedVariables = computed(() =>
  selectedFile.value ? filterVariables(selectedFile.value.variables, query.value) : [],
)
const allProjects = computed(() => {
  const collect = (sections: ProjectSection[]): Project[] => sections.flatMap((section) => [...section.projects, ...collect(section.children)])
  return [...catalog.value.unsectionedProjects, ...collect(catalog.value.sections)]
})
const selectedProject = computed(() => allProjects.value.find((project) =>
  project.files.some((file) => file.id === selectedFile.value?.id)
  || (project.ssh && selectedFile.value?.path.startsWith(`${project.id}/`))))
const selectedSsh = computed(() => selectedProject.value?.ssh)
const selectedEnvironment = computed(() => selectedProject.value?.environment ?? selectedSsh.value?.environment ?? 'development')
const readOnly = computed(() => !!selectedProject.value && selectedEnvironment.value !== 'development' && !editingUnlocked.value)
const availableTemplates = computed(() => selectedProject.value?.files
  .filter((file) => file.name === '.env.example' || file.name === '.env.exemple')
  .map((file) => file.name)
  .sort((left, right) => left === '.env.example' ? -1 : right === '.env.example' ? 1 : left.localeCompare(right)) ?? [])
const keyComparison = computed(() => compareEnvKeys(selectedFile.value?.variables.map((item) => item.key) ?? [], templateKeys.value?.keys ?? []))
const comparisonSummary = computed(() => {
  const missing = keyComparison.value.missing.length
  const extra = keyComparison.value.extra.length
  if (!missing && !extra) return 'Toutes les clés sont présentes'
  return `${missing} clé${missing > 1 ? 's' : ''} manquante${missing > 1 ? 's' : ''} · ${extra} supplémentaire${extra > 1 ? 's' : ''}`
})
const canCompare = computed(() => !!selectedFile.value && selectedFile.value.name !== '.env.example'
  && selectedFile.value.name !== '.env.exemple' && availableTemplates.value.length > 0)
const canAddProject = computed(() => projectMode.value === 'local'
  ? !!localDirectory.value && !!localName.value.trim()
  : !!sshName.value.trim() && !!sshAlias.value.trim() && sshDirectory.value.trim().startsWith('/')
    && (!sshPort.value || /^\d+$/.test(sshPort.value) && Number(sshPort.value) >= 1 && Number(sshPort.value) <= 65535))
function countSectionProjects(section: ProjectSection): number {
  return section.projects.length
    + section.children.reduce((count, child) => count + countSectionProjects(child), 0)
}
const projectCount = computed(() => catalog.value.unsectionedProjects.length
  + catalog.value.sections.reduce((count, section) => count + countSectionProjects(section), 0))
const allSections = computed(() => {
  const result: ProjectSection[] = []
  const visit = (sections: ProjectSection[]) => {
    for (const section of sections) {
      result.push(section)
      visit(section.children)
    }
  }
  visit(catalog.value.sections)
  return result
})
const railCountWidth = computed(() => Math.max(20, 12 + String(projectCount.value).length * 7))

function confirmDiscard(message: string): Promise<boolean> {
  if (!dirty.value) return Promise.resolve(true)
  return new Promise<boolean>((resolve) => {
    $q.dialog({
      title: 'Modifications non enregistrées',
      message,
      cancel: { label: 'Annuler', flat: true },
      ok: { label: 'Abandonner', color: 'negative' },
      persistent: true,
    }).onOk(() => resolve(true)).onCancel(() => resolve(false))
  })
}

async function selectFile(file: EnvFile) {
  if (dirty.value && selectedFile.value?.id !== file.id) {
    const discard = await confirmDiscard('Abandonner les modifications et changer de fichier ?')
    if (!discard) return
  }
  loadingFile.value = true
  try {
    const document = await invoke<ParsedDocument>('open_env_file', { path: file.path })
    file.variables = document.variables
    file.diagnostics = document.diagnostics
    selectedFile.value = file
    sessionId.value = document.sessionId ?? null
    originalVariables.value = document.variables.map((variable) => ({ ...variable }))
    editedValueIds.value = new Set()
    editingUnlocked.value = false
    query.value = ''
    dirty.value = false
    ++templateRequestId
    selectedTemplateName.value = null
    templateKeys.value = null
    templateError.value = ''
    templateLoading.value = false
    comparisonOpen.value = false
    if (file.name !== '.env.example' && file.name !== '.env.exemple') {
      const project = selectedProject.value
      const templates = project?.files.filter((item) => item.name === '.env.example' || item.name === '.env.exemple') ?? []
      const preferred = templates.find((item) => item.name === '.env.example') ?? templates.find((item) => item.name === '.env.exemple')
      if (project && preferred) void loadTemplateKeys(project.id, preferred.name)
    }
  } catch (error) {
    $q.notify({ type: 'negative', message: String(error) })
  } finally {
    loadingFile.value = false
  }
}

async function loadTemplateKeys(projectId: string, name: string) {
  const requestId = ++templateRequestId
  selectedTemplateName.value = name
  templateKeys.value = null
  templateError.value = ''
  templateLoading.value = true
  try {
    const result = await invoke<TemplateKeys>('get_env_template_keys', { projectId, templateName: name })
    if (requestId === templateRequestId) templateKeys.value = result
  } catch (error) {
    if (requestId === templateRequestId) templateError.value = String(error)
  } finally {
    if (requestId === templateRequestId) templateLoading.value = false
  }
}

function changeTemplate(name: string | null) {
  if (name && selectedProject.value) void loadTemplateKeys(selectedProject.value.id, name)
}

function addMissingKeys(keys: string[]) {
  if (!selectedFile.value || readOnly.value) return
  const existing = new Set(selectedFile.value.variables.map((item) => item.key))
  for (const key of keys) {
    if (existing.has(key)) continue
    selectedFile.value.variables.push({ id: crypto.randomUUID(), key, value: '', comment: templateKeys.value?.comments?.[key] ?? '', sensitive: isSensitiveKey(key) })
    existing.add(key)
    dirty.value = true
  }
}

async function saveCurrent() {
  if (!selectedFile.value || !sessionId.value || !dirty.value || readOnly.value || saving.value) return
  saving.value = true
  try {
    const blankKeys = newlyBlankKeys(originalVariables.value, selectedFile.value.variables, editedValueIds.value)
    const blankWarning = blankKeys.length
      ? `\nAttention : ${blankKeys.length} clé${blankKeys.length > 1 ? 's' : ''} ajoutée${blankKeys.length > 1 ? 's' : ''} ou vidée${blankKeys.length > 1 ? 's' : ''} sans valeur (ou avec seulement des espaces) : ${blankKeys.join(', ')}.`
      : ''
    if (selectedProject.value && selectedEnvironment.value !== 'development') {
      const before = new Map(originalVariables.value.map((variable) => [variable.id, variable]))
      const after = new Map(selectedFile.value.variables.map((variable) => [variable.id, variable]))
      const changed = selectedFile.value.variables.filter((variable) => {
        const old = before.get(variable.id)
        return !old || old.key !== variable.key || old.value !== variable.value || old.comment !== variable.comment
      }).map((variable) => variable.key)
      const removed = originalVariables.value.filter((variable) => !after.has(variable.id)).map((variable) => variable.key)
      const confirmed = await new Promise<boolean>((resolve) => {
        $q.dialog({ title: selectedSsh.value ? 'Confirmer la sauvegarde distante' : 'Confirmer la sauvegarde',
          message: `${selectedProject.value?.name} · ${selectedFile.value?.path}\nClés modifiées ou ajoutées : ${changed.join(', ') || 'aucune'}\nClés supprimées : ${removed.join(', ') || 'aucune'}${blankWarning}${selectedSsh.value ? '\nUne sauvegarde de l’ancienne version sera créée sur le serveur.' : ''}`,
          cancel: { label: 'Annuler', flat: true }, ok: { label: blankKeys.length ? 'Enregistrer quand même' : 'Enregistrer', color: 'warning' }, persistent: true,
        }).onOk(() => resolve(true)).onCancel(() => resolve(false))
      })
      if (!confirmed) return
    } else if (blankKeys.length) {
      const confirmed = await new Promise<boolean>((resolve) => {
        $q.dialog({ title: 'Valeurs vides à enregistrer',
          message: `Ces clés ont été ajoutées ou vidées et n'ont pas de valeur (ou seulement des espaces) : ${blankKeys.join(', ')}. Cela peut être volontaire pour une variable optionnelle.`,
          cancel: { label: 'Revenir à l’édition', flat: true }, ok: { label: 'Enregistrer quand même', color: 'warning' }, persistent: true,
        }).onOk(() => resolve(true)).onCancel(() => resolve(false))
      })
      if (!confirmed) return
    }
    const document = await invoke<ParsedDocument>('save_env_file', {
      sessionId: sessionId.value,
      variables: selectedFile.value.variables.map(({ lineIndex, key, value, comment }) => ({
        lineIndex,
        key,
        value,
        comment,
      })),
    })
    selectedFile.value.variables = document.variables
    selectedFile.value.diagnostics = document.diagnostics
    originalVariables.value = document.variables.map((variable) => ({ ...variable }))
    editedValueIds.value = new Set()
    dirty.value = false
    editingUnlocked.value = false
    $q.notify({ type: 'positive', message: 'Fichier enregistré' })
  } catch (error) {
    const message = String(error)
    $q.notify({ type: 'negative', message, timeout: 7000 })
    if (message.includes('Conflit') && selectedFile.value) {
      const file = selectedFile.value
      $q.dialog({
        title: 'Fichier modifié sur le disque',
        message: 'La sauvegarde a été refusée. Recharger la version présente sur le disque ?',
        cancel: { label: 'Conserver mes modifications', flat: true },
        ok: { label: 'Recharger', color: 'primary' },
      }).onOk(() => {
        dirty.value = false
        void selectFile(file)
      })
    }
  } finally {
    saving.value = false
  }
}

function handleShortcut(event: KeyboardEvent) {
  if ((event.ctrlKey || event.metaKey) && event.key.toLocaleLowerCase() === 's') {
    event.preventDefault()
    void saveCurrent()
  }
}

async function loadProjects() {
  loadingProjects.value = true
  try {
    catalog.value = await invoke<ProjectCatalog>('get_catalog')
    const collapse = (sections: ProjectSection[]) => {
      for (const section of sections) {
        section.expanded = false
        section.projects.forEach((project) => { project.expanded = false })
        collapse(section.children)
      }
    }
    collapse(catalog.value.sections)
    catalog.value.unsectionedProjects.forEach((project) => { project.expanded = false })
    unsectionedExpanded.value = false
  } catch (error) {
    $q.notify({ type: 'negative', message: String(error) })
  } finally {
    loadingProjects.value = false
  }
}

function applyCatalog(incoming: ProjectCatalog) {
  const oldProjects = new Map(allProjects.value.map((project) => [project.id, project]))
  const oldSections = new Map(allSections.value.map((section) => [section.id, section]))
  const merge = (sections: ProjectSection[]) => {
    for (const section of sections) {
      section.expanded = oldSections.get(section.id)?.expanded ?? section.expanded
      merge(section.children)
      mergeProjects(section.projects)
    }
  }
  const mergeProjects = (projects: Project[]) => {
    for (const project of projects) {
      const previous = oldProjects.get(project.id)
      if (!previous) continue
      project.expanded = previous.expanded
      if (project.ssh) {
        project.files = previous.files
        project.connectionError = previous.connectionError
        project.loadingFiles = previous.loadingFiles
      } else {
        const oldFiles = new Map(previous.files.map((file) => [file.id, file]))
        project.files = project.files.map((file) => oldFiles.get(file.id) ?? file)
      }
    }
  }
  merge(incoming.sections)
  mergeProjects(incoming.unsectionedProjects)
  catalog.value = incoming
}

async function loadProjectFiles(project: Project) {
  if (!project.ssh || project.loadingFiles) return
  project.loadingFiles = true
  project.connectionError = null
  try {
    const discovered = await invoke<EnvFile[]>('discover_project_files', { projectId: project.id })
    const current = allProjects.value.find((item) => item.id === project.id)
    if (!current) return
    const existing = new Map(current.files.map((file) => [file.id, file]))
    current.files = discovered.map((file) => existing.get(file.id) ?? file)
    current.connectionError = null
  } catch (error) {
    const current = allProjects.value.find((item) => item.id === project.id)
    if (current) current.connectionError = String(error)
  } finally {
    const current = allProjects.value.find((item) => item.id === project.id)
    if (current) current.loadingFiles = false
  }
}

async function refreshProjects() {
  if (!await confirmDiscard('Abandonner les modifications et actualiser les projets ?')) return
  dirty.value = false
  selectedFile.value = null
  sessionId.value = null
  ++templateRequestId
  templateKeys.value = null
  await loadProjects()
}

function showAddProject(sectionId?: string) {
  editingProject.value = null
  projectSectionId.value = sectionId ?? null
  projectMode.value = 'local'
  localDirectory.value = ''
  localName.value = ''
  localEnvironment.value = 'development'
  sshName.value = ''
  sshAlias.value = ''
  sshUser.value = ''
  sshPort.value = ''
  sshDirectory.value = ''
  sshEnvironment.value = 'preproduction'
  projectDialog.value = true
}

function editProject(project: Project) {
  editingProject.value = project
  projectMode.value = project.ssh ? 'ssh' : 'local'
  localDirectory.value = project.ssh ? '' : project.path
  localName.value = project.name
  localEnvironment.value = project.environment ?? 'development'
  sshName.value = project.name
  sshAlias.value = project.ssh?.alias ?? ''
  sshUser.value = project.ssh?.user ?? ''
  sshPort.value = project.ssh?.port ? String(project.ssh.port) : ''
  sshDirectory.value = project.ssh ? project.path.replace(/^ssh:\/\/[^/]+/, '') : ''
  sshEnvironment.value = project.environment ?? project.ssh?.environment ?? 'preproduction'
  projectDialog.value = true
}

async function chooseLocalDirectory() {
  try {
    const selected = await open({ directory: true, multiple: false, title: 'Choisir le dossier du projet' })
    if (typeof selected === 'string') {
      localDirectory.value = selected
      if (!editingProject.value) localName.value = selected.split('/').filter(Boolean).at(-1) ?? ''
    }
  } catch (error) {
    $q.notify({ type: 'negative', message: String(error) })
  }
}

function revealProjectSection() {
  if (!projectSectionId.value) {
    unsectionedExpanded.value = true
    return
  }
  const expand = (sections: ProjectSection[]): boolean => sections.some((section) => {
    if (section.id === projectSectionId.value || expand(section.children)) { section.expanded = true; return true }
    return false
  })
  expand(catalog.value.sections)
}

async function addProject() {
  if (projectAdding.value) return
  if (editingProject.value && selectedProject.value?.id === editingProject.value.id
      && !await confirmDiscard('Abandonner les modifications et modifier ce projet ?')) return
  projectAdding.value = true
  try {
    if (editingProject.value) {
      const project = editingProject.value
      const wasSelected = selectedProject.value?.id === project.id
      applyCatalog(await invoke<ProjectCatalog>('update_project', {
        projectId: project.id, name: projectMode.value === 'local' ? localName.value.trim() : sshName.value.trim(),
        path: projectMode.value === 'local' ? localDirectory.value : sshDirectory.value.trim(),
        environment: projectMode.value === 'local' ? localEnvironment.value : sshEnvironment.value,
        alias: projectMode.value === 'ssh' ? sshAlias.value.trim() : null,
        user: projectMode.value === 'ssh' ? sshUser.value.trim() || null : null,
        port: projectMode.value === 'ssh' && sshPort.value ? Number(sshPort.value) : null,
      }))
      if (wasSelected) {
        selectedFile.value = null
        sessionId.value = null
        dirty.value = false
        editingUnlocked.value = false
        ++templateRequestId
        templateKeys.value = null
      }
    } else if (projectMode.value === 'local') {
      if (!localDirectory.value) return
      applyCatalog(await invoke<ProjectCatalog>('add_project', { path: localDirectory.value, name: localName.value.trim(), environment: localEnvironment.value, sectionId: projectSectionId.value }))
    } else {
      applyCatalog(await invoke<ProjectCatalog>('add_ssh_project', { name: sshName.value.trim(), alias: sshAlias.value.trim(),
        user: sshUser.value.trim() || null, port: sshPort.value ? Number(sshPort.value) : null,
        directory: sshDirectory.value.trim(), environment: sshEnvironment.value, sectionId: projectSectionId.value }))
    }
    if (!editingProject.value) revealProjectSection()
    projectDialog.value = false
    $q.notify({ type: 'positive', message: editingProject.value ? 'Projet modifié' : 'Projet ajouté' })
  } catch (error) {
    $q.notify({ type: 'negative', message: String(error) })
  } finally {
    projectAdding.value = false
  }
}

async function unlockEditing() {
  if (!sessionId.value || !selectedFile.value || !selectedProject.value) return
  const confirmed = await new Promise<boolean>((resolve) => {
    $q.dialog({ title: `Activer l’édition ${selectedEnvironment.value === 'production' ? 'production' : 'préproduction'} ?`,
      message: `${selectedProject.value?.name} · ${selectedFile.value?.path}`,
      cancel: { label: 'Annuler', flat: true }, ok: { label: 'Activer pour ce fichier', color: 'warning' }, persistent: true,
    }).onOk(() => resolve(true)).onCancel(() => resolve(false))
  })
  if (!confirmed) return
  try { await invoke('unlock_project_file', { sessionId: sessionId.value }); editingUnlocked.value = true }
  catch (error) { $q.notify({ type: 'negative', message: String(error) }) }
}

async function removeProject(project: Project) {
  const containsSelection = selectedProject.value?.id === project.id
  if (containsSelection && !await confirmDiscard('Abandonner les modifications et retirer ce projet de la liste ?')) return
  $q.dialog({
    title: 'Retirer le projet',
    message: `Retirer « ${project.name} » de la liste ? Aucun fichier ne sera supprimé.`,
    cancel: { label: 'Annuler', flat: true },
    ok: { label: 'Retirer', color: 'negative' },
  }).onOk(async () => {
    try {
      applyCatalog(await invoke<ProjectCatalog>('remove_project', { path: project.path }))
      if (containsSelection) {
        selectedFile.value = null
        sessionId.value = null
        dirty.value = false
        ++templateRequestId
        templateKeys.value = null
      }
      $q.notify({ type: 'positive', message: 'Projet retiré de la liste — fichiers conservés' })
    } catch (error) {
      $q.notify({ type: 'negative', message: String(error) })
    }
  })
}

function promptSectionName(title: string, initialValue = ''): Promise<string | null> {
  return new Promise((resolve) => {
    $q.dialog({
      title,
      prompt: { model: initialValue, type: 'text', isValid: (value) => value.trim().length > 0 },
      cancel: { label: 'Annuler', flat: true },
      ok: { label: 'Valider' },
    }).onOk((value: string) => resolve(value.trim())).onCancel(() => resolve(null))
  })
}

async function createSection(parentSectionId?: string) {
  const name = await promptSectionName(parentSectionId ? 'Nouvelle sous-section' : 'Nouvelle section')
  if (!name) return
  try {
    applyCatalog(await invoke<ProjectCatalog>('create_section', { name, parentSectionId: parentSectionId ?? null }))
  } catch (error) {
    $q.notify({ type: 'negative', message: String(error) })
  }
}

async function renameSection(section: ProjectSection) {
  const name = await promptSectionName('Renommer la section', section.name)
  if (!name || name === section.name) return
  try {
    applyCatalog(await invoke<ProjectCatalog>('rename_section', { sectionId: section.id, name }))
  } catch (error) {
    $q.notify({ type: 'negative', message: String(error) })
  }
}

function deleteSection(section: ProjectSection) {
  $q.dialog({
    title: 'Supprimer la section',
    message: `Supprimer « ${section.name} » et ses sous-sections ? Tous leurs projets seront déplacés dans « Sans section ».`,
    cancel: { label: 'Annuler', flat: true },
    ok: { label: 'Supprimer', color: 'negative' },
  }).onOk(async () => {
    try {
      applyCatalog(await invoke<ProjectCatalog>('delete_section', { sectionId: section.id }))
    } catch (error) {
      $q.notify({ type: 'negative', message: String(error) })
    }
  })
}

async function moveProject(project: Project, sectionId?: string) {
  try {
    applyCatalog(await invoke<ProjectCatalog>('move_project', { path: project.path, sectionId: sectionId ?? null }))
  } catch (error) {
    $q.notify({ type: 'negative', message: String(error) })
  }
}

async function moveSection(section: ProjectSection, parentSectionId?: string) {
  try {
    applyCatalog(await invoke<ProjectCatalog>('move_section', {
      sectionId: section.id,
      parentSectionId: parentSectionId ?? null,
    }))
  } catch (error) {
    $q.notify({ type: 'negative', message: String(error) })
  }
}

function setSectionExpanded(section: ProjectSection, expanded: boolean) {
  section.expanded = expanded
}

onMounted(async () => {
  void loadProjects()
  window.addEventListener('keydown', handleShortcut)
  unlistenClose = await getCurrentWindow().onCloseRequested(async (event) => {
    if (closingApproved || !dirty.value) return
    event.preventDefault()
    if (await confirmDiscard('Abandonner les modifications et fermer EnvEdit ?')) {
      closingApproved = true
      await getCurrentWindow().close()
    }
  })
})
onBeforeUnmount(() => {
  window.removeEventListener('keydown', handleShortcut)
  unlistenClose?.()
  if (copyFeedbackTimer) clearTimeout(copyFeedbackTimer)
})

async function copyValue(variable: EnvVariable) {
  try {
    await writeText(variable.value)
    copiedVariableId.value = variable.id
    if (copyFeedbackTimer) clearTimeout(copyFeedbackTimer)
    copyFeedbackTimer = setTimeout(() => {
      if (copiedVariableId.value === variable.id) copiedVariableId.value = null
    }, 1400)
  } catch {
    $q.notify({ type: 'negative', message: 'Impossible de copier la valeur' })
  }
}

function updateValue(variable: EnvVariable, value: string | number | null) {
  if (readOnly.value) return
  const nextValue = String(value ?? '')
  editedValueIds.value = new Set(editedValueIds.value).add(variable.id)
  variable.value = nextValue
  dirty.value = true
}

function handleValueInput(variable: EnvVariable, event: Event) {
  const target = event.target
  if (target instanceof HTMLInputElement) updateValue(variable, target.value)
}

function updateKey(variable: EnvVariable, key: string | number | null) {
  if (readOnly.value) return
  variable.key = String(key ?? '')
  variable.sensitive = isSensitiveKey(variable.key)
  dirty.value = true
}

function toggleReveal(id: string) {
  const next = new Set(revealed.value)
  next.has(id) ? next.delete(id) : next.add(id)
  revealed.value = next
}

function addVariable() {
  if (!selectedFile.value || readOnly.value) return
  const index = selectedFile.value.variables.length + 1
  selectedFile.value.variables.push({
    id: crypto.randomUUID(),
    key: `NOUVELLE_VARIABLE_${index}`,
    value: '',
    comment: '',
    sensitive: false,
  })
  dirty.value = true
}

function removeVariable(id: string) {
  if (!selectedFile.value || readOnly.value) return
  selectedFile.value.variables = selectedFile.value.variables.filter((item) => item.id !== id)
  dirty.value = true
}
</script>

<template>
  <q-layout view="hHh Lpr lFf" class="app-shell">
    <q-header class="app-header">
      <q-toolbar>
        <q-avatar square size="32px">
          <img src="/icon.svg" alt="Logo EnvEdit" />
        </q-avatar>
        <q-toolbar-title>
          EnvEdit <span class="app-version">v{{ appVersion }}</span>
          <span class="subtitle">Éditeur de fichiers .env</span>
        </q-toolbar-title>
        <q-btn flat round icon="create_new_folder" aria-label="Ajouter un projet" @click="showAddProject()">
          <q-tooltip>Ajouter un projet</q-tooltip>
        </q-btn>
        <q-btn flat round icon="refresh" aria-label="Actualiser" :loading="loadingProjects" @click="refreshProjects">
          <q-tooltip>Actualiser les fichiers</q-tooltip>
        </q-btn>
        <q-btn color="primary" unelevated icon="save" label="Enregistrer" :disable="!dirty || !sessionId || readOnly" :loading="saving" @click="saveCurrent">
          <q-tooltip>Enregistrer — Ctrl+S</q-tooltip>
        </q-btn>
      </q-toolbar>
    </q-header>

    <q-drawer
      :model-value="true"
      show-if-above
      :breakpoint="0"
      :width="310"
      :mini-width="64"
      :mini="drawerMini"
      bordered
      class="project-drawer"
    >
      <div class="drawer-layout">
        <div class="drawer-scroll">
          <div v-if="drawerMini" class="navigation-rail">
            <q-btn
              flat round
              icon="account_tree"
              class="rail-button"
              aria-label="Ouvrir les projets"
              @click="drawerMini = false"
            >
              <svg
                v-if="projectCount > 0"
                class="rail-count"
                :width="railCountWidth"
                height="20"
                :viewBox="`0 0 ${railCountWidth} 20`"
                aria-hidden="true"
              >
                <rect x="1" y="1" :width="railCountWidth - 2" height="18" rx="9" />
                <text :x="railCountWidth / 2" y="10">{{ projectCount }}</text>
              </svg>
              <q-tooltip anchor="center right" self="center left" :offset="[10, 0]">
                Projets — ouvrir le panneau
              </q-tooltip>
            </q-btn>
          </div>
          <template v-else>
          <div class="drawer-heading">
            <span>Projets</span>
            <q-btn flat round dense icon="create_new_folder" aria-label="Nouvelle section" @click="createSection()">
              <q-tooltip>Nouvelle section</q-tooltip>
            </q-btn>
          </div>
          <div v-if="!loadingProjects && projectCount === 0 && catalog.sections.length === 0" class="no-projects q-mini-drawer-hide">
            <q-icon name="create_new_folder" size="36px" />
            <div>Aucun projet</div>
            <q-btn flat color="primary" label="Ajouter un projet" @click="showAddProject()" />
            <q-btn flat color="primary" label="Créer une section" @click="createSection()" />
          </div>
          <q-list padding>
            <SectionTree
              :sections="catalog.sections"
              :destinations="allSections"
              :drawer-mini="drawerMini"
              :selected-file-id="selectedFile?.id"
              @select="selectFile"
              @expand-project="loadProjectFiles"
              @remove-project="removeProject"
              @edit-project="editProject"
              @move-project="moveProject"
              @add-project="showAddProject"
              @create-child="createSection"
              @rename="renameSection"
              @delete="deleteSection"
              @move-section="moveSection"
              @set-expanded="setSectionExpanded"
            />

            <div v-if="catalog.unsectionedProjects.length > 0" class="tree-node section-node">
              <div class="tree-row section-row" @click="unsectionedExpanded = !unsectionedExpanded">
                <q-icon class="tree-chevron" :name="unsectionedExpanded ? 'keyboard_arrow_down' : 'keyboard_arrow_right'" />
                <q-icon class="tree-kind-icon" name="folder_off" />
                <span class="tree-label section-name q-mini-drawer-hide">Sans section</span>
              </div>
              <ProjectTree
                v-if="unsectionedExpanded"
                :projects="catalog.unsectionedProjects"
                :sections="allSections"
                :drawer-mini="drawerMini"
                :selected-file-id="selectedFile?.id"
                :depth="1"
                @select="selectFile"
                @expand="loadProjectFiles"
                @remove="removeProject"
                @edit="editProject"
                @move="moveProject"
              />
            </div>
          </q-list>
          </template>
        </div>
        <div class="drawer-footer">
          <q-separator />
          <q-btn
            flat round dense
            :icon="drawerMini ? 'keyboard_double_arrow_right' : 'keyboard_double_arrow_left'"
            :aria-label="drawerMini ? 'Déplier le panneau des projets' : 'Replier le panneau des projets'"
            @click="drawerMini = !drawerMini"
          >
            <q-tooltip>{{ drawerMini ? 'Déplier le panneau' : 'Replier le panneau' }}</q-tooltip>
          </q-btn>
        </div>
      </div>
    </q-drawer>

    <q-page-container>
      <q-page class="editor-page">
        <template v-if="selectedFile">
          <div class="editor-heading">
            <div>
              <div class="file-title"><span v-if="dirty" class="dirty-dot">●</span>{{ selectedFile.name }}</div>
              <div class="file-path">{{ selectedFile.path }}</div>
              <div v-if="selectedProject" class="remote-status">
                <q-badge :color="selectedEnvironment === 'production' ? 'negative' : selectedEnvironment === 'preproduction' ? 'warning' : 'info'">
                  {{ selectedSsh ? 'SSH' : 'LOCAL' }} · {{ selectedEnvironment === 'production' ? 'PRODUCTION' : selectedEnvironment === 'preproduction' ? 'PRÉPRODUCTION' : 'DÉVELOPPEMENT' }}
                </q-badge>
                <q-btn v-if="readOnly" flat dense no-caps icon="lock" label="Lecture seule · Activer l’édition" @click="unlockEditing" />
                <q-badge v-else-if="selectedEnvironment !== 'development'" color="warning">Édition activée pour ce fichier</q-badge>
              </div>
            </div>
            <q-btn unelevated rounded no-caps color="primary" icon="add" label="Nouvelle variable" :disable="readOnly" @click="addVariable">
              <q-tooltip>Ajouter une variable à la fin du fichier</q-tooltip>
            </q-btn>
          </div>

          <q-card v-if="canCompare" flat bordered class="comparison-card">
            <div class="comparison-heading">
              <q-btn flat no-caps class="comparison-toggle" :icon="comparisonOpen ? 'expand_less' : 'compare_arrows'"
                :label="templateLoading ? 'Comparaison en cours…' : templateError ? 'Comparaison indisponible' : templateKeys ? comparisonSummary : 'Comparer les clés'"
                :aria-label="comparisonOpen ? 'Replier la comparaison' : 'Voir la comparaison'" @click="comparisonOpen = !comparisonOpen" />
              <span class="comparison-reference">avec {{ selectedTemplateName }}</span>
              <q-spinner v-if="templateLoading" size="18px" color="primary" />
            </div>
            <div v-if="comparisonOpen" class="comparison-body">
              <q-select v-if="availableTemplates.length > 1" :model-value="selectedTemplateName" :options="availableTemplates"
                dense outlined label="Fichier de référence" class="comparison-select" @update:model-value="changeTemplate" />
              <div v-if="templateError" class="text-negative">
                {{ templateError }}
                <q-btn flat dense no-caps color="primary" label="Réessayer" @click="selectedProject && selectedTemplateName && loadTemplateKeys(selectedProject.id, selectedTemplateName)" />
              </div>
              <template v-else-if="templateKeys">
                <div v-if="templateKeys.unknownLines || templateKeys.duplicateKeys.length" class="comparison-warning">
                  Comparaison partielle : {{ templateKeys.unknownLines }} ligne(s) non reconnue(s), {{ templateKeys.duplicateKeys.length }} clé(s) dupliquée(s) dans l'exemple.
                </div>
                <div v-if="!keyComparison.missing.length && !keyComparison.extra.length" class="comparison-success">
                  <q-icon name="check_circle" /> Les mêmes clés sont présentes dans les deux fichiers.
                </div>
                <template v-else>
                  <div class="comparison-section-title">
                    <span>Manquantes dans {{ selectedFile.name }} ({{ keyComparison.missing.length }})</span>
                    <q-btn v-if="keyComparison.missing.length" flat dense no-caps color="primary" icon="playlist_add"
                      label="Tout ajouter" :disable="readOnly" @click="addMissingKeys(keyComparison.missing)" />
                  </div>
                  <div v-if="!keyComparison.missing.length" class="comparison-empty">Aucune clé manquante.</div>
                  <div v-for="key in keyComparison.missing" :key="`missing-${key}`" class="comparison-row">
                    <code>{{ key }}</code>
                    <q-btn flat dense no-caps color="primary" icon="add" label="Ajouter" :disable="readOnly" :aria-label="`Ajouter ${key}`" @click="addMissingKeys([key])" />
                  </div>
                  <div class="comparison-section-title">Présentes seulement dans {{ selectedFile.name }} ({{ keyComparison.extra.length }})</div>
                  <div v-if="!keyComparison.extra.length" class="comparison-empty">Aucune clé supplémentaire.</div>
                  <div v-for="key in keyComparison.extra" :key="`extra-${key}`" class="comparison-row"><code>{{ key }}</code></div>
                </template>
                <div class="comparison-note">Seuls les noms des clés sont comparés. Les ajouts reprennent le commentaire de l'exemple, mais jamais sa valeur ; ils ne sont écrits qu'à l'enregistrement.</div>
              </template>
            </div>
          </q-card>

          <q-input v-model="query" outlined dense clearable debounce="80" placeholder="Rechercher une clé ou un commentaire" class="search-field">
            <template #prepend><q-icon name="search" /></template>
          </q-input>

          <q-banner v-for="diagnostic in selectedFile.diagnostics" :key="diagnostic" rounded class="diagnostic-banner">
            <template #avatar><q-icon name="warning_amber" color="warning" /></template>
            {{ diagnostic }}
          </q-banner>

          <div class="variable-count">
            <q-spinner v-if="loadingFile" size="16px" />
            {{ displayedVariables.length }} variable(s) affichée(s)
          </div>
          <q-list bordered separator class="variable-list">
            <q-item v-for="variable in displayedVariables" :key="variable.id" class="variable-row">
              <q-item-section class="key-column">
                <q-input :model-value="variable.key" borderless dense aria-label="Nom de variable" :readonly="readOnly" @update:model-value="updateKey(variable, $event)" />
              </q-item-section>
              <q-item-section @input.capture="handleValueInput(variable, $event)">
                <q-input
                  :model-value="variable.value"
                  :type="variable.sensitive && !revealed.has(variable.id) ? 'password' : 'text'"
                  outlined dense aria-label="Valeur" :readonly="readOnly"
                >
                  <template v-if="variable.sensitive" #append>
                    <q-icon
                      :name="revealed.has(variable.id) ? 'visibility_off' : 'visibility'"
                      class="cursor-pointer"
                      role="button"
                      tabindex="0"
                      aria-label="Afficher ou masquer la valeur"
                      @click="toggleReveal(variable.id)"
                      @keydown.enter="toggleReveal(variable.id)"
                    />
                  </template>
                </q-input>
                <div v-if="isBlankValue(variable.value)" class="empty-value-hint">
                  <q-icon name="warning_amber" size="14px" />
                  {{ variable.value.length ? 'Valeur composée d’espaces' : 'Valeur vide' }}
                </div>
              </q-item-section>
              <q-item-section side>
                <q-btn
                  flat round
                  :icon="copiedVariableId === variable.id ? 'check' : 'content_copy'"
                  :color="copiedVariableId === variable.id ? 'positive' : undefined"
                  :aria-label="variable.sensitive ? 'Copier la valeur masquée' : 'Copier la valeur'"
                  @click="copyValue(variable)"
                >
                  <q-tooltip>{{ copiedVariableId === variable.id ? 'Valeur copiée' : (variable.sensitive ? 'Copier la valeur masquée' : 'Copier la valeur') }}</q-tooltip>
                </q-btn>
              </q-item-section>
              <q-item-section side>
                <q-btn flat round icon="info_outline" :aria-label="variable.comment || 'Ajouter un commentaire'">
                  <q-tooltip>{{ variable.comment || 'Ajouter un commentaire' }}</q-tooltip>
                  <q-popup-edit v-if="!readOnly" v-model="variable.comment" buttons label-set="Appliquer" label-cancel="Annuler" @save="dirty = true" v-slot="scope">
                    <q-input v-model="scope.value" type="textarea" autofocus outlined label="Commentaire" @keyup.enter.stop />
                  </q-popup-edit>
                </q-btn>
              </q-item-section>
              <q-item-section side>
                <q-btn flat round icon="delete_outline" aria-label="Supprimer la variable" :disable="readOnly" @click="removeVariable(variable.id)" />
              </q-item-section>
            </q-item>
          </q-list>
          <q-banner v-if="displayedVariables.length === 0" rounded class="empty-state">
            Aucune variable ne correspond à cette recherche.
          </q-banner>
        </template>
        <div v-else class="empty-page">
          <q-icon name="description" size="64px" />
          <div>Sélectionnez un fichier .env</div>
        </div>
      </q-page>
    </q-page-container>
    <q-dialog v-model="projectDialog">
      <q-card class="project-dialog">
        <q-card-section><div class="text-h6">{{ editingProject ? 'Modifier le projet' : 'Ajouter un projet' }}</div></q-card-section>
        <q-tabs v-if="!editingProject" v-model="projectMode" dense align="left" active-color="primary" indicator-color="primary" class="project-mode-tabs">
          <q-tab name="local" icon="folder" label="Local" no-caps />
          <q-tab name="ssh" icon="dns" label="SSH" no-caps />
        </q-tabs>
        <q-separator />
        <q-card-section v-if="projectMode === 'local'" class="project-local-content">
          <div class="text-body2 q-mb-md">Choisissez le dossier contenant les fichiers .env du projet.</div>
          <q-input v-model="localName" outlined label="Nom du projet" class="q-mb-md" />
          <q-btn outline no-caps color="primary" icon="folder_open" label="Choisir un dossier…" @click="chooseLocalDirectory" />
          <div v-if="localDirectory" class="project-local-path" :title="localDirectory">{{ localDirectory }}</div>
          <q-select v-model="localEnvironment" outlined emit-value map-options label="Environnement" class="q-mt-md" :options="environmentOptions" />
          <div class="text-caption q-mt-sm">Préproduction et production s'ouvrent en lecture seule.</div>
        </q-card-section>
        <q-card-section v-else class="q-gutter-md project-ssh-content">
          <q-input v-model="sshName" outlined label="Nom du projet" />
          <q-input v-model="sshAlias" outlined label="Hôte ou alias SSH" hint="Adresse IP, nom DNS ou alias ~/.ssh/config" />
          <q-input v-model="sshUser" outlined label="Utilisateur SSH" hint="Laissez vide si défini dans ~/.ssh/config" />
          <q-input v-model="sshDirectory" outlined label="Dossier distant" hint="Chemin absolu, par exemple /srv/mon-app" />
          <q-expansion-item class="project-ssh-options" label="Options : port, environnement" dense expand-separator :duration="0">
            <div class="q-gutter-md q-pa-md">
              <q-input v-model="sshPort" outlined label="Port SSH (optionnel)" hint="Laisser vide pour utiliser la configuration SSH ou le port 22" inputmode="numeric" />
              <q-select v-model="sshEnvironment" outlined emit-value map-options label="Environnement"
                :options="environmentOptions" />
              <div class="text-caption">Préproduction et production s'ouvrent en lecture seule.</div>
            </div>
          </q-expansion-item>
        </q-card-section>
        <q-card-actions align="right">
          <q-btn flat label="Annuler" :disable="projectAdding" v-close-popup />
          <q-btn color="primary" :label="editingProject ? 'Enregistrer le projet' : 'Ajouter le projet'" :disable="!canAddProject" :loading="projectAdding" @click="addProject" />
        </q-card-actions>
      </q-card>
    </q-dialog>
  </q-layout>
</template>
