# Préparation de la version 1.0.0

Historique de préparation : la branche `release/1.0.0` a été fusionnée dans `main`.
Le processus courant prépare désormais les releases directement sur `main` ; cette
branche de validation reste un cas ponctuel. Aucun tag ni release publiée à ce stade.

## Vérifications effectuées localement

- Versions synchronisées dans `package.json`, `package-lock.json`, `Cargo.toml`,
  `Cargo.lock` et `tauri.conf.json` (`v1.0.0`).
- Vitest : 18 tests réussis ; Rust : 22 tests réussis.
- `npm run tauri build -- --bundles deb` : paquet
  `src-tauri/target/release/bundle/deb/EnvEdit_1.0.0_amd64.deb` produit.
- Métadonnées du paquet contrôlées avec `dpkg-deb --info` : paquet `env-edit`,
  version `1.0.0`, architecture `amd64`, dépendances GTK/WebKitGTK annoncées.
- Contenu extrait dans un dossier temporaire ; le binaire démarre avec
  `XDG_CONFIG_HOME` et `XDG_CACHE_HOME` isolés et reste actif jusqu'au délai de
  huit secondes du test de lancement. Une fixture `.env` synthétique a été copiée
  dans le dossier temporaire, sans être ouverte par l'application pendant ce test.

## Validation manuelle encore nécessaire

Le propriétaire du projet a installé localement le `.deb` 1.0.0 et indique que
l'application « semble fonctionnelle ». Ce retour confirme l'installation et un
premier essai utilisateur, sans constituer une validation exhaustive des critères
ci-dessous. L'agent n'a pas exécuté lui-même l'installation système : `sudo`
demande le mot de passe de l'utilisateur. Pour réinstaller depuis la racine du
dépôt :

```bash
sudo apt install ./src-tauri/target/release/bundle/deb/EnvEdit_1.0.0_amd64.deb
```

Tester ensuite l'application installée avec **un nouveau dossier temporaire et une
fixture sans secret réel** : ajout du projet, ouverture du `.env`, édition inline,
commentaire en tooltip, masquage/copie d'une valeur sensible, sauvegarde ciblée,
avertissement de valeur vide, fermeture avec modifications non sauvegardées et
comparaison avec `.env.example`. Ne pas utiliser un projet de production pour
valider cette release.

Les vérifications SSH sur serveur de test, les critères manuels encore ouverts
dans `ACCEPTANCE.md` et la parité Windows/macOS restent à traiter ou à déclarer
explicitement comme limites de la release. Le test historique Python/GTK n'a
pas été exécuté localement : `pytest` est absent de cet environnement.
