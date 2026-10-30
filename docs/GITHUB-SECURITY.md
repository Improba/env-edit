# Protection du dépôt public EnvEdit

Ces réglages s'appliquent dans **Improba/env-edit → Settings** avec un compte administrateur.
Ils ne sont pas définis par les fichiers du dépôt. Vérifier leur état dans GitHub après
activation, notamment avant le prochain tag. Le dépôt public permet les rulesets GitHub Free.

## Branche `main`

Créer dans **Rules → Rulesets** un *branch ruleset* actif ciblant `main` :

- **Require a pull request before merging** : les changements se préparent et se testent
  toujours en local, mais arrivent sur `main` par PR. Une approbation obligatoire n'est
  judicieuse que si une autre personne peut effectivement relire les PR ; sinon, garder
  zéro approbation requise pour éviter de bloquer le mainteneur unique.
- **Require status checks to pass** : choisir le check exact `Linux tests and .deb` du
  workflow `Validate main`. Activer la mise à jour de la branche avant fusion si souhaité.
  Les builds Windows/macOS ne sont pas exigés sur les PR : ils tournent après fusion sur
  `main`, et le tag attend le succès du run complet pour son SHA.
- **Block force pushes** et **Restrict deletions**. Ne pas créer de contournement permanent
  pour les administrateurs si l'objectif est de réellement empêcher le push direct.

Un push direct sur `main` déclenche la CI seulement après la mise à jour : ce n'est pas
une protection préventive équivalente à une PR avec check requis.

## Tags de version

Créer un *tag ruleset* actif ciblant `v*` avec **Restrict creations** et un contournement
réservé aux personnes/équipes chargées des releases. Vérifier qu'elles peuvent toujours
créer `vX.Y.Z` après activation. Créer un second ruleset sur `v*` avec **Restrict updates**
et **Restrict deletions**, sans contournement courant, pour éviter le déplacement ou la
suppression des tags publiés. Une personne ayant un contournement sur un ruleset peut
contourner toutes ses règles : la séparation empêche qu'une autorisation de création
devienne aussi une autorisation de déplacement.

Le workflow refuse un tag si la version des fichiers ne correspond pas, si le commit
n'est pas la pointe actuelle de `main`, ou si `Validate main` n'a pas réussi sur ce SHA.
Un tag historique ou un commit seulement présent sur une branche ne publie donc rien.

## GitHub Actions

Dans **Actions → General** :

- Choisir **Read repository contents permission** pour le `GITHUB_TOKEN` par défaut.
  `Validate main` reste en lecture ; seul le job final de publication reçoit
  `contents: write`.
- Demander une approbation pour les workflows des contributeurs externes sur les PR de
  forks. Examiner particulièrement les changements à `.github/workflows/` avant de
  lancer un workflow proposé par un tiers.
- Si une liste d'actions autorisées est activée, y inclure `actions/checkout`,
  `actions/setup-node`, `actions/upload-artifact`, `actions/download-artifact`,
  `dtolnay/rust-toolchain` et `swatinem/rust-cache`. Tester une PR et un run `main`
  avant de restreindre davantage cette liste.
- Ne pas activer la politique « SHA complet obligatoire » avant d'avoir remplacé les
  références `@v*` des workflows par des SHA vérifiés : cela bloquerait la CI actuelle.

Le workflow de release construit les trois installateurs en parallèle, mais un seul job
les télécharge, vérifie leur présence et publie la release **après** la réussite des trois.
Ne pas activer les *immutable releases* sans valider d'abord ce nouveau parcours sur une
version ultérieure : la release v1.0.0 a été produite par l'ancien workflow et cette
option verrouille les pièces jointes dès la publication.
