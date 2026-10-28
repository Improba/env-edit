# Prompt à donner à Codex

Développe **EnvEdit** à partir des documents présents dans ce dossier : `README.md`, `SPEC.md`, `TECHNICAL.md`, `ACCEPTANCE.md` et `AGENTS.md`.

Je veux une application **native Ubuntu** avec **Python 3 + GTK4 + libadwaita**, destinée à éditer directement les fichiers `.env` de plusieurs projets. L'interface est scindée : **barre latérale persistante de projets/fichiers à gauche**, **liste de variables éditables directement à droite**. Les commentaires qui précèdent chaque variable sont consultables via un **tooltip** sur une icône info et modifiables via un popover. Les valeurs normales sont visibles ; les secrets sont masqués par défaut avec un bouton œil. **Ctrl+S sauvegarde le vrai fichier**. Pas de snapshots, pas de modales pour l'édition courante, pas de cloud.

Commence par examiner les spécifications et créer un plan d'implémentation court, puis **implémente effectivement** une première version fonctionnelle, en découpant en étapes :

1. Parseur/écrivain `dotenv` non destructif et tests (identité octet pour octet, modification ciblée, commentaires, quotes, CRLF, doublons, syntaxes ambiguës).
2. Gestion des projets et sauvegarde atomique avec détection de conflit externe.
3. UI GTK4/libadwaita avec arbre de projets, éditeur inline, infobulles de commentaires et secrets masqués.
4. Instructions précises d'installation et de lancement sous Ubuntu, tests automatisés et vérification manuelle de bout en bout.

Ne t'arrête pas après le plan : écris le code. Si une dépendance native empêche les tests UI dans ton environnement, poursuis les tests du modèle et signale précisément le blocage. Ne modifie aucun véritable fichier `.env` autre que les fixtures et fichiers temporaires de test.

Critère crucial : sauvegarder un fichier sans modification doit produire **exactement les mêmes octets** et éditer une variable ne doit pas réécrire les autres lignes. Ne jamais perdre silencieusement de commentaires ni de secrets.
