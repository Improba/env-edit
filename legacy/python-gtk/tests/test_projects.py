from envedit.projects import ProjectCatalog, discover_env_files


def test_catalog_persists_and_remove_does_not_delete_project(tmp_path):
    folder = tmp_path / "project"
    folder.mkdir()
    env_file = folder / ".env"
    env_file.write_text("SAFE=fixture\n")
    config = tmp_path / "config" / "projects.json"
    catalog = ProjectCatalog(config)
    project = catalog.add(folder)
    restored = ProjectCatalog(config)
    restored.load()
    assert restored.projects == [project]
    restored.remove(restored.projects[0])
    assert env_file.read_text() == "SAFE=fixture\n"


def test_discovery_default_and_recursive_ignores_large_directories(tmp_path):
    (tmp_path / ".env").write_text("")
    (tmp_path / ".env.local").write_text("")
    nested = tmp_path / "nested"
    nested.mkdir()
    (nested / ".env.production").write_text("")
    ignored = tmp_path / "node_modules"
    ignored.mkdir()
    (ignored / ".env.hidden").write_text("")
    assert [p.name for p in discover_env_files(tmp_path)] == [".env", ".env.local"]
    assert [p.name for p in discover_env_files(tmp_path, recursive=True)] == [
        ".env", ".env.local", ".env.production"
    ]
