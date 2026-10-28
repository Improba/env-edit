<script setup lang="ts">
import type { EnvFile, Project, ProjectSection } from './model'
import ProjectTree from './ProjectTree.vue'

defineOptions({ name: 'SectionTree' })
withDefaults(defineProps<{ sections: ProjectSection[]; destinations: ProjectSection[]; drawerMini: boolean; selectedFileId?: string; depth?: number }>(), { depth: 0 })
defineEmits<{
  select: [file: EnvFile]; removeProject: [project: Project]; editProject: [project: Project]; moveProject: [project: Project, sectionId?: string]
  expandProject: [project: Project]
  addProject: [sectionId: string]; createChild: [sectionId: string]; rename: [section: ProjectSection]
  delete: [section: ProjectSection]; moveSection: [section: ProjectSection, parentSectionId?: string]
  setExpanded: [section: ProjectSection, expanded: boolean]
}>()

function containsSection(section: ProjectSection, id: string): boolean {
  return section.id === id || section.children.some((child) => containsSection(child, id))
}
function destinationsFor(section: ProjectSection, destinations: ProjectSection[]): ProjectSection[] {
  return destinations.filter((destination) => !containsSection(section, destination.id))
}
</script>

<template>
  <div v-for="section in sections" :key="section.id" class="tree-node section-node">
    <div class="tree-row section-row" :style="{ paddingLeft: `${8 + depth * 18}px` }" @click="$emit('setExpanded', section, !section.expanded)">
      <q-icon class="tree-chevron" :name="section.expanded ? 'keyboard_arrow_down' : 'keyboard_arrow_right'" />
      <q-icon class="tree-kind-icon" :name="section.expanded ? 'folder_open' : 'folder'" />
      <span class="tree-label section-name q-mini-drawer-hide">{{ section.name }}</span>
      <span class="tree-spacer" />
      <q-btn class="tree-actions q-mini-drawer-hide" flat round dense icon="more_horiz" aria-label="Actions de la section" @click.stop>
        <q-menu anchor="bottom right" self="top right">
          <q-list class="project-menu">
            <q-item v-close-popup clickable @click="$emit('createChild', section.id)"><q-item-section avatar><q-icon name="create_new_folder" /></q-item-section><q-item-section>Nouvelle sous-section</q-item-section></q-item>
            <q-item v-close-popup clickable @click="$emit('addProject', section.id)"><q-item-section avatar><q-icon name="add" /></q-item-section><q-item-section>Ajouter un projet</q-item-section></q-item>
            <q-item v-close-popup clickable @click="$emit('rename', section)"><q-item-section avatar><q-icon name="edit" /></q-item-section><q-item-section>Renommer</q-item-section></q-item>
            <q-item-label header>Déplacer vers</q-item-label>
            <q-item v-close-popup clickable @click="$emit('moveSection', section)"><q-item-section avatar><q-icon name="vertical_align_top" /></q-item-section><q-item-section>Racine</q-item-section></q-item>
            <q-item v-for="destination in destinationsFor(section, destinations)" :key="destination.id" v-close-popup clickable @click="$emit('moveSection', section, destination.id)"><q-item-section avatar><q-icon name="drive_file_move" /></q-item-section><q-item-section>{{ destination.name }}</q-item-section></q-item>
            <q-separator />
            <q-item v-close-popup clickable @click="$emit('delete', section)"><q-item-section avatar><q-icon name="delete_outline" color="negative" /></q-item-section><q-item-section>Supprimer</q-item-section></q-item>
          </q-list>
        </q-menu>
      </q-btn>
      <q-tooltip v-if="drawerMini">{{ section.name }}</q-tooltip>
    </div>
    <div v-if="section.expanded" class="tree-branch">
      <SectionTree :sections="section.children" :destinations="destinations" :drawer-mini="drawerMini" :selected-file-id="selectedFileId" :depth="depth + 1"
        @select="$emit('select', $event)" @remove-project="$emit('removeProject', $event)" @edit-project="$emit('editProject', $event)" @move-project="(project, sectionId) => $emit('moveProject', project, sectionId)" @expand-project="$emit('expandProject', $event)"
        @add-project="$emit('addProject', $event)" @create-child="$emit('createChild', $event)" @rename="$emit('rename', $event)" @delete="$emit('delete', $event)"
        @move-section="(child, parentId) => $emit('moveSection', child, parentId)" @set-expanded="(child, expanded) => $emit('setExpanded', child, expanded)" />
      <ProjectTree :projects="section.projects" :sections="destinations" :drawer-mini="drawerMini" :selected-file-id="selectedFileId" :depth="depth + 1"
        @select="$emit('select', $event)" @remove="$emit('removeProject', $event)" @edit="$emit('editProject', $event)" @move="(project, sectionId) => $emit('moveProject', project, sectionId)" @expand="$emit('expandProject', $event)" />
      <button v-if="section.projects.length === 0 && section.children.length === 0" type="button" class="tree-add q-mini-drawer-hide" :style="{ marginLeft: `${29 + depth * 18}px` }" @click="$emit('addProject', section.id)">+ Ajouter un projet</button>
    </div>
  </div>
</template>
