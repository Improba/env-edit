# Instructions pour les agents de développement

Tu travailles sur EnvEdit, un éditeur desktop de fichiers `.env` multiprojets.

L'application principale est à la racine : Tauri 2 + Vue 3/Quasar avec un cœur Rust. La
cible active de livraison est Linux ; Windows et macOS restent des cibles futures. L'ancien
prototype Python/GTK est conservé sous `legacy/python-gtk/` uniquement comme référence
historique et oracle de conformité. N'ajoute aucune dépendance de l'application principale
vers ce runtime Python.

Lis d'abord `README.md`, `SPEC.md`, `TECHNICAL.md`, `ACCEPTANCE.md`.

Priorités absolues : (1) sauvegarde fidèle/non destructive ; (2) édition directe sans modale ; (3) navigation multiprojets persistante ; (4) tooltips des commentaires et confidentialité des secrets.

Ne modifie jamais les vrais `.env` de l'utilisateur pour les tests : crée exclusivement des fixtures sous `tests/` ou un dossier temporaire. Ne lis pas les vrais secrets du projet si cela n'est pas nécessaire. N'ajoute aucune télémétrie ni dépendance réseau à l'exécution.

Procède par jalons vérifiables : cœur Rust et tests de round-trip, stockage sûr, commandes
Tauri minimales, puis interface Vue/Quasar. Après chaque jalon, exécute les tests pertinents
et indique clairement ce qui fonctionne, ce qui reste à faire et les risques connus. Fais
des choix raisonnables si une spécification manque, documente-les, n'invente pas de prise
en charge de syntaxes non testées.

Avant de déclarer le MVP terminé, valide tous les critères de `ACCEPTANCE.md` ou indique
explicitement les écarts. Les commandes courantes se lancent depuis la racine avec `npm` ;
les tests historiques Python se lancent depuis `legacy/python-gtk/`.

Avant de déclarer la migration portable terminée, valide également la section de parité
multiplateforme de `ACCEPTANCE.md`. Toute divergence de parsing ou de sauvegarde entre Python
et Rust doit être documentée et approuvée, jamais normalisée silencieusement.
