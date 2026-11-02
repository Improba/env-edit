---
name: Signature macOS techniquement valide
overview: Vérifier que le prochain paquet macOS a une signature cohérente, qu'il reste refusé comme distribution identifiée, et que les textes disent ce niveau.
creation: "30/10/2026 16:24"
modification: "02/11/2026 13:46"
---

# Recette : Signature macOS techniquement valide

| Specs | Plan | Review |
|:------|:-----|:-------|
| [`20261030162015-signature-macos-valide-specs.md`](../specs/20261030162015-signature-macos-valide-specs.md) | [`20261030162015-signature-macos-valide-plan.md`](../plans/20261030162015-signature-macos-valide-plan.md) | [`20261030162015-signature-macos-valide-review.md`](../reviews/20261030162015-signature-macos-valide-review.md) |

> Cahier **manuel + automatique**, guidé par `/imp-recette`. Haut : tests manuels (`R1.n`) — init `🧪 À recetter`, date `—`. Seul l'utilisateur passe une ligne manuelle en `✅ Validé` (date `jj/mm/aaaa`). Bas : `## Tests automatiques` — init `⏳ Non lancé`, date `—`. `/imp-test` évalue tous les contrôles attendus de chaque couple visé, avec une preuve de lancement ; un lancement incomplet ou indisponible ne devient pas vert.
>
> **Cascade.** Une modification des specs ou du plan entraîne **obligatoirement** une révision de cette recette.
>
> **Backlog.** Seul `imp-backlog` met à jour `BACKLOG.md`. Ce fichier ne l'écrit pas.

---

## R1 — Prochain paquet macOS à signature cohérente

| Spec | [S1](../specs/20261030162015-signature-macos-valide-specs.md) |
|:-----|:---|

> je veux que la vérification de signature réussisse sur le prochain paquet macOS, tout en restant distincte d'une distribution publique identifiée

### R1.1 — Vérifier la signature du prochain paquet

| Statut | Date | Spec | Plan | Review |
|:-------|:-----|:-----|:-----|:-------|
| ✅ Validé | 02/11/2026 | [S1](../specs/20261030162015-signature-macos-valide-specs.md) | [D1.1](../plans/20261030162015-signature-macos-valide-plan.md) | [V1.1](../reviews/20261030162015-signature-macos-valide-review.md) |

**Contexte**

Un Mac Apple Silicon. Le paquet à contrôler est celui produit par le build qui contient le correctif, pas le DMG déjà publié en 1.0.0.

**Je fais**

1. Construire le paquet macOS depuis la branche du correctif : `npm run tauri build -- --bundles dmg`.
2. Ouvrir l'application produite dans ce paquet.
3. Lancer `codesign --verify --deep --strict` sur cette application, puis `codesign -d --verbose=4`.
4. Lancer `spctl --assess --type execute` sur la même application.
5. Ouvrir la release GitHub `v1.0.0` et comparer son asset macOS à celui qui était déjà publié.

**Il doit se passer**

La vérification de signature réussit. L'évaluation pour une distribution identifiée échoue. La release `v1.0.0` n'a pas changé d'asset.

**Je dois voir**

- `valid on disk` et `satisfies its Designated Requirement`.
- Une signature ad hoc avec runtime durci, des ressources scellées, et `TeamIdentifier=not set`.
- `spctl` répond `rejected`.
- L'asset macOS de `v1.0.0` est toujours celui publié le 29 octobre 2026.

---

## R2 — Textes alignés sur ce niveau de signature

| Spec | [S2](../specs/20261030162015-signature-macos-valide-specs.md) |
|:-----|:---|

> je veux que la documentation et la note de la prochaine publication disent la vérité sur ce niveau de signature

### R2.1 — Lire les textes du prochain build

| Statut | Date | Spec | Plan | Review |
|:-------|:-----|:-----|:-----|:-------|
| ✅ Validé | 02/11/2026 | [S2](../specs/20261030162015-signature-macos-valide-specs.md) | [D2.1](../plans/20261030162015-signature-macos-valide-plan.md) | [V2.1](../reviews/20261030162015-signature-macos-valide-review.md) |

**Contexte**

Le README du dépôt et la note que le workflow de publication joindra à la prochaine release. Les documents qui reportent la validation fonctionnelle macOS sont encore ceux d'aujourd'hui.

**Je fais**

1. Lire le paragraphe de `README.md` qui décrit la signature des installateurs.
2. Lire la note `--notes` de `gh release create` dans `.github/workflows/publish-release.yml`.
3. Relire les phrases de `ACCEPTANCE.md` et `IMPLEMENTATION.md` qui reportent la validation macOS.

**Il doit se passer**

Les deux premiers textes décrivent une signature macOS vérifiable et l'absence de notarisation. Les phrases de validation différée sont toujours là.

**Je dois voir**

- Le README dit que le paquet macOS a une signature ad hoc vérifiable, qu'il n'est pas notarié, et que les installateurs Linux et Windows ne sont pas signés.
- La note de la prochaine publication dit la même chose, et que l'installateur macOS reste expérimental.
- `ACCEPTANCE.md` et `IMPLEMENTATION.md` disent encore que la validation macOS est différée.

---

## Tests automatiques

> Filet **du projet** — pas de pont `D1.n` ↔ ligne, pas d'id `R`, pas d'étapes à jouer à la main. Une ligne par couple environnement × type **réellement disponible**. Clé = Environnement + Type. États et preuve de lancement : contrat de `imp-test/SKILL.md`, section Cahier. Un vert historique ne prouve pas que le code courant a été testé.
>
> Aucune suite sur le projet → une phrase ici, **pas** de ligne inventée.

| Environnement | Type | Statut | Date |
|:--------------|:-----|:-------|:-----|
| front | unitaire | ✅ Validé | 30/10/2026 |
| src-tauri | unitaire | ✅ Validé | 30/10/2026 |
| legacy | unitaire | ✅ Validé | 30/10/2026 |

Preuve du 30/10/2026 16:36. Commit `58c1570`, changements non commités : `README.md` `b357c7dcef74`, `src-tauri/tauri.conf.json` `ef3971e87764`, `.github/workflows/publish-release.yml` `b6c436090221`.

- front unitaire — `npm test` (`vitest run`). PASS. 3 fichiers, 21 tests, 0 échec, 0 ignoré.
- src-tauri unitaire — `cargo test --locked --manifest-path src-tauri/Cargo.toml`. PASS. `src/lib.rs` : 21 tests, 0 échec, 0 ignoré. `src/main.rs` et doctests : 0 test.
- legacy unitaire — `python3 -m pytest` dans `legacy/python-gtk`. PASS. 25 tests, 0 échec.

Tentative historique du 30/10/2026 16:31 : front PASS (21), src-tauri et legacy INDISPONIBLES. Elle ne vaut plus pour l'état actuel.
