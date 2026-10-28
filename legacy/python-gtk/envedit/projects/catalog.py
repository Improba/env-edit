from __future__ import annotations

from dataclasses import dataclass
import json
import os
from pathlib import Path
import tempfile


IGNORED_DIRECTORIES = {".git", "node_modules", ".venv", "venv", "dist", "build"}


@dataclass(frozen=True)
class Project:
    name: str
    path: Path


def _is_env_file(path: Path) -> bool:
    return path.name == ".env" or path.name.startswith(".env.")


def discover_env_files(project: Path, recursive: bool = False) -> list[Path]:
    project = Path(project)
    if not recursive:
        try:
            return sorted((p for p in project.iterdir() if p.is_file() and _is_env_file(p)),
                          key=lambda p: p.name)
        except OSError:
            return []
    result: list[Path] = []
    for root, directories, files in os.walk(project):
        directories[:] = sorted(d for d in directories if d not in IGNORED_DIRECTORIES)
        for filename in files:
            path = Path(root) / filename
            if _is_env_file(path):
                result.append(path)
    return sorted(result, key=lambda p: str(p.relative_to(project)))


class ProjectCatalog:
    def __init__(self, config_path: Path | None = None) -> None:
        if config_path is None:
            base = Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config"))
            config_path = base / "envedit" / "projects.json"
        self.config_path = config_path
        self.projects: list[Project] = []

    def load(self) -> None:
        if not self.config_path.exists():
            self.projects = []
            return
        try:
            payload = json.loads(self.config_path.read_text(encoding="utf-8"))
            self.projects = [Project(str(item["name"]), Path(item["path"]))
                             for item in payload.get("projects", [])
                             if isinstance(item, dict) and "name" in item and "path" in item]
        except (OSError, json.JSONDecodeError, TypeError) as exc:
            raise ValueError(f"Catalogue de projets illisible : {exc}") from exc

    def save(self) -> None:
        self.config_path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
        payload = {"projects": [{"name": p.name, "path": str(p.path)} for p in self.projects]}
        fd, temporary = tempfile.mkstemp(prefix=".projects.", suffix=".tmp", dir=self.config_path.parent)
        try:
            os.fchmod(fd, 0o600)
            with os.fdopen(fd, "w", encoding="utf-8") as stream:
                json.dump(payload, stream, ensure_ascii=False, indent=2)
                stream.write("\n")
                stream.flush()
                os.fsync(stream.fileno())
            os.replace(temporary, self.config_path)
        finally:
            if os.path.exists(temporary):
                os.unlink(temporary)

    def add(self, path: Path) -> Project:
        normalized = Path(path).expanduser().resolve()
        if not normalized.is_dir():
            raise ValueError("Le projet doit être un dossier existant")
        existing = next((p for p in self.projects if p.path == normalized), None)
        if existing:
            return existing
        project = Project(normalized.name or str(normalized), normalized)
        self.projects.append(project)
        self.save()
        return project

    def remove(self, project: Project) -> None:
        self.projects.remove(project)
        self.save()
