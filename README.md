# EnvEdit

EnvEdit est une application de bureau qui permet d'ouvrir et d'éditer **directement** des
fichiers `.env` répartis dans plusieurs projets. Sa priorité est la lisibilité et la
rapidité : sélectionner un projet à gauche, modifier une valeur à droite, enregistrer avec
Ctrl+S. Pas de snapshots, pas de base de secrets, pas de fenêtre modale pour modifier une
valeur.

## Organisation du dépôt

```text
src/                 interface Vue 3 / Quasar et tests Vitest
src-tauri/           shell Tauri, cœur Rust, capacités et packaging
public/              ressources statiques du frontend
legacy/python-gtk/   prototype historique et tests Python de conformité
docs/                documentation de développement complémentaire
```

Les documents de référence sont `SPEC.md` (produit et UX), `TECHNICAL.md` (architecture et
format `.env`), `ACCEPTANCE.md` (tests) et `AGENTS.md` (instructions persistantes).

## Cible

- Application principale : Tauri 2, Vue 3/Quasar et cœur Rust.
- Livraison actuellement validée : Ubuntu Linux en `.deb` et AppImage.
- Cibles futures : Windows et macOS, dont la validation est différée.
- Fonctionnement local et hors ligne.
- Licence proposée : MIT (à confirmer par le propriétaire).

## Architecture

Vue/Quasar fournit l'interface, tandis que Rust assure le parsing round-trip, la découverte
des projets, la détection des conflits et la sauvegarde atomique. Les deux couches ne
communiquent que par des commandes Tauri étroites. Le prototype Python/GTK est archivé sous
`legacy/python-gtk/` et n'est jamais embarqué dans l'application distribuée.

## Fonctionnalités disponibles

Catalogue persistant avec sections de projets, découverte des `.env*`, édition inline, recherche,
commentaires en tooltip et popover, masquage des clés sensibles,
copie rapide des valeurs, ajout/suppression/renommage, diagnostics, sauvegarde atomique et
détection des conflits. Le numéro de version figure dans l'en-tête.

Chaque projet local ou SSH possède un environnement (développement, préproduction ou
production). À l'ajout local, choisir le dossier, le nom et l'environnement dans la même
modale. Le menu « … » d'un projet permet ensuite de modifier son nom, son dossier ou ses
paramètres de connexion et son environnement, sans changer son type local/SSH ni sa section.
Un changement de paramètres ferme le fichier ouvert de ce projet et invalide les sessions
d'édition, afin que la nouvelle protection s'applique à la prochaine ouverture.
Préproduction et production s'ouvrent en lecture seule, y compris en local : l'édition
requiert une activation explicite par fichier, une confirmation résumant les clés, puis se
reverrouille après sauvegarde. La sauvegarde locale reste atomique mais ne crée pas de
copie de secours automatique ; la sauvegarde SSH en crée une sur le serveur.

## Comparaison avec un fichier d'exemple

Lorsqu'un projet contient `.env.example` ou `.env.exemple`, ouvrir un `.env` ou un autre
`.env.*` affiche un résumé des clés manquantes et supplémentaires. Le panneau « Voir la
comparaison » détaille les écarts. Si les deux exemples existent, `.env.example` est
choisi d'abord et le fichier de référence peut être changé dans le panneau. Seuls les noms
de clés sont comparés ; aucune valeur de l'exemple n'est envoyée à l'interface par la
commande de comparaison. Les clés manquantes peuvent être ajoutées séparément ou toutes
ensemble, avec le commentaire associé dans l'exemple et une valeur vide à compléter.
Si la clé est dupliquée dans l'exemple, aucun de ses commentaires n'est copié pour éviter
un choix ambigu. Ces ajouts restent des modifications non
enregistrées jusqu'à l'action « Enregistrer » et suivent les protections préprod/prod.
Les clés supplémentaires sont seulement signalées : EnvEdit ne les supprime pas et ne
copie rien automatiquement vers l'exemple. Les lignes non reconnues et clés dupliquées
dans l'exemple rendent la comparaison partielle et sont signalées.

Les valeurs vides (ou composées uniquement d'espaces) sont indiquées sous leur champ.
Lorsqu'une clé vient d'être ajoutée sans valeur, ou qu'une valeur renseignée a été vidée,
EnvEdit demande confirmation avant de sauvegarder. Les valeurs déjà vides à l'ouverture
ne provoquent pas d'alerte répétée si elles restent intactes ; si une valeur vide est
renseignée puis de nouveau vidée pendant la même édition, l'avertissement réapparaît.
La confirmation cite uniquement les noms des clés,
jamais leurs valeurs ; une variable optionnelle peut être enregistrée vide volontairement.
Pour les projets préproduction/production, cet avertissement est inclus dans la
confirmation de sauvegarde existante, sans deuxième boîte de dialogue.

## Projets SSH (Linux)

Le bouton « Ajouter un projet » ouvre une modale avec les onglets **Local** et **SSH**.
L'onglet Local permet de choisir un dossier ; l'onglet SSH accepte une adresse IP, un nom DNS ou un alias de
`~/.ssh/config`. Renseigner le même utilisateur SSH et, si nécessaire, le même port que
dans le client SSH habituel. Sans utilisateur ni port explicite, OpenSSH utilise sa
configuration habituelle ; pour une IP non configurée, l'utilisateur est le compte local.
La clé `~/.ssh/id_rsa` est essayée automatiquement par OpenSSH, sans import dans EnvEdit.
Vérifier d'abord la clé d'hôte avec `ssh utilisateur@hote` (ou `ssh mon-alias`).
EnvEdit ne stocke ni clé privée ni mot de passe.
OpenSSH et les outils GNU `find`, `stat`, `sha256sum`, `mktemp`, `cp`, `mv` doivent être
disponibles sur le serveur Linux.

Le catalogue et l'interface se chargent sans contacter les serveurs SSH. Déplier un projet
distant lance sa découverte et affiche un indicateur de connexion. Une erreur reste limitée
à ce projet ; replier puis déplier permet de réessayer. Les projets préproduction et
production s'ouvrent en lecture seule. L'édition doit être
activée pour chaque fichier ; un récapitulatif des clés et de la cible précède la
sauvegarde. La session se reverrouille après l'écriture.

La sauvegarde vérifie le SHA-256 distant, écrit un temporaire dans le même dossier,
crée une copie de l'ancienne version sous `.NOM.envedit-backup-DATE-PID`, puis remplace
le fichier. Elle refuse les liens symboliques et les fichiers dont l'utilisateur SSH
n'est pas propriétaire ou dont le groupe ne pourrait être conservé. Les copies contiennent
des secrets : prévoir leur durée de conservation côté serveur. Une modification concurrente
juste après la dernière vérification reste possible sans verrou partagé. Tester sur un
serveur de test avant la production.

## Développement et tests

L'ajout persistant de projets, la découverte des `.env*`, le parseur Rust et la sauvegarde
sûre sont fonctionnels sous Linux. Le remplacement atomique Windows est implémenté via
`ReplaceFileW`, mais reste à compiler et valider sur un hôte Windows réel.

La validation et la distribution Windows/macOS sont explicitement différées. La priorité
actuelle est une version Linux fiable et publiable, sans supprimer les chemins portables.

```bash
npm install
npm run test
npm run build
npm run tauri dev
```

Le développement Tauri sous Linux nécessite aussi les bibliothèques WebKitGTK indiquées par
la documentation Tauri. Le frontend peut être prévisualisé sans shell natif avec
`npm run dev`.

Pour produire les paquets Linux :

```bash
npm run tauri build -- --bundles deb,appimage
```

Ils sont écrits sous `src-tauri/target/release/bundle/`. Le paquet Debian s'installe
avec `sudo apt install ./EnvEdit_1.0.0_amd64.deb` depuis son dossier. L'AppImage se lance
après `chmod +x EnvEdit_1.0.0_amd64.AppImage`. Ces paquets ne contiennent pas le backend
Python. Les projets locaux restent hors ligne ; seuls les projets SSH utilisent une
connexion configurée explicitement par l'utilisateur.

## Préparer une release GitHub

La préparation se fait normalement **sur `main`, en local**. Mettre d'abord `main` à
jour, choisir la version cible et aligner `package.json`, `package-lock.json`,
`src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` et `src-tauri/tauri.conf.json`.
Exécuter les tests et construire/essayer le `.deb` sur des fixtures sans secret réel.
Faire une revue de sécurité et de qualité si nécessaire. Si un contrôle échoue,
corriger sur `main` et recommencer ; une branche `release/*` reste possible pour
une stabilisation exceptionnelle, mais n'est pas imposée.

Après le push, attendre que `Validate main` soit vert pour le **SHA exact** retenu.
Ce workflow exécute les tests frontend et Rust sous Linux et construit le `.deb` ;
sur `main` seulement, il construit également les installateurs Windows et macOS
expérimentaux. Une PR éventuelle ne lance que le job Linux. Le workflow du tag
refuse de publier si le même commit n'a pas réussi sur `main`.

Créer alors le tag annoté `vX.Y.Z` sur ce commit. `Publish release draft` vérifie
les versions et reconstruit les paquets pour ce même SHA, sans répéter les suites
de tests déjà validées. Les fichiers sont joints à une **release en brouillon** :
examiner les installateurs avant de la publier manuellement. Ne pas déplacer un
tag déjà publié ; corriger sur une nouvelle version si nécessaire.

Exemple pour `1.0.0` :

```bash
git switch main
git pull --ff-only origin main
# Aligner les cinq fichiers de version, puis effectuer la revue locale.
node scripts/check-release-version.mjs v1.0.0
npm ci
npm test
cargo test --locked --manifest-path src-tauri/Cargo.toml
npm run tauri build -- --bundles deb
git diff --check
git add package.json package-lock.json src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json
git commit -m "Prepare EnvEdit 1.0.0"
git push origin main
# Attendre le succès de Validate main pour le commit courant.
git tag -a v1.0.0 -m "EnvEdit 1.0.0"
git push origin v1.0.0
```

La CI produit des builds Windows/macOS **expérimentaux** ; elle ne valide pas encore
les parcours manuels et la parité de sauvegarde exigés par `ACCEPTANCE.md`. Leur
distribution comme versions prises en charge reste différée. Les installateurs ne
sont pas encore signés ni, pour macOS, notarisés. Aucun déploiement n'est déclenché
par une PR ni par un simple push sur `main`.

La configuration réside dans `$XDG_CONFIG_HOME/envedit/catalog.json` (ou
`~/.config/envedit/catalog.json`). Elle contient les sections, leurs noms et les chemins des
projets, jamais les valeurs des variables. L'ancien `projects.json` n'est pas migré et est
ignoré. Tous les tests utilisent la fixture sous `legacy/python-gtk/tests/fixtures/` ou des
dossiers temporaires : aucun `.env` réel de l'utilisateur n'est lu ou modifié.

## Garanties et limites du format

Le fichier est conservé ligne par ligne avec son terminateur exact. Une édition de valeur
ne remplace que sa tranche de texte ; ordre, blancs, commentaires, doublons, guillemets,
LF/CRLF et absence de saut final restent intacts. Les fichiers sont limités à UTF-8.

Sont reconnus : `KEY=value`, `export KEY = value`, valeurs non citées, guillemets simples
ou doubles, `#` dans une valeur citée et commentaires finaux séparés par un blanc. Les
commentaires immédiatement contigus au-dessus d'une variable lui sont associés ; une ligne
blanche coupe l'association.

Les citations non fermées, continuations et valeurs multilignes, lignes ne correspondant
pas à ces formes, ainsi que les encodages non UTF-8 ne sont pas interprétés. Les lignes
inconnues sont signalées et préservées intégralement. Une valeur contenant le délimiteur de
citation existant est refusée plutôt que rééchappée implicitement.

La sauvegarde vérifie taille, date de modification et SHA-256, puis écrit dans un fichier
temporaire du même dossier, synchronise et remplace atomiquement tout en conservant les
permissions. Les liens symboliques sont refusés afin d'éviter un remplacement surprenant.
