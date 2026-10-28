# État d'implémentation du MVP

La version Tauri 2 + Vue 3/Quasar + Rust est désormais l'application principale à la racine
du dépôt. Le prototype Python/GTK est archivé sous `legacy/python-gtk/` comme référence de
conformité. La validation et la distribution Windows/macOS sont différées par décision du
propriétaire ; la cible active est la version Linux.

## Jalons vérifiés

1. **Parseur round-trip** : lignes et fins de ligne exactes, éditions ciblées,
   commentaires associés, doublons et diagnostics de syntaxe inconnue.
2. **Stockage sûr** : conflit par métadonnées et empreinte, écriture atomique, permissions,
   catalogue XDG et découverte non récursive par défaut.
3. **Interface Vue/Quasar** : panneau de projets repliable en rail, édition inline, filtre,
   secrets masqués, commentaires accessibles et raccourcis.

## Validation des critères d'acceptation

- Plusieurs projets persistants : test automatisé.
- Sélection sans modale et édition inline + `Ctrl+S` : implémenté, contrôle UI manuel requis.
- Secret masqué sans altération : logique testée et champ HTML de type `password`.
- Valeurs ordinaires visibles : implémenté.
- Tooltip et popover de commentaire multiligne : implémenté.
- Ajout, suppression et renommage : modèle testé et contrôles inline implémentés.
- Changements non sauvés : point dans le titre et choix enregistrer/abandonner/annuler à
  la navigation.
- Conflit externe : test automatisé, écrasement refusé.
- Erreurs d'encodage, permissions et syntaxes ambiguës : messages explicites.
- 100+ variables : round-trip ciblé testé avec 120 variables ; recette UI manuelle à refaire.

## Écarts et risques connus

- Les tests de composants couvrent le chargement d'un fichier et le panneau repliable ; les
  interactions desktop complètes restent à valider manuellement sous Ubuntu.
- La recherche récursive existe dans le modèle, reste désactivée par défaut et n'a pas de
  réglage dans cette première UI (elle est optionnelle dans le P0).
- Le glisser-déposer, les modes Compact/Confort et la comparaison `.env.example` sont P1.
- La fermeture directe de la fenêtre n'affiche pas encore de confirmation ; la protection
  est active lors du changement de fichier.

## Jalons de la migration Tauri

1. **Socle — réalisé** : Tauri 2, Vue 3, Quasar et TypeScript initialisés.
   L'interface de démonstration reproduit projets repliables, sélection de fichier, recherche,
   édition inline, commentaires, ajout/suppression et masquage des secrets. Les capacités
   Tauri restent volontairement limitées à `core:default` et au dialogue d'ouverture.
   L'ajout persistant d'un dossier et la découverte non récursive des `.env*` sont désormais
   connectés au backend Rust ; le chargement et la sauvegarde sont maintenant actifs.
2. **Contrat de conformité — réalisé pour le parseur initial** : la fixture actuelle est
   consommée directement par Rust ; identité, modification ciblée et CRLF sont comparés
   octet-pour-octet.
3. **Cœur Rust — parseur réalisé, opérations avancées à poursuivre** : modèle ligne par
   ligne, affectations, quotes, commentaires associés, diagnostics, doublons et détection
   des clés sensibles sont portés. La sélection d'un fichier enregistré charge désormais
   ses variables réelles dans Quasar. Renommage, commentaires, ajout, suppression et
   sauvegarde persistante sont maintenant connectés.
4. **Stockage portable — Unix validé, Windows à valider** : sessions en mémoire, changements
   ciblés, empreinte SHA-256, métadonnées, permissions, temporaire dans le même dossier,
   `fsync` et remplacement atomique sont actifs sous Linux/macOS. Le code conditionnel
   Windows emploie `ReplaceFileW` mais n'a pas été compilé ni exécuté sur un hôte Windows.
5. **Interface — réalisée sous Linux** : navigation avec rail compact, édition inline,
   tooltips/popovers, recherche et gestion des changements non enregistrés.
6. **Tests UI — amorcés** : chargement d'un `.env` et repli du panneau couverts par des
   tests de composants ; la recette visuelle et clavier complète reste manuelle.
7. **Distribution Linux — réalisée** : paquets Debian amd64 et AppImage générés. La
   signature, la publication et la parité Windows/macOS restent différées.
8. **Arborescence de projets — réalisée** : sections et sous-sections récursives, zone « Sans
   section », création, renommage, suppression sûre, déplacement des sections et projets par
   menu et arborescence repliée au démarrage. Les catalogues plats restent lisibles ;
   l'ancien `projects.json` est volontairement ignoré.

### Validation du jalon portable 1

- `npm run build` : TypeScript strict et build Vite réussis.
- `npm run test` : 4 tests frontend réussis, dont 2 tests de composants couvrant le
  chargement des variables réelles et le rail de navigation compact.
- `cargo test` : 12 tests Rust réussis, dont round-trip, charge utile inchangée strictement
  identique, transformations ciblées, conflit, permissions et remplacement atomique sur
  fixture temporaire, ainsi que retrait du catalogue sans suppression du dossier.
- `npm audit` : aucune vulnérabilité connue après mise à jour de Vitest.
- `npm run tauri dev` : compilation et démarrage du binaire Linux réussis.
- Suite Python de référence : 25 tests toujours réussis.

La sauvegarde est connectée à Quasar avec `Ctrl+S` et état modifié. Les changements de
fichier, rafraîchissements, retraits de projet et fermetures de fenêtre protègent les
modifications non sauvées. Un conflit propose désormais le rechargement explicite. Les
paquets Linux de développement sont générés avec succès au format Debian et AppImage. Le
prochain jalon est une recette manuelle Linux structurée (installation propre, clavier,
accessibilité, conflits et fichiers volumineux), puis la préparation d'une publication avec
numéro de version, notes et sommes de contrôle.

### Décision de périmètre temporaire

- Cible active : Linux, testée et packagée.
- Windows/macOS : architecture conservée, validation, signature et distribution reportées.
- Le chemin Windows `ReplaceFileW` reste non vérifié et ne doit pas être présenté comme prêt.
- La parité multiplateforme reste un objectif ultérieur, pas un critère de sortie de la
  prochaine version Linux.

### Principaux risques restants

- Les garanties de remplacement atomique et de permissions diffèrent entre Unix et Windows.
- Un frontend web peut accidentellement dupliquer des secrets dans son état, ses outils de
  développement ou ses logs ; les frontières Tauri doivent rester minimales.
- Le portage du parseur peut sembler correct tout en changeant des octets non édités ; la
  comparaison différentielle avec les fixtures Python est donc bloquante.
- Quasar facilite l'homogénéité, mais l'accessibilité clavier, les tooltips et les conventions
  de chaque OS devront être vérifiés explicitement.
