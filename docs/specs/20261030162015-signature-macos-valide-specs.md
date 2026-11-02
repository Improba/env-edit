---
name: Signature macOS techniquement valide
overview: Le prochain paquet macOS a une signature que le système reconnaît comme cohérente, sans devenir une distribution publique identifiée.
statut: "✅ Cadrées"
livraison: "📤 à présenter"
creation: "30/10/2026 16:20"
modification: "30/10/2026 16:24"
---

# Specs : Signature macOS techniquement valide

**Statut** : ✅ Cadrées

**Livraison** : 📤 à présenter

> L'application fonctionne sur Apple Silicon après suppression de l'attribut de quarantaine, mais la vérification de signature du bundle échoue (`code has no resources but signature indicates they must be present`). Il faut une solution qui rende cette signature techniquement valide, puis la distinguer des exigences de signature Developer ID et de notarisation pour une distribution publique. La version déjà publiée ne sera pas réémise.

| Plan | Recette | Review |
|:-----|:--------|:-------|
| [`20261030162015-signature-macos-valide-plan.md`](../plans/20261030162015-signature-macos-valide-plan.md) | [`20261030162015-signature-macos-valide-recette.md`](../recettes/20261030162015-signature-macos-valide-recette.md) | [`20261030162015-signature-macos-valide-review.md`](../reviews/20261030162015-signature-macos-valide-review.md) |

> **Cascade.** Une modification du besoin entraîne **obligatoirement** une révision du **plan** (et de la **recette** si le cahier existe). Inscrire seul `✅ Cadrées` ne le fait pas.
>
> **Backlog.** Seul `imp-backlog` met à jour `BACKLOG.md`. Ce fichier ne l'écrit pas.

---

## Besoins

La personne qui récupère le prochain paquet macOS obtient une application dont la vérification de signature réussit. Cette application n'est pas pour autant acceptée comme un logiciel identifié pour une distribution publique. Les textes du projet et de la prochaine publication décrivent ce niveau. La version 1.0.0 déjà en ligne reste celle qui a été publiée.

---

## Découpage

### Principales

#### S1 — Prochain paquet macOS à signature cohérente

> je veux que la vérification de signature réussisse sur le prochain paquet macOS, tout en restant distincte d'une distribution publique identifiée

| | |
|:---|:---|
| **Origine** | PO — « déterminer pourquoi le bundle déclenche `code has no resources but signature indicates they must be present`, puis générer un bundle dont la signature est techniquement valide, et distinguer ce correctif de la signature Developer ID et de la notarisation » |
| **Dépend de** | — |

##### Besoin (ticket / PO / client)

| # | Comportement |
|:-:|:---|
| 1 | Le prochain paquet macOS contient une application dont la vérification de signature réussit. |
| 2 | La même application reste refusée lorsqu'on demande au système si elle peut être ouverte comme un logiciel identifié pour une distribution publique. |
| 3 | Ce résultat vaut pour le prochain build. Il ne modifie pas le comportement déjà observé de la version publiée. |

##### Critères d'acceptation (ticket / PO / client)

- [ ] La vérification de signature de l'application du prochain paquet macOS réussit.
- [ ] L'évaluation de cette application pour une distribution publique identifiée échoue encore.
- [ ] La version 1.0.0 déjà publiée n'est pas remplacée.

### Finition

#### S2 — Textes alignés sur ce niveau de signature

> je veux que la documentation et la note de la prochaine publication disent la vérité sur ce niveau de signature

| | |
|:---|:---|
| **Origine** | PO — le correctif de signature valide doit rester distinct d'une distribution publique, et les textes qui disent aujourd'hui que les installateurs ne sont pas signés ne doivent pas devenir faux |
| **Dépend de** | — |

##### Besoin (ticket / PO / client)

| # | Comportement |
|:-:|:---|
| 1 | La documentation du projet dit que le paquet macOS du prochain build a une signature vérifiable, et qu'il n'est pas notarié pour une distribution publique. |
| 2 | La note de la prochaine publication dit la même chose. |
| 3 | Les textes qui disent que la validation fonctionnelle macOS reste différée restent vrais. |

##### Critères d'acceptation (ticket / PO / client)

- [ ] La documentation du projet ne dit plus que le paquet macOS n'est pas signé.
- [ ] Elle dit qu'il n'est pas notarié pour une distribution publique.
- [ ] La note prévue pour la prochaine publication dit la même chose.
- [ ] Les documents qui reportent la validation fonctionnelle macOS ne sont pas réécrits.

---

## Hors scope

| Exclu | Pourquoi |
|:------|:---------|
| Certificat Developer ID | Identité d'éditeur pour une distribution publique, au-delà d'une signature vérifiable. |
| Secrets du compte Apple | Ils n'existent que pour porter cette identité et la notarisation. |
| Notarisation | Contrôle Apple exigé pour une distribution publique identifiée. |
| Ticket de notarisation gravé sur le paquet | Dernière étape de cette même distribution publique. |
| Réémission de la version 1.0.0 | La version déjà publiée reste en ligne. Le correctif concerne le prochain build. |
| Validation fonctionnelle macOS et prise en charge de cette cible | Déjà reportées. Une signature vérifiable ne les rend pas acquises. |
