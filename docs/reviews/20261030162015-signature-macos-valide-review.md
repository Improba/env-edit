---
name: Signature macOS techniquement valide
overview: Les deux développements livrés correspondent aux specs et au plan. Aucun correctif.
todos: []
creation: "30/10/2026 16:31"
modification: "30/10/2026 16:31"
---

# Review : Signature macOS techniquement valide

| Specs | Plan | Recette |
|:------|:-----|:--------|
| [`20261030162015-signature-macos-valide-specs.md`](../specs/20261030162015-signature-macos-valide-specs.md) | [`20261030162015-signature-macos-valide-plan.md`](../plans/20261030162015-signature-macos-valide-plan.md) | [`20261030162015-signature-macos-valide-recette.md`](../recettes/20261030162015-signature-macos-valide-recette.md) |

---

> **À l'implémentation.** `/imp-work` n'exécute que les correctifs `🔨 À développer`. Ids `V1.n` ↔ `D1.n`. Ordre du fichier **et** des `todos` : 🚨 Critique → ⚠️ Important → 💡 Mineur (à égalité : ordre du plan). Avancement : **ce fichier seulement**. Ne pas écrire le cahier. Ne pas passer en `✅ Validé`.
>
> **Backlog.** Seul `imp-backlog` met à jour `BACKLOG.md`. Ce fichier ne l'écrit pas.

---

## Synthèse

D1.1 et D2.1 sont `🧪 À recetter`. Le code touche les trois chemins du plan et rien d'autre. `bundle.macOS.signingIdentity` vaut `"-"`. `hardenedRuntime` n'est pas écrit, donc le défaut du bundler reste. Les commandes de build des workflows ne changent pas. Le README et la note de publication disent une signature ad hoc vérifiable, l'absence de notarisation, et que Linux et Windows ne sont pas signés. Les documents de validation macOS différée ne sont pas modifiés. Aucun écart, aucun bug dans ce diff, aucun correctif.

---

## Écarts specs ↔ code

RAS.

---

## Régressions et impacts

RAS. Le diff n'atteint pas le parseur, la sauvegarde ni l'interface.

---

## Conventions du projet

RAS. Le JSON reste en indentation de 2. Les phrases ajoutées restent dans le registre du README et de la note existante.

---

## Bugs

RAS.

---

## Performances

RAS.

---

## Optimisations

RAS.

---

## Refactorings

RAS.

---

## Sécurité

RAS. Aucun secret ajouté. L'identité `"-"` n'est pas un certificat.
