# Prototype historique Python/GTK

Cette arborescence conserve la première implémentation d'EnvEdit et son corpus de tests.
Elle sert d'oracle de comportement pour le parseur round-trip ; elle n'est plus
l'application distribuée et ne doit pas devenir une dépendance du cœur Tauri/Rust.

```bash
cd legacy/python-gtk
python3 -m pip install --user -e '.[test]'
python3 -m pytest
./run-dev.sh
```

Les tests ne manipulent que `tests/fixtures/` et des dossiers temporaires.
