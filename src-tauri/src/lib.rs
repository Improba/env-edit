use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Manager;

mod parser;
#[cfg(unix)]
mod remote;

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);

struct DocumentSession {
    path: PathBuf,
    document: parser::Document,
    original_len: u64,
    original_modified: Option<SystemTime>,
    original_hash: [u8; 32],
    locked: bool,
    protected: bool,
    #[cfg(unix)]
    mode: u32,
    #[cfg(unix)]
    remote: Option<RemoteSession>,
}

#[cfg(unix)]
struct RemoteSession {
    alias: String,
    user: Option<String>,
    port: Option<u16>,
    path: String,
    fingerprint: String,
    mode: String,
}

#[derive(Default)]
struct SessionStore(Mutex<HashMap<String, DocumentSession>>);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OpenDocumentDto {
    session_id: String,
    variables: Vec<parser::VariableDto>,
    diagnostics: Vec<String>,
    unknown_lines: Vec<usize>,
    duplicate_keys: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VariableEdit {
    line_index: Option<usize>,
    key: String,
    value: String,
    comment: String,
}

#[tauri::command]
fn backend_status() -> &'static str {
    "EnvEdit portable backend ready"
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectRecord {
    name: String,
    path: PathBuf,
    #[serde(default = "default_environment")]
    environment: String,
    #[serde(default)]
    ssh: Option<SshProject>,
}

fn default_environment() -> String {
    "development".into()
}

fn valid_environment(value: &str) -> bool {
    ["development", "preproduction", "production"].contains(&value)
}

fn project_environment(project: &ProjectRecord) -> &str {
    project
        .ssh
        .as_ref()
        .map_or(&project.environment, |ssh| &ssh.environment)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SshProject {
    alias: String,
    #[serde(default)]
    user: Option<String>,
    #[serde(default)]
    port: Option<u16>,
    environment: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SectionRecord {
    id: String,
    name: String,
    expanded: bool,
    projects: Vec<ProjectRecord>,
    #[serde(default)]
    children: Vec<SectionRecord>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Catalog {
    sections: Vec<SectionRecord>,
    unsectioned_projects: Vec<ProjectRecord>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EnvFileDto {
    id: String,
    name: String,
    path: String,
    variables: Vec<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectDto {
    id: String,
    name: String,
    path: String,
    environment: String,
    expanded: bool,
    files: Vec<EnvFileDto>,
    ssh: Option<SshProject>,
    connection_error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SectionDto {
    id: String,
    name: String,
    expanded: bool,
    projects: Vec<ProjectDto>,
    children: Vec<SectionDto>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CatalogDto {
    sections: Vec<SectionDto>,
    unsectioned_projects: Vec<ProjectDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TemplateKeysDto {
    keys: Vec<String>,
    comments: BTreeMap<String, String>,
    unknown_lines: usize,
    duplicate_keys: Vec<String>,
}

fn template_keys(data: &[u8]) -> Result<TemplateKeysDto, String> {
    let parsed = parser::parse_bytes(data)?.to_dto();
    let mut comments = BTreeMap::new();
    for variable in &parsed.variables {
        if !variable.comment.is_empty() && !parsed.duplicate_keys.contains(&variable.key) {
            comments.insert(variable.key.clone(), variable.comment.clone());
        }
    }
    let mut keys = parsed
        .variables
        .into_iter()
        .map(|variable| variable.key)
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    Ok(TemplateKeysDto {
        keys,
        comments,
        unknown_lines: parsed.unknown_lines.len(),
        duplicate_keys: parsed.duplicate_keys,
    })
}

fn valid_template_name(name: &str) -> bool {
    name == ".env.example" || name == ".env.exemple"
}

fn is_env_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == ".env" || name.starts_with(".env."))
}

fn discover_env_files(project: &Path) -> Result<Vec<EnvFileDto>, String> {
    let entries = fs::read_dir(project)
        .map_err(|error| format!("Impossible de parcourir le dossier : {error}"))?;
    let mut files = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && is_env_file(path))
        .map(|path| EnvFileDto {
            id: path.to_string_lossy().into_owned(),
            name: path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            path: path.to_string_lossy().into_owned(),
            variables: Vec::new(),
        })
        .collect::<Vec<_>>();
    files.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(files)
}

fn project_to_dto(project: ProjectRecord) -> Result<ProjectDto, String> {
    let path = project_identity(&project);
    let files = if project.ssh.is_some() {
        Vec::new()
    } else {
        discover_env_files(&project.path)?
    };
    Ok(ProjectDto {
        id: path.clone(),
        name: project.name,
        path,
        environment: project
            .ssh
            .as_ref()
            .map_or(&project.environment, |ssh| &ssh.environment)
            .clone(),
        expanded: false,
        files,
        ssh: project.ssh,
        connection_error: None,
    })
}

#[cfg(unix)]
fn discover_ssh_files(project: &ProjectRecord) -> Result<Vec<EnvFileDto>, String> {
    let ssh = project.ssh.as_ref().ok_or("Ce projet n'est pas SSH")?;
    let path = project_identity(project);
    remote::list(
        remote::Connection {
            alias: &ssh.alias,
            user: ssh.user.as_deref(),
            port: ssh.port,
        },
        &project.path.to_string_lossy(),
    )
    .map(|names| {
        names
            .into_iter()
            .map(|name| {
                let id = format!("{path}/{name}");
                EnvFileDto {
                    id: id.clone(),
                    name,
                    path: id,
                    variables: Vec::new(),
                }
            })
            .collect()
    })
}

fn project_identity(project: &ProjectRecord) -> String {
    if let Some(ssh) = &project.ssh {
        format!("ssh://{}{}", ssh.alias, project.path.to_string_lossy())
    } else {
        project.path.to_string_lossy().into_owned()
    }
}

fn catalog_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|directory| directory.join("catalog.json"))
        .map_err(|error| format!("Dossier de configuration indisponible : {error}"))
}

fn read_catalog(app: &tauri::AppHandle) -> Result<Catalog, String> {
    let path = catalog_path(app)?;
    if !path.exists() {
        return Ok(Catalog::default());
    }
    let data =
        fs::read(&path).map_err(|error| format!("Catalogue de projets illisible : {error}"))?;
    serde_json::from_slice(&data)
        .map_err(|error| format!("Catalogue de projets invalide : {error}"))
}

fn write_catalog(app: &tauri::AppHandle, catalog: &Catalog) -> Result<(), String> {
    let path = catalog_path(app)?;
    let directory = path
        .parent()
        .ok_or_else(|| "Dossier de configuration invalide".to_string())?;
    fs::create_dir_all(directory)
        .map_err(|error| format!("Impossible de créer le dossier de configuration : {error}"))?;
    let temporary = path.with_extension("json.tmp");
    let data = serde_json::to_vec_pretty(catalog)
        .map_err(|error| format!("Impossible de préparer le catalogue : {error}"))?;
    fs::write(&temporary, data)
        .map_err(|error| format!("Impossible d'écrire le catalogue : {error}"))?;
    fs::rename(&temporary, &path)
        .map_err(|error| format!("Impossible d'enregistrer le catalogue : {error}"))
}

fn projects_to_dto(projects: Vec<ProjectRecord>) -> Result<Vec<ProjectDto>, String> {
    projects
        .into_iter()
        .filter(|project| project.ssh.is_some() || project.path.is_dir())
        .map(project_to_dto)
        .collect()
}

fn catalog_to_dto(catalog: Catalog) -> Result<CatalogDto, String> {
    fn section_to_dto(section: SectionRecord) -> Result<SectionDto, String> {
        Ok(SectionDto {
            id: section.id,
            name: section.name,
            expanded: section.expanded,
            projects: projects_to_dto(section.projects)?,
            children: section
                .children
                .into_iter()
                .map(section_to_dto)
                .collect::<Result<Vec<_>, String>>()?,
        })
    }
    let sections = catalog
        .sections
        .into_iter()
        .map(section_to_dto)
        .collect::<Result<Vec<_>, String>>()?;
    Ok(CatalogDto {
        sections,
        unsectioned_projects: projects_to_dto(catalog.unsectioned_projects)?,
    })
}

fn sections_contain_project(sections: &[SectionRecord], path: &Path) -> bool {
    sections.iter().any(|section| {
        section
            .projects
            .iter()
            .any(|project| project_identity(project) == path.to_string_lossy())
            || sections_contain_project(&section.children, path)
    })
}

fn catalog_contains_project(catalog: &Catalog, path: &Path) -> bool {
    catalog
        .unsectioned_projects
        .iter()
        .any(|project| project_identity(project) == path.to_string_lossy())
        || sections_contain_project(&catalog.sections, path)
}

fn find_local_project<'a>(catalog: &'a Catalog, path: &Path) -> Option<&'a ProjectRecord> {
    fn in_sections<'a>(sections: &'a [SectionRecord], path: &Path) -> Option<&'a ProjectRecord> {
        for section in sections {
            if let Some(project) = section.projects.iter().find(|project| {
                project.ssh.is_none() && path.parent() == Some(project.path.as_path())
            }) {
                return Some(project);
            }
            if let Some(project) = in_sections(&section.children, path) {
                return Some(project);
            }
        }
        None
    }
    catalog
        .unsectioned_projects
        .iter()
        .find(|project| project.ssh.is_none() && path.parent() == Some(project.path.as_path()))
        .or_else(|| in_sections(&catalog.sections, path))
}

fn find_project<'a>(catalog: &'a Catalog, id: &str) -> Option<&'a ProjectRecord> {
    fn in_sections<'a>(sections: &'a [SectionRecord], id: &str) -> Option<&'a ProjectRecord> {
        for section in sections {
            if let Some(project) = section
                .projects
                .iter()
                .find(|project| project_identity(project) == id)
            {
                return Some(project);
            }
            if let Some(project) = in_sections(&section.children, id) {
                return Some(project);
            }
        }
        None
    }
    catalog
        .unsectioned_projects
        .iter()
        .find(|project| project_identity(project) == id)
        .or_else(|| in_sections(&catalog.sections, id))
}

fn find_project_mut<'a>(catalog: &'a mut Catalog, id: &str) -> Option<&'a mut ProjectRecord> {
    fn in_sections<'a>(
        sections: &'a mut [SectionRecord],
        id: &str,
    ) -> Option<&'a mut ProjectRecord> {
        for section in sections {
            if let Some(project) = section
                .projects
                .iter_mut()
                .find(|project| project_identity(project) == id)
            {
                return Some(project);
            }
            if let Some(project) = in_sections(&mut section.children, id) {
                return Some(project);
            }
        }
        None
    }
    catalog
        .unsectioned_projects
        .iter_mut()
        .find(|project| project_identity(project) == id)
        .or_else(|| in_sections(&mut catalog.sections, id))
}

fn find_section_mut<'a>(
    sections: &'a mut [SectionRecord],
    id: &str,
) -> Option<&'a mut SectionRecord> {
    for section in sections {
        if section.id == id {
            return Some(section);
        }
        if let Some(found) = find_section_mut(&mut section.children, id) {
            return Some(found);
        }
    }
    None
}

fn take_project(catalog: &mut Catalog, path: &Path) -> Option<ProjectRecord> {
    if let Some(index) = catalog
        .unsectioned_projects
        .iter()
        .position(|project| project_identity(project) == path.to_string_lossy())
    {
        return Some(catalog.unsectioned_projects.remove(index));
    }
    fn take_from_sections(sections: &mut [SectionRecord], path: &Path) -> Option<ProjectRecord> {
        for section in sections {
            if let Some(index) = section
                .projects
                .iter()
                .position(|project| project_identity(project) == path.to_string_lossy())
            {
                return Some(section.projects.remove(index));
            }
            if let Some(project) = take_from_sections(&mut section.children, path) {
                return Some(project);
            }
        }
        None
    }
    take_from_sections(&mut catalog.sections, path)
}

fn place_project(
    catalog: &mut Catalog,
    project: ProjectRecord,
    section_id: Option<&str>,
) -> Result<(), String> {
    match section_id {
        Some(id) => find_section_mut(&mut catalog.sections, id)
            .ok_or_else(|| "Section introuvable".to_string())?
            .projects
            .push(project),
        None => catalog.unsectioned_projects.push(project),
    }
    Ok(())
}

#[tauri::command]
fn get_catalog(app: tauri::AppHandle) -> Result<CatalogDto, String> {
    catalog_to_dto(read_catalog(&app)?)
}

#[cfg(unix)]
#[tauri::command]
async fn discover_project_files(
    app: tauri::AppHandle,
    project_id: String,
) -> Result<Vec<EnvFileDto>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let catalog = read_catalog(&app)?;
        let project = find_remote_project(&catalog, &project_id)
            .ok_or("Projet SSH introuvable dans le catalogue")?;
        discover_ssh_files(project)
    })
    .await
    .map_err(|_| "Découverte SSH interrompue".to_string())?
}

#[cfg(not(unix))]
#[tauri::command]
async fn discover_project_files(
    _app: tauri::AppHandle,
    _project_id: String,
) -> Result<Vec<EnvFileDto>, String> {
    Err("Projet SSH indisponible sur cette plateforme".into())
}

#[tauri::command]
async fn get_env_template_keys(
    app: tauri::AppHandle,
    project_id: String,
    template_name: String,
) -> Result<TemplateKeysDto, String> {
    tauri::async_runtime::spawn_blocking(move || {
        if !valid_template_name(&template_name) {
            return Err("Nom de fichier exemple invalide".into());
        }
        let catalog = read_catalog(&app)?;
        let project =
            find_project(&catalog, &project_id).ok_or("Projet introuvable dans le catalogue")?;
        let path = project.path.join(&template_name);
        let data = if let Some(ssh) = &project.ssh {
            #[cfg(unix)]
            {
                remote::read(
                    remote::Connection {
                        alias: &ssh.alias,
                        user: ssh.user.as_deref(),
                        port: ssh.port,
                    },
                    &path.to_string_lossy(),
                )?
                .data
            }
            #[cfg(not(unix))]
            {
                let _ = ssh;
                return Err("Projet SSH indisponible sur cette plateforme".into());
            }
        } else {
            if path.is_symlink() {
                return Err(
                    "Les liens symboliques ne sont pas pris en charge pour la comparaison".into(),
                );
            }
            let canonical = path
                .canonicalize()
                .map_err(|error| format!("Fichier exemple inaccessible : {error}"))?;
            if !canonical.is_file() || canonical.parent() != Some(project.path.as_path()) {
                return Err("Fichier exemple hors du projet".into());
            }
            fs::read(&canonical).map_err(|error| format!("Fichier exemple illisible : {error}"))?
        };
        template_keys(&data)
    })
    .await
    .map_err(|_| "Comparaison interrompue".to_string())?
}

#[tauri::command]
fn add_project(
    app: tauri::AppHandle,
    path: String,
    name: String,
    environment: String,
    section_id: Option<String>,
) -> Result<CatalogDto, String> {
    if !valid_environment(&environment) {
        return Err("Environnement invalide".into());
    }
    let canonical = PathBuf::from(path)
        .canonicalize()
        .map_err(|error| format!("Dossier de projet inaccessible : {error}"))?;
    if !canonical.is_dir() {
        return Err("Le projet sélectionné n'est pas un dossier".to_string());
    }
    let mut catalog = read_catalog(&app)?;
    if catalog_contains_project(&catalog, &canonical) {
        return catalog_to_dto(catalog);
    }
    let project = ProjectRecord {
        name: if name.trim().is_empty() {
            canonical
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| canonical.to_string_lossy().into_owned())
        } else {
            name.trim().into()
        },
        path: canonical,
        environment,
        ssh: None,
    };
    place_project(&mut catalog, project, section_id.as_deref())?;
    write_catalog(&app, &catalog)?;
    catalog_to_dto(catalog)
}

#[cfg(unix)]
#[tauri::command]
fn add_ssh_project(
    app: tauri::AppHandle,
    name: String,
    alias: String,
    user: Option<String>,
    port: Option<u16>,
    directory: String,
    environment: String,
    section_id: Option<String>,
) -> Result<CatalogDto, String> {
    let user = user.filter(|user| !user.trim().is_empty());
    if !remote::valid_alias(&alias)
        || user
            .as_deref()
            .is_some_and(|user| !remote::valid_user(user))
        || port == Some(0)
        || !remote::valid_path(&directory)
    {
        return Err("Hôte, utilisateur, port ou chemin distant invalide".into());
    }
    if !valid_environment(&environment) {
        return Err("Environnement invalide".into());
    }
    let name = name.trim();
    if name.is_empty() {
        return Err("Nom du projet requis".into());
    }
    remote::list(
        remote::Connection {
            alias: &alias,
            user: user.as_deref(),
            port,
        },
        &directory,
    )?;
    let mut catalog = read_catalog(&app)?;
    let project = ProjectRecord {
        name: name.into(),
        path: PathBuf::from(directory),
        environment: environment.clone(),
        ssh: Some(SshProject {
            alias,
            user,
            port,
            environment,
        }),
    };
    if catalog_contains_project(&catalog, Path::new(&project_identity(&project))) {
        return Err("Ce projet SSH est déjà enregistré".into());
    }
    place_project(&mut catalog, project, section_id.as_deref())?;
    write_catalog(&app, &catalog)?;
    catalog_to_dto(catalog)
}

#[tauri::command]
fn update_project(
    app: tauri::AppHandle,
    sessions: tauri::State<SessionStore>,
    project_id: String,
    name: String,
    path: String,
    environment: String,
    alias: Option<String>,
    user: Option<String>,
    port: Option<u16>,
) -> Result<CatalogDto, String> {
    if name.trim().is_empty() {
        return Err("Nom du projet requis".into());
    }
    if !valid_environment(&environment) {
        return Err("Environnement invalide".into());
    }
    let mut catalog = read_catalog(&app)?;
    let existing = find_project_mut(&mut catalog, &project_id)
        .ok_or("Projet introuvable dans le catalogue")?
        .clone();
    let updated = if existing.ssh.is_some() {
        #[cfg(unix)]
        {
            let alias = alias.ok_or("Hôte SSH requis")?;
            let user = user.filter(|value| !value.trim().is_empty());
            if !remote::valid_alias(&alias)
                || user
                    .as_deref()
                    .is_some_and(|value| !remote::valid_user(value))
                || port == Some(0)
                || !remote::valid_path(&path)
            {
                return Err("Hôte, utilisateur, port ou chemin distant invalide".into());
            }
            ProjectRecord {
                name: name.trim().into(),
                path: PathBuf::from(path),
                environment: environment.clone(),
                ssh: Some(SshProject {
                    alias,
                    user,
                    port,
                    environment,
                }),
            }
        }
        #[cfg(not(unix))]
        {
            return Err("Projet SSH indisponible sur cette plateforme".into());
        }
    } else {
        let canonical = PathBuf::from(path)
            .canonicalize()
            .map_err(|error| format!("Dossier de projet inaccessible : {error}"))?;
        if !canonical.is_dir() {
            return Err("Le projet sélectionné n'est pas un dossier".into());
        }
        ProjectRecord {
            name: name.trim().into(),
            path: canonical,
            environment,
            ssh: None,
        }
    };
    let updated_id = project_identity(&updated);
    if updated_id != project_id && catalog_contains_project(&catalog, Path::new(&updated_id)) {
        return Err("Ce projet est déjà enregistré".into());
    }
    *find_project_mut(&mut catalog, &project_id).ok_or("Projet introuvable")? = updated;
    write_catalog(&app, &catalog)?;
    sessions
        .0
        .lock()
        .map_err(|_| "État interne indisponible")?
        .clear();
    catalog_to_dto(catalog)
}

#[cfg(unix)]
fn find_remote_project<'a>(catalog: &'a Catalog, identity: &str) -> Option<&'a ProjectRecord> {
    fn in_sections<'a>(sections: &'a [SectionRecord], identity: &str) -> Option<&'a ProjectRecord> {
        sections.iter().find_map(|section| {
            section
                .projects
                .iter()
                .find(|project| project.ssh.is_some() && project_identity(project) == identity)
                .or_else(|| in_sections(&section.children, identity))
        })
    }
    catalog
        .unsectioned_projects
        .iter()
        .find(|project| project.ssh.is_some() && project_identity(project) == identity)
        .or_else(|| in_sections(&catalog.sections, identity))
}

#[tauri::command]
fn remove_project(app: tauri::AppHandle, path: String) -> Result<CatalogDto, String> {
    let requested = PathBuf::from(path);
    let mut catalog = read_catalog(&app)?;
    if take_project(&mut catalog, &requested).is_none() {
        return Err("Projet introuvable dans le catalogue".to_string());
    }
    write_catalog(&app, &catalog)?;
    catalog_to_dto(catalog)
}

#[tauri::command]
fn create_section(
    app: tauri::AppHandle,
    name: String,
    parent_section_id: Option<String>,
) -> Result<CatalogDto, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Le nom de la section ne peut pas être vide".to_string());
    }
    let mut catalog = read_catalog(&app)?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "Horloge système invalide".to_string())?
        .as_nanos();
    let section = SectionRecord {
        id: format!("section-{nonce}"),
        name: name.to_string(),
        expanded: false,
        projects: Vec::new(),
        children: Vec::new(),
    };
    match parent_section_id {
        Some(parent_id) => find_section_mut(&mut catalog.sections, &parent_id)
            .ok_or_else(|| "Section parente introuvable".to_string())?
            .children
            .push(section),
        None => catalog.sections.push(section),
    }
    write_catalog(&app, &catalog)?;
    catalog_to_dto(catalog)
}

#[tauri::command]
fn rename_section(
    app: tauri::AppHandle,
    section_id: String,
    name: String,
) -> Result<CatalogDto, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Le nom de la section ne peut pas être vide".to_string());
    }
    let mut catalog = read_catalog(&app)?;
    let section = find_section_mut(&mut catalog.sections, &section_id)
        .ok_or_else(|| "Section introuvable".to_string())?;
    section.name = name.to_string();
    write_catalog(&app, &catalog)?;
    catalog_to_dto(catalog)
}

#[tauri::command]
fn delete_section(app: tauri::AppHandle, section_id: String) -> Result<CatalogDto, String> {
    fn drain_projects(section: SectionRecord, destination: &mut Vec<ProjectRecord>) {
        destination.extend(section.projects);
        for child in section.children {
            drain_projects(child, destination);
        }
    }
    let mut catalog = read_catalog(&app)?;
    let section = take_section(&mut catalog.sections, &section_id)
        .ok_or_else(|| "Section introuvable".to_string())?;
    drain_projects(section, &mut catalog.unsectioned_projects);
    write_catalog(&app, &catalog)?;
    catalog_to_dto(catalog)
}

fn take_section(sections: &mut Vec<SectionRecord>, id: &str) -> Option<SectionRecord> {
    if let Some(index) = sections.iter().position(|section| section.id == id) {
        return Some(sections.remove(index));
    }
    for section in sections {
        if let Some(found) = take_section(&mut section.children, id) {
            return Some(found);
        }
    }
    None
}

#[tauri::command]
fn move_section(
    app: tauri::AppHandle,
    section_id: String,
    parent_section_id: Option<String>,
) -> Result<CatalogDto, String> {
    if parent_section_id.as_deref() == Some(section_id.as_str()) {
        return Err("Une section ne peut pas contenir elle-même".to_string());
    }
    let mut catalog = read_catalog(&app)?;
    let section = take_section(&mut catalog.sections, &section_id)
        .ok_or_else(|| "Section introuvable".to_string())?;
    match parent_section_id {
        Some(parent_id) => find_section_mut(&mut catalog.sections, &parent_id)
            .ok_or_else(|| {
                "Destination invalide : une section ne peut pas être déplacée dans sa descendance"
                    .to_string()
            })?
            .children
            .push(section),
        None => catalog.sections.push(section),
    }
    write_catalog(&app, &catalog)?;
    catalog_to_dto(catalog)
}

#[tauri::command]
fn move_project(
    app: tauri::AppHandle,
    path: String,
    section_id: Option<String>,
) -> Result<CatalogDto, String> {
    let mut catalog = read_catalog(&app)?;
    let project = take_project(&mut catalog, Path::new(&path))
        .ok_or_else(|| "Projet introuvable dans le catalogue".to_string())?;
    place_project(&mut catalog, project, section_id.as_deref())?;
    write_catalog(&app, &catalog)?;
    catalog_to_dto(catalog)
}

#[tauri::command]
fn open_env_file(
    app: tauri::AppHandle,
    path: String,
    sessions: tauri::State<SessionStore>,
) -> Result<OpenDocumentDto, String> {
    #[cfg(unix)]
    if path.starts_with("ssh://") {
        let (project_id, name) = path.rsplit_once('/').ok_or("Chemin SSH invalide")?;
        if !is_env_file(Path::new(name)) || name.contains('/') {
            return Err("Fichier SSH invalide".into());
        }
        let catalog = read_catalog(&app)?;
        let project = find_remote_project(&catalog, project_id)
            .ok_or("Ce fichier n'appartient pas à un projet SSH enregistré")?;
        let ssh = project.ssh.as_ref().ok_or("Projet SSH invalide")?;
        let remote_path = project.path.join(name).to_string_lossy().into_owned();
        let file = remote::read(
            remote::Connection {
                alias: &ssh.alias,
                user: ssh.user.as_deref(),
                port: ssh.port,
            },
            &remote_path,
        )?;
        let document = parser::parse_bytes(&file.data)?;
        let dto = document.to_dto();
        let session_id = NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed).to_string();
        sessions
            .0
            .lock()
            .map_err(|_| "État interne indisponible")?
            .insert(
                session_id.clone(),
                DocumentSession {
                    path: PathBuf::from(path),
                    document,
                    original_len: file.data.len() as u64,
                    original_modified: None,
                    original_hash: Sha256::digest(&file.data).into(),
                    locked: ssh.environment != "development",
                    protected: ssh.environment != "development",
                    mode: 0,
                    remote: Some(RemoteSession {
                        alias: ssh.alias.clone(),
                        user: ssh.user.clone(),
                        port: ssh.port,
                        path: remote_path,
                        fingerprint: file.fingerprint,
                        mode: file.mode,
                    }),
                },
            );
        return Ok(OpenDocumentDto {
            session_id,
            variables: dto.variables,
            diagnostics: dto.diagnostics,
            unknown_lines: dto.unknown_lines,
            duplicate_keys: dto.duplicate_keys,
        });
    }
    let requested = PathBuf::from(path);
    if requested.is_symlink() {
        return Err("Les liens symboliques sont ouverts en lecture seule par sécurité".to_string());
    }
    let canonical = requested
        .canonicalize()
        .map_err(|error| format!("Fichier inaccessible : {error}"))?;
    if !canonical.is_file() || !is_env_file(&canonical) {
        return Err("Le chemin sélectionné n'est pas un fichier .env pris en charge".to_string());
    }
    let catalog = read_catalog(&app)?;
    let project = find_local_project(&catalog, &canonical)
        .ok_or("Ce fichier n'appartient pas à un projet enregistré")?;
    let protected = project_environment(project) != "development";
    let metadata = fs::metadata(&canonical)
        .map_err(|error| format!("Impossible de lire les métadonnées : {error}"))?;
    let data =
        fs::read(&canonical).map_err(|error| format!("Impossible de lire le fichier : {error}"))?;
    let document = parser::parse_bytes(&data)?;
    let dto = document.to_dto();
    let session_id = NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed).to_string();
    let session = DocumentSession {
        path: canonical,
        document,
        original_len: metadata.len(),
        original_modified: metadata.modified().ok(),
        original_hash: Sha256::digest(&data).into(),
        locked: protected,
        protected,
        #[cfg(unix)]
        mode: {
            use std::os::unix::fs::PermissionsExt;
            metadata.permissions().mode()
        },
        #[cfg(unix)]
        remote: None,
    };
    sessions
        .0
        .lock()
        .map_err(|_| "État interne indisponible".to_string())?
        .insert(session_id.clone(), session);
    Ok(OpenDocumentDto {
        session_id,
        variables: dto.variables,
        diagnostics: dto.diagnostics,
        unknown_lines: dto.unknown_lines,
        duplicate_keys: dto.duplicate_keys,
    })
}

#[tauri::command]
fn unlock_project_file(
    session_id: String,
    sessions: tauri::State<SessionStore>,
) -> Result<(), String> {
    let mut store = sessions.0.lock().map_err(|_| "État interne indisponible")?;
    let session = store.get_mut(&session_id).ok_or("Session expirée")?;
    session.locked = false;
    Ok(())
}

fn apply_edits(document: &mut parser::Document, edits: &[VariableEdit]) -> Result<(), String> {
    let edit_by_line = edits
        .iter()
        .filter_map(|edit| edit.line_index.map(|index| (index, edit)))
        .collect::<HashMap<_, _>>();
    let retained = edit_by_line.keys().copied().collect::<HashSet<_>>();
    let mut original_assignments = document
        .lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| (line.kind == parser::LineKind::Assignment).then_some(index))
        .collect::<Vec<_>>();
    original_assignments.sort_by_key(|index| std::cmp::Reverse(*index));

    for line_index in original_assignments {
        if !retained.contains(&line_index) {
            document.remove_assignment(line_index)?;
            continue;
        }
        let edit = edit_by_line
            .get(&line_index)
            .ok_or_else(|| "Modification incohérente".to_string())?;
        let original_comment = document.comments_for(line_index);
        {
            let line = document
                .lines
                .get_mut(line_index)
                .ok_or_else(|| "Ligne modifiée introuvable".to_string())?;
            if line.key.as_deref() != Some(edit.key.as_str()) {
                line.rename(&edit.key)?;
            }
            if line.value() != Some(edit.value.as_str()) {
                line.set_value(&edit.value)?;
            }
        }
        if original_comment != edit.comment {
            document.set_comments(line_index, &edit.comment)?;
        }
    }
    for edit in edits.iter().filter(|edit| edit.line_index.is_none()) {
        document.add_assignment(&edit.key, &edit.value, &edit.comment)?;
    }
    Ok(())
}

fn current_file_matches(session: &DocumentSession) -> Result<bool, String> {
    let metadata = fs::metadata(&session.path)
        .map_err(|error| format!("Impossible de vérifier le fichier : {error}"))?;
    let data = fs::read(&session.path)
        .map_err(|error| format!("Impossible de vérifier le fichier : {error}"))?;
    Ok(metadata.len() == session.original_len
        && metadata.modified().ok() == session.original_modified
        && <[u8; 32]>::from(Sha256::digest(&data)) == session.original_hash)
}

#[cfg(unix)]
fn atomic_replace(session: &DocumentSession, data: &[u8]) -> Result<(), String> {
    use std::fs::OpenOptions;
    use std::os::unix::fs::OpenOptionsExt;

    let directory = session
        .path
        .parent()
        .ok_or_else(|| "Dossier du fichier invalide".to_string())?;
    let name = session
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(".env");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "Horloge système invalide".to_string())?
        .as_nanos();
    let temporary = directory.join(format!(".{name}.{nonce}.tmp"));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(session.mode)
            .open(&temporary)
            .map_err(|error| format!("Impossible de créer le fichier temporaire : {error}"))?;
        file.write_all(data)
            .map_err(|error| format!("Impossible d'écrire le fichier temporaire : {error}"))?;
        file.sync_all()
            .map_err(|error| format!("Impossible de synchroniser le fichier : {error}"))?;
        fs::rename(&temporary, &session.path)
            .map_err(|error| format!("Impossible de remplacer le fichier : {error}"))?;
        fs::File::open(directory)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| format!("Impossible de synchroniser le dossier : {error}"))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(windows)]
fn atomic_replace(session: &DocumentSession, data: &[u8]) -> Result<(), String> {
    use std::fs::OpenOptions;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::ReplaceFileW;

    let directory = session
        .path
        .parent()
        .ok_or_else(|| "Dossier du fichier invalide".to_string())?;
    let name = session
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(".env");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "Horloge système invalide".to_string())?
        .as_nanos();
    let temporary = directory.join(format!(".{name}.{nonce}.tmp"));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| format!("Impossible de créer le fichier temporaire : {error}"))?;
        file.write_all(data)
            .map_err(|error| format!("Impossible d'écrire le fichier temporaire : {error}"))?;
        file.sync_all()
            .map_err(|error| format!("Impossible de synchroniser le fichier : {error}"))?;
        drop(file);
        let destination = session
            .path
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        let replacement = temporary
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        let replaced = unsafe {
            ReplaceFileW(
                destination.as_ptr(),
                replacement.as_ptr(),
                std::ptr::null(),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if replaced == 0 {
            return Err(format!(
                "Impossible de remplacer le fichier : {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[tauri::command]
fn save_env_file(
    session_id: String,
    variables: Vec<VariableEdit>,
    sessions: tauri::State<SessionStore>,
) -> Result<parser::ParsedDocumentDto, String> {
    let mut store = sessions
        .0
        .lock()
        .map_err(|_| "État interne indisponible".to_string())?;
    let session = store
        .get_mut(&session_id)
        .ok_or_else(|| "Session expirée : rechargez le fichier".to_string())?;
    if session.locked {
        return Err("Édition de ce projet verrouillée".into());
    }
    #[cfg(unix)]
    if let Some(remote) = session.remote.as_mut() {
        let connection = remote::Connection {
            alias: &remote.alias,
            user: remote.user.as_deref(),
            port: remote.port,
        };
        let current = remote::read(connection, &remote.path)?;
        if current.fingerprint != remote.fingerprint || current.mode != remote.mode {
            return Err("Conflit : le fichier distant a changé".into());
        }
        let mut updated = session.document.clone();
        apply_edits(&mut updated, &variables)?;
        let data = updated.to_bytes();
        remote::save(
            connection,
            &remote.path,
            &remote.fingerprint,
            &remote.mode,
            &data,
        )?;
        session.document = updated;
        session.original_len = data.len() as u64;
        session.original_hash = Sha256::digest(&data).into();
        remote.fingerprint = format!("{:x}", Sha256::digest(&data));
        session.locked = session.protected;
        return Ok(session.document.to_dto());
    }
    if session.path.is_symlink() {
        return Err("La sauvegarde via un lien symbolique est désactivée".to_string());
    }
    if !current_file_matches(session)? {
        return Err("Conflit : le fichier a été modifié sur le disque".to_string());
    }
    let mut updated = session.document.clone();
    apply_edits(&mut updated, &variables)?;
    let data = updated.to_bytes();
    atomic_replace(session, &data)?;
    let metadata = fs::metadata(&session.path)
        .map_err(|error| format!("Sauvegarde effectuée mais métadonnées illisibles : {error}"))?;
    session.document = updated;
    session.original_len = metadata.len();
    session.original_modified = metadata.modified().ok();
    session.original_hash = Sha256::digest(&data).into();
    session.locked = session.protected;
    Ok(session.document.to_dto())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(SessionStore::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .invoke_handler(tauri::generate_handler![
            backend_status,
            get_catalog,
            discover_project_files,
            get_env_template_keys,
            add_project,
            update_project,
            #[cfg(unix)]
            add_ssh_project,
            remove_project,
            create_section,
            rename_section,
            delete_section,
            move_section,
            move_project,
            open_env_file,
            unlock_project_file,
            save_env_file
        ])
        .run(tauri::generate_context!())
        .expect("failed to run EnvEdit");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn temporary_session(contents: &[u8]) -> (PathBuf, DocumentSession) {
        use std::os::unix::fs::PermissionsExt;
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("envedit-rust-test-{nonce}"));
        fs::create_dir(&directory).unwrap();
        let path = directory.join(".env");
        fs::write(&path, contents).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        let metadata = fs::metadata(&path).unwrap();
        let session = DocumentSession {
            path: path.clone(),
            document: parser::parse_bytes(contents).unwrap(),
            original_len: metadata.len(),
            original_modified: metadata.modified().ok(),
            original_hash: Sha256::digest(contents).into(),
            locked: false,
            protected: false,
            mode: metadata.permissions().mode(),
            remote: None,
        };
        (path, session)
    }

    #[test]
    fn status_does_not_expose_environment_data() {
        assert_eq!(backend_status(), "EnvEdit portable backend ready");
    }

    #[test]
    fn template_comparison_returns_only_keys_and_diagnostics() {
        let result = template_keys(b"# Cle privee\nAPI_KEY=secret-fixture\n# URL publique\n# A renseigner\nAPP_URL=https://example.test\nAPI_KEY=other\nUNKNOWN LINE\n").unwrap();
        assert_eq!(result.keys, vec!["API_KEY", "APP_URL"]);
        assert_eq!(
            result.comments.get("APP_URL").map(String::as_str),
            Some("URL publique\nA renseigner")
        );
        assert!(!result.comments.contains_key("API_KEY"));
        assert_eq!(result.unknown_lines, 1);
        assert_eq!(result.duplicate_keys, vec!["API_KEY"]);
        let serialized = serde_json::to_string(&result).unwrap();
        assert!(!serialized.contains("secret-fixture"));
        assert!(!serialized.contains("example.test"));
        assert!(valid_template_name(".env.example"));
        assert!(valid_template_name(".env.exemple"));
        assert!(!valid_template_name("../.env"));
    }

    #[test]
    fn adding_from_example_keeps_existing_bytes_and_writes_comment_only_for_new_key() {
        let source = b"EXISTING=one\r\n";
        let mut document = parser::parse_bytes(source).unwrap();
        apply_edits(
            &mut document,
            &[
                VariableEdit {
                    line_index: Some(0),
                    key: "EXISTING".into(),
                    value: "one".into(),
                    comment: String::new(),
                },
                VariableEdit {
                    line_index: None,
                    key: "DB_HOST".into(),
                    value: String::new(),
                    comment: "Serveur\nA renseigner".into(),
                },
            ],
        )
        .unwrap();
        assert_eq!(
            document.to_bytes(),
            b"EXISTING=one\r\n# Serveur\r\n# A renseigner\r\nDB_HOST="
        );
    }

    #[test]
    fn older_local_catalog_defaults_to_development() {
        let project: ProjectRecord =
            serde_json::from_str(r#"{"name":"fixture","path":"/fixture"}"#).unwrap();
        assert_eq!(project_environment(&project), "development");
    }

    #[test]
    fn environment_and_project_changes_stay_in_catalog_only() {
        let directory = std::env::temp_dir().join(format!(
            "envedit-settings-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&directory).unwrap();
        fs::write(directory.join(".env"), b"SECRET=fixture\n").unwrap();
        let mut catalog = Catalog {
            sections: Vec::new(),
            unsectioned_projects: vec![ProjectRecord {
                name: "ancien".into(),
                path: directory.clone(),
                environment: "production".into(),
                ssh: None,
            }],
        };
        let project = find_local_project(&catalog, &directory.join(".env")).unwrap();
        assert_eq!(project_environment(project), "production");
        find_project_mut(&mut catalog, &directory.to_string_lossy())
            .unwrap()
            .name = "nouveau".into();
        assert_eq!(catalog.unsectioned_projects[0].name, "nouveau");
        assert_eq!(
            fs::read(directory.join(".env")).unwrap(),
            b"SECRET=fixture\n"
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn catalog_does_not_connect_to_ssh_projects() {
        let project = ProjectRecord {
            name: "Fixture distant".into(),
            path: PathBuf::from("/fixture/inaccessible"),
            environment: "production".into(),
            ssh: Some(SshProject {
                alias: "hote-inexistant-fixture".into(),
                user: Some("deploy".into()),
                port: None,
                environment: "production".into(),
            }),
        };
        let dto = catalog_to_dto(Catalog {
            sections: Vec::new(),
            unsectioned_projects: vec![project],
        })
        .unwrap();
        assert_eq!(dto.unsectioned_projects.len(), 1);
        assert_eq!(dto.unsectioned_projects[0].environment, "production");
        assert!(dto.unsectioned_projects[0].files.is_empty());
        assert!(dto.unsectioned_projects[0].connection_error.is_none());
    }

    #[test]
    fn dotenv_file_names_are_recognized_conservatively() {
        assert!(is_env_file(Path::new(".env")));
        assert!(is_env_file(Path::new(".env.local")));
        assert!(is_env_file(Path::new(".env.production")));
        assert!(!is_env_file(Path::new("env")));
        assert!(!is_env_file(Path::new("example.env")));
    }

    #[test]
    fn removing_project_record_never_touches_its_directory() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("envedit-removal-{nonce}"));
        fs::create_dir(&directory).unwrap();
        fs::write(directory.join(".env"), b"SAFE=fixture\n").unwrap();
        let mut catalog = Catalog {
            sections: Vec::new(),
            unsectioned_projects: vec![ProjectRecord {
                name: "sentinel".into(),
                path: directory.clone(),
                environment: default_environment(),
                ssh: None,
            }],
        };
        assert!(take_project(&mut catalog, &directory).is_some());
        assert!(catalog.unsectioned_projects.is_empty());
        assert_eq!(fs::read(directory.join(".env")).unwrap(), b"SAFE=fixture\n");
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn deleting_a_section_can_preserve_its_projects_unsectioned() {
        let project = ProjectRecord {
            name: "fixture".into(),
            path: PathBuf::from("/fixture"),
            environment: default_environment(),
            ssh: None,
        };
        let mut catalog = Catalog {
            sections: vec![SectionRecord {
                id: "work".into(),
                name: "Travail".into(),
                expanded: true,
                projects: vec![project],
                children: Vec::new(),
            }],
            unsectioned_projects: Vec::new(),
        };
        let section = catalog.sections.remove(0);
        catalog.unsectioned_projects.extend(section.projects);
        assert!(catalog.sections.is_empty());
        assert_eq!(catalog.unsectioned_projects[0].name, "fixture");
    }

    #[test]
    fn nested_sections_are_found_and_projects_can_be_moved() {
        let project = ProjectRecord {
            name: "nested-project".into(),
            path: PathBuf::from("/fixture/nested"),
            environment: default_environment(),
            ssh: None,
        };
        let child = SectionRecord {
            id: "child".into(),
            name: "Enfant".into(),
            expanded: true,
            projects: vec![project],
            children: Vec::new(),
        };
        let mut catalog = Catalog {
            sections: vec![SectionRecord {
                id: "root".into(),
                name: "Racine".into(),
                expanded: true,
                projects: Vec::new(),
                children: vec![child],
            }],
            unsectioned_projects: Vec::new(),
        };

        assert!(catalog_contains_project(
            &catalog,
            Path::new("/fixture/nested")
        ));
        assert!(find_local_project(&catalog, Path::new("/fixture/nested/.env")).is_some());
        assert!(find_local_project(&catalog, Path::new("/fixture/other/.env")).is_none());
        let moved = take_project(&mut catalog, Path::new("/fixture/nested")).unwrap();
        place_project(&mut catalog, moved, Some("root")).unwrap();
        assert_eq!(catalog.sections[0].projects.len(), 1);
        assert!(catalog.sections[0].children[0].projects.is_empty());
    }

    #[test]
    fn edits_are_targeted_and_keep_unknown_lines() {
        let mut document = parser::parse_bytes(b"# note\nA=one\nUNKNOWN LINE\nB='two'\n").unwrap();
        let edits = vec![
            VariableEdit {
                line_index: Some(1),
                key: "RENAMED".into(),
                value: "changed".into(),
                comment: "updated".into(),
            },
            VariableEdit {
                line_index: Some(3),
                key: "B".into(),
                value: "kept".into(),
                comment: String::new(),
            },
            VariableEdit {
                line_index: None,
                key: "C".into(),
                value: "new".into(),
                comment: "added".into(),
            },
        ];
        apply_edits(&mut document, &edits).unwrap();
        assert_eq!(
            document.to_bytes(),
            b"# updated\nRENAMED=changed\nUNKNOWN LINE\nB='kept'\n# added\nC=new"
        );
    }

    #[test]
    fn omitted_assignment_is_removed_with_its_comment() {
        let mut document = parser::parse_bytes(b"# remove me\nA=1\nB=2\n").unwrap();
        let edits = vec![VariableEdit {
            line_index: Some(2),
            key: "B".into(),
            value: "2".into(),
            comment: String::new(),
        }];
        apply_edits(&mut document, &edits).unwrap();
        assert_eq!(document.to_bytes(), b"B=2\n");
    }

    #[test]
    fn unchanged_edit_payload_keeps_nonstandard_comment_bytes() {
        let original = b"  #comment without normalized spacing\n  export A = one  # inline\n";
        let mut document = parser::parse_bytes(original).unwrap();
        let edits = vec![VariableEdit {
            line_index: Some(1),
            key: "A".into(),
            value: "one".into(),
            comment: "comment without normalized spacing".into(),
        }];
        apply_edits(&mut document, &edits).unwrap();
        assert_eq!(document.to_bytes(), original);
    }

    #[cfg(unix)]
    #[test]
    fn atomic_replace_preserves_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let (path, session) = temporary_session(b"A=one\n");
        atomic_replace(&session, b"A=changed\n").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"A=changed\n");
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o640
        );
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn external_change_is_detected_by_hash() {
        let (path, session) = temporary_session(b"A=one\n");
        fs::write(&path, b"A=two\n").unwrap();
        assert!(!current_file_matches(&session).unwrap());
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
