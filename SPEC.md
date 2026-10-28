# Cahier des charges — EnvEdit

## Vision

Un éditeur graphique de fichiers `.env` lisible et immédiat, avec une barre latérale de projets, inspirée de la navigation d'Envarsa mais **sans** sa logique de snapshots ni ses modales d'édition.

## MVP impératif (P0)

1. Fenêtre desktop avec deux panneaux redimensionnables. L'expérience doit rester cohérente
   sous Linux, Windows et macOS sans perdre les raccourcis et conventions de chaque système.
2. Panneau gauche : liste de projets (nom + dossier) avec ajout/suppression de projets, projets repliables, fichiers `.env` présents dans chaque projet. Ajouter un projet en choisissant un dossier ; persister la liste entre lancements. Supprimer un projet de la liste ne supprime jamais les fichiers du disque.
   Les projets peuvent être regroupés dans une arborescence de sections et sous-sections,
   créées, renommées, déplacées, supprimées et repliées. Supprimer une branche replace tous
   ses projets dans « Sans section » et ne supprime jamais leurs dossiers. Un projet peut
   être déplacé vers n'importe quelle section.
3. Découverte des fichiers `.env`, `.env.local`, `.env.production`, `.env.example`, et de façon générale `.env.*` dans le dossier projet. Recherche récursive optionnelle, **désactivée par défaut** pour éviter de parcourir de gros répertoires ; ignorer `.git`, `node_modules`, `.venv`, `venv`, `dist`, `build`. Prévoir rafraîchissement explicite.
4. Panneau droit : titre du fichier sélectionné, champ de recherche, liste scrollable de paires clé/valeur. Édition **inline** sans modale ; interface réactive sur 100+ variables.
5. Commentaire précédant une variable affiché via une petite icône d'information et un tooltip (survol / focus clavier). Au clic sur l'icône, permettre d'éditer ce commentaire par un popover non bloquant. Supporter les commentaires multiligne.
6. Valeurs standard visibles par défaut. Masquer par défaut les variables à nom sensible (`PASSWORD`, `PASSWD`, `SECRET`, `TOKEN`, `API_KEY`, `PRIVATE_KEY`, `CREDENTIAL`, comparaison insensible à la casse) avec bouton révéler. Ne jamais modifier la valeur sous-jacente quand elle est masquée ou révélée.
7. `Ctrl+S` enregistre **dans le fichier d'origine**, avec confirmation seulement en cas de conflit ou anomalie. Indicateur de modifications non enregistrées. Au changement de fichier/projet avec des changements non sauvés : enregistrer / abandonner / annuler, ou conserver des brouillons par fichier ; choisir l'approche la plus simple et la tester.
8. Ajouter/supprimer/renommer une variable. Détecter les doublons et afficher un avertissement sans réécrire automatiquement le fichier.
9. Préserver l'ordre, les commentaires, les lignes vides, l'indentation, le type de fin de ligne (LF/CRLF), les guillemets et les valeurs non modifiées. Les sauvegardes doivent être **minimales** et ne modifier que les lignes/sections explicitement éditées.
10. Interface en français par défaut, chaînes préparées pour traduction ; raccourcis clavier standard.
11. Affichage clair des erreurs de lecture, d'encodage, de permission, de syntaxe non prise en charge ; **ne jamais sauvegarder silencieusement en détruisant du contenu inconnu**.
12. Signaler discrètement les valeurs vides ou composées uniquement d'espaces. À la
    sauvegarde, avertir uniquement pour une clé nouvellement ajoutée sans valeur ou une
    valeur qui vient d'être vidée, même si elle était vide à l'ouverture et a été
    renseignée entre-temps ; ne pas répéter l'alerte pour les valeurs restées vides
    sans être renseignées. La confirmation ne bloque pas les variables optionnelles et ne
    révèle jamais les valeurs. En préproduction/production, intégrer l'avertissement à
    la confirmation de sauvegarde existante.

## Priorités secondaires (P1)

- Drag-and-drop d'un dossier sur la barre latérale.
- Mode Compact (une ligne par variable) et Confort (étiquette + valeur sur deux lignes).
- Comparaison `.env` ou `.env.*` vs `.env.example` / `.env.exemple` : clés manquantes,
  supplémentaires et communes, **sans exposer les valeurs**. Un résumé discret ouvre
  le détail ; ajout explicite des clés manquantes avec valeur vide et commentaire associé
  de l'exemple (sauf clé dupliquée), sans sauvegarde
  automatique. Les clés supplémentaires restent informatives. L'édition protégée
  préprod/prod s'applique aussi aux ajouts.
- Filtre par préfixe (`APP_`, `DB_`, etc.) et recherche dans les commentaires.
- Option de masquer toutes les valeurs / révéler toutes les valeurs (avec avertissement).
- Raccourci Ctrl+O, Ctrl+F, Ctrl+N / ajout selon les conventions de la plateforme.

## Hors périmètre MVP

- Snapshots, versioning, synchronisation cloud, coffre-fort, comptes utilisateurs, gestion d'équipes.
- Exécution / sourcing du `.env`, substitutions de variables ou appels réseau hors projet SSH.
- Modification automatique de fichiers sans action de l'utilisateur.

## Projets SSH (extension Linux)

Chaque projet local comme SSH a un nom et un environnement développement, préproduction ou
production. Le formulaire local demande le dossier, le nom et l'environnement ; le menu du
projet permet de modifier ces paramètres après création (ou l'hôte, l'utilisateur, le port
et le chemin pour SSH). Le type local/SSH et la section ne changent pas dans ce formulaire.
L'environnement est visible dans l'arborescence et près du fichier ouvert. Les projets
locaux préproduction/production suivent aussi le parcours lecture seule, activation par
fichier, confirmation des clés et reverrouillage après sauvegarde.

- Un seul point d'entrée « Ajouter un projet », y compris dans les sections. Une modale
  à onglets Local / SSH évite un écran de choix supplémentaire ; le mode Local s'ouvre
  par défaut et les options SSH secondaires restent repliées.
- Ajouter un projet par hôte ou alias `~/.ssh/config`, utilisateur et port optionnels,
  dossier absolu distant et environnement développement / préproduction / production.
  Les clés restent gérées par OpenSSH.
- Préproduction et production : lecture seule à l'ouverture, activation explicite de
  l'édition pour un seul fichier, récapitulatif des noms de clés avant sauvegarde et
  reverrouillage après sauvegarde.
- Refuser l'écrasement lorsqu'une modification distante est détectée ; garder les
  modifications dans l'interface lors d'un conflit ou d'une déconnexion.
- L'utilisation locale reste entièrement hors ligne. Pas de `sudo` ni de mot de passe
  interactif dans cette étape.

## Portabilité

- L'application principale est une application Tauri 2 avec un frontend Vue 3/Quasar et un
  cœur Rust. Le prototype GTK/Linux est archivé sous `legacy/python-gtk/`.
- Linux est la cible actuellement distribuée ; Windows et macOS restent des cibles futures.
- Toutes les fonctionnalités P0, les garanties de confidentialité et le round-trip
  octet-pour-octet sont obligatoires sur chaque plateforme.
- L'interface peut adopter les conventions visuelles de la plateforme, mais le parcours
  projet → fichier → édition inline → sauvegarde doit rester identique.
- Aucun serveur local, runtime Python, compte ou service cloud ne doit être requis.
  La connexion réseau n'est utilisée que lorsqu'un projet SSH est ouvert ou actualisé.
- Le corpus de tests GTK/Python doit être conservé tant qu'il sert à vérifier la conformité
  du parseur Rust, sans devenir une dépendance d'exécution.

## Parcours nominal

1. L'utilisateur ajoute un dossier `mon-api`.
2. EnvEdit affiche `mon-api` et ses `.env` dans l'arbre de gauche.
3. Il sélectionne `.env.local`.
4. Il voit immédiatement les valeurs non sensibles et les commentaires en tooltip.
5. Il clique dans `DB_HOST`, modifie la valeur et presse `Ctrl+S`.
6. Le fichier `.env.local` sur disque reflète la seule modification prévue.

## Ergonomie

- La barre latérale reste visible lors de l'édition ; possibilité de l'ajuster en largeur.
- Pas de modale pour les opérations courantes de modification.
- La sélection et le défilement restent stables après une édition.
- Les tooltips ont une alternative accessible au clavier et un popover pour les écrans tactiles.
- Les valeurs longues doivent être lisibles et éditables sans tronquer les données.
- Ne pas afficher de secrets dans les logs, les messages d'erreur ou les notifications.
