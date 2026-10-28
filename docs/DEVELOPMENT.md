# Développement d'EnvEdit

Version portable d'EnvEdit fondée sur Tauri 2, Vue 3, Quasar et Rust. Elle permet d'ajouter
des projets persistants, de découvrir leurs fichiers `.env*`, de modifier directement les
variables et de sauvegarder les fichiers existants sans réécrire les lignes non modifiées.

```bash
npm install
npm run test
npm run build
npm run tauri dev
```

Le cœur Rust charge désormais les `.env`, effectue le round-trip ligne par ligne et expose
les variables et diagnostics à Quasar. Sous Linux et macOS, les modifications ciblées sont
enregistrées par remplacement atomique après contrôle des métadonnées et de l'empreinte
SHA-256. Le chemin Windows utilise désormais l'API native `ReplaceFileW`, mais doit encore
être validé par un build et des tests exécutés sous Windows.

La validation Windows/macOS est différée. Le développement courant cible une livraison
Linux sans supprimer les chemins portables ni considérer ceux-ci comme validés.

## État vérifié

- Build TypeScript/Vite réussi.
- 4 tests Vitest (modèle et composants) et 12 tests Rust réussis, y compris la fixture
  Python historique octet-pour-octet, le conflit
  externe, les permissions et le remplacement atomique dans un dossier temporaire.
- Audit npm sans vulnérabilité connue.
- Démarrage de la fenêtre Tauri vérifié sous Linux.

L'ajout, l'actualisation, le chargement et la sauvegarde sont actifs. `Ctrl+S` fonctionne et
un changement de fichier, un rafraîchissement ou la fermeture avec des modifications demande
confirmation. Un projet peut être retiré du catalogue sans supprimer aucun fichier. En cas
de conflit externe, l'utilisateur peut conserver ses modifications ou recharger le disque.
Le catalogue ne contient que les noms et chemins de projets, jamais de valeur issue d'un
`.env`.

## Construire les paquets Linux

Les dépendances de compilation Tauri/WebKitGTK doivent être installées, puis :

```bash
npm install
npm run test
npm run build
npm run tauri build -- --bundles deb,appimage
```

Les artefacts sont produits dans `src-tauri/target/release/bundle/deb/` et
`src-tauri/target/release/bundle/appimage/`.

```bash
sudo apt install ./src-tauri/target/release/bundle/deb/EnvEdit_0.1.0_amd64.deb
chmod +x src-tauri/target/release/bundle/appimage/EnvEdit_0.1.0_amd64.AppImage
./src-tauri/target/release/bundle/appimage/EnvEdit_0.1.0_amd64.AppImage
```

La validation et la distribution Windows/macOS restent volontairement différées.
