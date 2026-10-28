# Critères d'acceptation et tests

## Checklist fonctionnelle

- [ ] Je peux enregistrer plusieurs dossiers de projets et les retrouver après redémarrage.
- [ ] Je peux imbriquer et déplacer des sections, y ajouter ou déplacer des projets, puis
  retrouver cette arborescence repliée par défaut après redémarrage.
- [ ] Supprimer une branche déplace tous ses projets dans « Sans section » sans supprimer de dossier.
- [ ] Je peux sélectionner un fichier `.env` dans la barre latérale sans ouvrir une modale.
- [ ] Je peux modifier inline une valeur puis Ctrl+S ; le fichier sur disque est modifié.
- [ ] Une variable sensible est masquée visuellement par défaut mais n'est pas écrasée lors de la sauvegarde.
- [ ] Une variable non sensible est visible dès l'ouverture.
- [ ] Je peux copier une valeur, y compris masquée, sans révélation visuelle ni modification
  du document ; EnvEdit ne peut pas lire le presse-papiers.
- [ ] Les commentaires au-dessus d'une clé sont visibles en tooltip et éditables dans un popover.
- [ ] Ajouter, supprimer ou renommer une variable fonctionne.
- [ ] Les modifications non sauvées sont signalées, y compris lors d'un changement de projet.
- [ ] Une modification externe sur disque n'est jamais écrasée silencieusement.
- [ ] Les erreurs (fichier illisible, permissions, format ambigu) sont explicites et sûres.

## Tests automatisés indispensables

1. **Identité** : charger puis sauvegarder sans modification retourne exactement les mêmes octets.
2. **Modification ciblée** : une seule valeur change ; toutes les autres lignes conservent leurs octets.
3. **LF et CRLF** : préserver les fins de ligne et l'absence éventuelle de saut final.
4. **Commentaires** : contigus, multiligne, blancs entre blocs, commentaires inline.
5. **Quotes** : valeurs `abc`, `'a # b'`, `"a # b"`, espaces, `=`, `#`, caractères Unicode.
6. **Doublons** : détecter les clés dupliquées ; ne pas perdre les occurrences.
7. **Syntaxe inconnue** : conservation intégrale et impossibilité d'une transformation dangereuse.
8. **Secrets** : basculer œil ouvert/fermé ne marque pas le document comme modifié ; aucune sauvegarde de points `••••`.
   Copier une valeur masquée transmet la valeur originale au presse-papiers sans la journaliser.
9. **Conflit externe** : changement du fichier entre lecture et sauvegarde → refus d'écrasement.
10. **Répertoire de projets** : ajout/suppression du catalogue ne supprime jamais des fichiers.
11. **Sections** : imbrication récursive, renommage, déplacement de section ou de projet,
    repli par défaut et suppression avec conservation des projets.
12. **SSH Linux** : hôte, utilisateur et chemin invalides refusés ; erreurs SSH classées
    sans exposer stderr ; les scripts distants remplacent
    le fichier après comparaison du hash, gardent une sauvegarde et refusent un conflit.

## Paramètres de projets et version (0.3.0)

- [x] Création locale avec nom et environnement ; anciens projets locaux sans champ
  `environment` interprétés comme développement.
- [x] Modification depuis le menu d'un projet local/SSH, sans conversion de type ;
  contrôle de validité et d'unicité du chemin ou de l'hôte.
- [x] Les fichiers locaux préproduction/production sont verrouillés dans Rust jusqu'à
  activation explicite ; confirmation des noms de clés et reverrouillage après sauvegarde.
- [x] Version visible dans l'en-tête, issue de `package.json` et synchronisée avec Tauri.
- [ ] Validation manuelle de ces parcours dans le paquet Linux installé.

## Comparaison des clés (0.4.0)

- [x] `.env.example` et `.env.exemple` reconnus ; si les deux existent, l'utilisateur
  peut changer de référence.
- [x] Manquantes, supplémentaires et communes calculées sur les noms distincts ; aucune
  valeur de l'exemple dans la réponse de comparaison.
- [x] Ajout individuel ou groupé avec valeur vide, sans écriture avant Enregistrer.
- [x] Commentaire simple ou multiligne repris de l'exemple lors d'un ajout ; en cas de
  clé dupliquée dans l'exemple, aucun commentaire ambigu n'est copié.
- [x] Préprod/prod conserve le verrouillage des ajouts ; doublons et lignes inconnues
  de l'exemple sont signalés comme comparaison partielle.
- [ ] Validation manuelle du panneau sur projet local et SSH de test dans le paquet Linux.

## Avertissement des valeurs vides (0.4.2)

- [x] Valeur vide ou composée d'espaces signalée sous le champ, sans modification du document.
- [x] Ajout vide ou valeur vidée : confirmation avant écriture, ne contenant que les noms
  des clés ; « Enregistrer quand même » permet les valeurs optionnelles.
- [x] Valeur déjà vide à l'ouverture : aucune confirmation répétée lors d'une autre édition.
- [x] Valeur initialement vide, renseignée puis vidée avant sauvegarde : confirmation
  affichée ; une valeur finalement renseignée n'est pas signalée.
- [x] Deux événements de saisie rapprochés (renseigner puis vider) gardent le champ et
  le modèle synchronisés ; la sauvegarde reçoit la valeur vide après confirmation.
- [x] Préproduction/production : avertissement intégré à la confirmation existante.
- [ ] Validation manuelle du parcours dans le paquet Linux installé.

## Validation SSH avant usage en production

- [ ] Le bouton unique « Ajouter un projet » offre Local / SSH dans la même modale,
  depuis l'en-tête et depuis une section ; le projet est ajouté à la bonne section.
- [ ] Le lancement affiche le catalogue sans contacter les hôtes SSH ; déplier un projet
  distant charge ses fichiers et un hôte indisponible ne bloque pas les autres projets.
- [ ] Un projet SSH par clé et alias `~/.ssh/config` découvre les `.env*` du dossier.
- [ ] Une coupure SSH affiche une erreur sans effacer le catalogue ni le fichier édité.
- [ ] Préprod et prod s'ouvrent en lecture seule ; déverrouillage par fichier seulement.
- [ ] La confirmation affiche l'hôte, le fichier et les clés, jamais les valeurs.
- [ ] Une modification concurrente est refusée et le contenu édité reste visible.
- [ ] Les permissions et le groupe du fichier restent identiques ; une sauvegarde de
  l'ancienne version est créée sur le serveur.

Ces vérifications nécessitent un serveur de test contrôlé. Elles n'ont pas été exécutées
contre les serveurs de l'utilisateur.

## Exemple de fixture

```dotenv
# URL principale du service
APP_URL="https://example.test/a?x=1#frag"

# Connexion DB
# Ne pas exposer ce mot de passe
DB_PASSWORD='secret#with=sign'
export APP_DEBUG = false # mode production
UNUSUAL_LINE_NOT_PARSED
```

Le texte ci-dessus est une fixture de test sans secrets réels.

## Définition de terminé

- Application lançable sur Ubuntu avec documentation reproductible.
- Suites Vitest et Rust vertes ; suite `pytest` historique verte lorsqu'elle est exécutée
  depuis `legacy/python-gtk/` avec ses dépendances de développement.
- Parcours nominal vérifié manuellement avec un `.env` de test de 100 lignes.
- Liste transparente des syntaxes `.env` non prises en charge.

## Parité de la version portable

> **Statut : différé.** La validation Windows/macOS est volontairement reportée. Cette
> section reste la définition de parité future, mais ne bloque pas la prochaine version
> Linux.

La migration Tauri n'est considérée terminée que si :

- la checklist fonctionnelle ci-dessus passe sous Linux, Windows et macOS ;
- les mêmes fixtures sous `legacy/python-gtk/tests/fixtures/` produisent exactement les
  mêmes octets avec le cœur Python historique et le cœur Rust, sauf divergence explicitement
  documentée et approuvée ;
- les tests d'identité, de modification ciblée, de secrets et de conflit externe passent
  sur les trois systèmes ;
- les chemins contenant espaces, accents et caractères propres à la plateforme sont testés ;
- les fins de ligne LF et CRLF et l'absence de saut final sont conservées indépendamment du
  système hôte ;
- supprimer un projet du catalogue ne supprime jamais son dossier sur aucune plateforme ;
- aucun secret n'apparaît dans les logs frontend, Rust, Tauri, crash reports ou fichiers de
  configuration ;
- l'application packagée fonctionne sans Python, Node.js, serveur local ni accès réseau ;
- les changements non enregistrés et la fermeture de fenêtre sont protégés de façon
  identique sur chaque plateforme.
