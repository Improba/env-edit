<script setup lang="ts">
import type { EnvFile, Project, ProjectSection } from './model'

withDefaults(defineProps<{
  projects: Project[]
  sections: ProjectSection[]
  drawerMini: boolean
  selectedFileId?: string
  depth?: number
}>(), { depth: 0 })

const emit = defineEmits<{
  select: [file: EnvFile]
  remove: [project: Project]
  edit: [project: Project]
  move: [project: Project, sectionId?: string]
  expand: [project: Project]
}>()

function toggleProject(project: Project) {
  project.expanded = !project.expanded
  if (project.expanded && project.ssh) emit('expand', project)
}
</script>

<template>
  <div v-for="project in projects" :key="project.id" class="tree-node project-node">
    <div class="tree-row project-row" :style="{ paddingLeft: `${8 + depth * 18}px` }" @click="toggleProject(project)">
      <q-icon class="tree-chevron" :name="project.expanded ? 'keyboard_arrow_down' : 'keyboard_arrow_right'" />
      <q-icon class="tree-kind-icon" :name="project.ssh ? 'dns' : project.expanded ? 'folder_open' : 'folder'" />
      <span class="tree-label q-mini-drawer-hide">{{ project.name }}</span>
      <q-badge class="q-mini-drawer-hide" :color="project.environment === 'production' ? 'negative' : project.environment === 'preproduction' ? 'warning' : 'info'">
        {{ project.environment === 'production' ? 'PROD' : project.environment === 'preproduction' ? 'PRÉPROD' : 'DEV' }}
      </q-badge>
      <span class="tree-spacer" />
      <q-btn class="tree-actions q-mini-drawer-hide" flat round dense icon="more_horiz" aria-label="Actions du projet" @click.stop>
        <q-menu anchor="bottom right" self="top right">
          <q-list class="project-menu">
            <q-item v-close-popup clickable @click="$emit('edit', project)">
              <q-item-section avatar><q-icon name="tune" /></q-item-section>
              <q-item-section>Modifier le projet</q-item-section>
            </q-item>
            <q-separator />
            <q-item-label header>Déplacer vers</q-item-label>
            <q-item v-close-popup clickable @click="$emit('move', project)">
              <q-item-section avatar><q-icon name="folder_off" /></q-item-section>
              <q-item-section>Sans section</q-item-section>
            </q-item>
            <q-item v-for="section in sections" :key="section.id" v-close-popup clickable @click="$emit('move', project, section.id)">
              <q-item-section avatar><q-icon name="folder" /></q-item-section>
              <q-item-section>{{ section.name }}</q-item-section>
            </q-item>
            <q-separator />
            <q-item v-close-popup clickable @click="$emit('remove', project)">
              <q-item-section avatar><q-icon name="remove_circle_outline" color="negative" /></q-item-section>
              <q-item-section><q-item-label>Retirer de la liste</q-item-label><q-item-label caption>Les fichiers seront conservés</q-item-label></q-item-section>
            </q-item>
          </q-list>
        </q-menu>
      </q-btn>
      <q-tooltip>{{ project.path }}</q-tooltip>
    </div>
    <div v-if="project.expanded" class="tree-children" :style="{ marginLeft: `${25 + depth * 18}px` }">
      <div v-if="project.loadingFiles" class="tree-empty q-mini-drawer-hide"><q-spinner size="15px" class="q-mr-xs" /> Connexion SSH…</div>
      <div v-for="file in project.files" :key="file.id" class="tree-row file-row" :class="{ 'active-file': selectedFileId === file.id }" @click="$emit('select', file)">
        <span class="tree-elbow" />
        <q-icon class="tree-kind-icon" name="description" />
        <span class="tree-label q-mini-drawer-hide">{{ file.name }}</span>
        <q-tooltip v-if="drawerMini">{{ file.name }}</q-tooltip>
      </div>
      <div v-if="project.connectionError" class="tree-empty q-mini-drawer-hide text-negative">{{ project.connectionError }}</div>
      <div v-else-if="!project.loadingFiles && project.files.length === 0" class="tree-empty q-mini-drawer-hide">Aucun fichier .env</div>
    </div>
  </div>
</template>
