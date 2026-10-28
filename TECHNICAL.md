# Notes techniques et contraintes de sûreté

## Stack actuelle

- Tauri 2 pour le shell desktop, les capacités système et le packaging.
- Vue 3 + Quasar pour le frontend, avec TypeScript en mode strict.
- Rust pour le parseur round-trip, le stockage atomique, la détection de conflits, la
  découverte et le catalogue de projets.
- Vitest pour les composants et la logique frontend ; tests Rust unitaires et d'intégration
  pour le cœur. Les fixtures historiques sous `legacy/python-gtk/` restent le corpus de
  conformité commun.
- Distribution Linux en `.deb` et AppImage. Windows et macOS restent des cibles futures à
  compiler et valider explicitement.

Le runtime Python ne doit pas être embarqué dans le binaire Tauri. L'implémentation
Python/GTK archivée reste un oracle de comportement et non une dépendance d'exécution.

## Règle principale : round-trip non destructif

**Ne pas** utiliser un parseur `dotenv` qui reconstruit intégralement le fichier à partir d'un dictionnaire. Cela perdrait commentaires, doublons, encodage ou formatage.

Construire une représentation des lignes (texte exact + terminateur exact) avec un modèle d'entrées pour les affectations reconnues, commentaires, blancs et lignes inconnues. Pour une valeur modifiée, changer uniquement la portion de valeur de la ligne cible, en conservant tout le reste lorsqu'il est possible de le faire de manière non ambiguë. Pour un commentaire modifié, modifier seulement son bloc associé. Les lignes non modifiées doivent rester **octet pour octet** identiques.

Format reconnu a minima : `KEY=value`, `export KEY=value`, valeurs non quotées, simples/doubles quotes, espaces avant/après `=`, commentaire final lorsque clairement extérieur aux quotes, commentaires précédant une variable. Les commentaires sont associés à la variable suivante si contigus ; une ligne blanche sépare les blocs (documenter cette convention).

Cas ambigus ou difficiles (multi-lignes, échappements, continuations, interpolations, syntaxe invalide, encodages inattendus) : conserver sans transformation et afficher une représentation brute en lecture seule / message d'avertissement. Ne jamais prétendre supporter une syntaxe avant d'avoir des tests.

## Sauvegarde

- Détecter les changements externes (mtime + taille et, idéalement, hash) avant sauvegarde ; en cas de conflit : interrompre et demander à recharger ou conserver une copie, **jamais écraser silencieusement**.
- Écrire un fichier temporaire dans le même dossier, `flush` / `fsync` puis `os.replace` de manière atomique si possible ; préserver les permissions initiales. Éviter les permissions trop larges pour les nouveaux fichiers ; documenter les cas des liens symboliques.
- Ne pas sauvegarder automatiquement lors du masquage/révélation de secrets ni pendant la navigation.
- Faire échouer explicitement une sauvegarde qui modifierait involontairement des lignes inconnues.
- Adapter l'implémentation atomique aux garanties du système cible : remplacement dans le
  même volume, synchronisation quand elle est disponible, permissions Unix préservées et
  ACL/attributs Windows étudiés explicitement. Un comportement moins sûr doit produire une
  erreur, jamais une sauvegarde silencieusement dégradée.

## Sécurité et vie privée

Le contrôle de valeurs vides est une aide d'interface, non une validation du format `.env`.
Il compare l'état courant des variables à la copie chargée à l'ouverture ou après la
dernière sauvegarde, par identifiant de ligne, et conserve pour la session les identifiants
des champs de valeur édités. Le champ de valeur est synchronisé sur l'événement natif
`input` capturé par son conteneur : deux saisies rapprochées ne peuvent ainsi laisser
dans le modèle une valeur intermédiaire différente de celle visible dans le champ.
Une valeur `trim()` vide nouvellement ajoutée ou éditée puis laissée vide déclenche une
confirmation, même si elle était initialement vide ;
les anciennes valeurs restées vides ne la déclenchent pas. Ce suivi est réinitialisé à
l'ouverture d'un fichier et après une sauvegarde réussie. Aucune valeur n'est incluse
dans le dialogue, et une confirmation ne modifie pas les octets à sauvegarder.

- Les projets locaux fonctionnent hors ligne, sans télémétrie ni service tiers.
  Un projet SSH utilise uniquement l'alias OpenSSH configuré par l'utilisateur.
- Aucune valeur de `.env` dans la configuration persistante ou les logs.
- Masquage visuel des champs sensibles avec un vrai widget de saisie secrète (pas un faux texte `••••` injecté comme valeur).
- Ne jamais exécuter ni `source` les fichiers `.env`.
- La copie utilise uniquement la capacité Tauri `clipboard-manager:allow-write-text` :
  l'application ne demande aucune permission de lecture du presse-papiers.
- Les backups éventuels et les fichiers temporaires peuvent contenir des secrets : choisir un comportement sûr, explicite et documenté.

## Architecture du dépôt

- `src/` : interface Vue/Quasar, modèle de présentation et tests Vitest.
- `src-tauri/src/` : commandes Tauri, modèle round-trip, stockage et catalogue Rust.
- `src-tauri/capabilities/` : permissions minimales accordées au shell.
- `src-tauri/icons/` et `public/` : ressources de packaging et du frontend.
- `legacy/python-gtk/` : prototype et corpus historique de conformité.

Le catalogue est stocké dans `catalog.json` sous le dossier de configuration Tauri. Il
contient un arbre ordonné de sections, leurs projets et une liste de projets sans section.
Le champ récursif `children` est optionnel à la lecture afin que les catalogues plats déjà
créés restent valides. L'ancien `projects.json` est volontairement ignoré.

Le catalogue persiste `environment` sur les projets locaux ; l'absence de ce champ dans
un ancien catalogue vaut `development`. Pour les projets SSH existants, l'environnement
historique stocké dans `ssh.environment` reste la source de vérité. Les commandes Tauri
valident les trois valeurs autorisées. Modifier un projet invalide les sessions ouvertes.
La lecture d'un `.env` local détermine son projet par dossier parent exact ; si son
environnement est préproduction/production, la session Rust bloque toute sauvegarde avant
une activation explicite et se reverrouille après écriture. L'interface seule n'est pas
la frontière de sécurité.

## Comparaison de clés

La comparaison de clés utilise la même lecture locale/SSH et le même parseur Rust que
l'éditeur. Une commande Tauri étroite n'accepte que `.env.example` ou `.env.exemple` dans
un projet enregistré et ne renvoie que les noms des clés, le nombre de lignes inconnues
et les noms des doublons, ainsi que les commentaires associés aux clés non dupliquées,
mais jamais les valeurs. Pour un projet local,
le fichier exemple doit être un fichier régulier directement dans le dossier du projet ;
les liens symboliques sont refusés. Le frontend compare des ensembles de clés avec l'état
édité en mémoire ; les ajouts sont des variables à valeur vide et commentaire repris de
l'exemple, soumises à la sauvegarde
normale. Les syntaxes non reconnues ne sont pas déduites comme clés absentes.

## Transport SSH Linux

Un projet SSH conserve l'hôte ou l'alias OpenSSH, l'utilisateur et le port optionnels,
le chemin absolu et l'environnement dans le catalogue, sans clé ni valeur de variable.
Le catalogue ne lance aucune connexion SSH : la découverte des fichiers est déclenchée
à l'expansion du projet, dans une commande asynchrone. La découverte et la lecture passent par `ssh`
avec `BatchMode=yes` et `StrictHostKeyChecking=yes`. Les alias et chemins sont validés ;
les chemins sont cités dans les commandes distantes. Les erreurs SSH connues sont classées
sans réémettre le texte brut de stderr, afin qu'un serveur ne puisse pas inscrire un secret
dans l'interface ou les logs.

Le contenu distant est parsé par le même cœur Rust que les fichiers locaux. La sauvegarde
compare le hash SHA-256 et le mode avant transfert, puis le script distant vérifie de
nouveau le hash avant le remplacement. Un temporaire du même dossier est écrit, l'ancienne
version est copiée avec un `umask` restrictif, puis `mv` remplace la cible. Le propriétaire
doit être l'utilisateur SSH et le groupe du temporaire doit correspondre au fichier.
Les ACL et attributs étendus ne sont pas conservés explicitement : ne pas utiliser cette
fonction sur des fichiers qui en dépendent. Le shell distant doit fournir les outils GNU
indiqués dans le README. Aucun verrou interopérable avec les autres éditeurs n'est pris :
la fenêtre entre la dernière comparaison et le `mv` ne peut pas être éliminée dans ce
protocole. Cette limite doit être revue avant de présenter l'écriture SSH comme une
garantie forte de sauvegarde non destructive.

## Architecture d'exécution

```text
Frontend Quasar/Vue
        │ commandes Tauri typées et minimales
        ▼
Cœur Rust : document, parseur, stockage, projets
        │
        ▼
Fichiers .env et configuration locale
```

Le frontend ne reçoit une valeur secrète que lorsque son affichage ou son édition l'exige.
Les commandes ne doivent jamais journaliser leurs arguments contenant des valeurs. Les
capacités Tauri doivent être limitées aux boîtes de dialogue nécessaires et aux chemins de
projets explicitement enregistrés ; ne pas exposer une API générique de lecture/écriture.

### Historique de migration

1. Créer le shell Tauri/Quasar et reproduire la navigation avec des données de test.
2. Porter le modèle et le parseur en Rust ; exécuter les fixtures Python contre les deux
   implémentations et comparer les octets produits.
3. Porter la sauvegarde atomique, les conflits et le catalogue de projets.
4. Connecter l'interface à des commandes Tauri étroites.
5. Reproduire l'édition inline, les commentaires, le masquage et les diagnostics.
6. Tester le parcours nominal et les cas de sûreté sous Linux, Windows et macOS.
7. Archiver GTK après validation de la version Linux Tauri et décision explicite du
   propriétaire. Réalisé : le prototype réside dans `legacy/python-gtk/`.
