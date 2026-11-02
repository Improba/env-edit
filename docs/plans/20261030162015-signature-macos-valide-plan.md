---
name: Signature macOS techniquement valide
overview: Le bundler macOS signe le bundle en ad hoc avant de produire le DMG, et les textes du prochain build décrivent ce niveau.
todos:
  - id: D1.1
    content: Signer le bundle macOS en ad hoc
    status: done
  - id: D2.1
    content: Aligner les textes sur la signature ad hoc
    status: done
statut: "✅ Validé"
creation: "30/10/2026 16:24"
modification: "02/11/2026 13:34"
---

# Plan : Signature macOS techniquement valide

**Statut** : ✅ Validé

| Specs | Recette | Review |
|:------|:--------|:-------|
| [`20261030162015-signature-macos-valide-specs.md`](../specs/20261030162015-signature-macos-valide-specs.md) | [`20261030162015-signature-macos-valide-recette.md`](../recettes/20261030162015-signature-macos-valide-recette.md) | [`20261030162015-signature-macos-valide-review.md`](../reviews/20261030162015-signature-macos-valide-review.md) |

---

> **À l'implémentation.** `/imp-work` n'exécute que les développements `🔨 À développer`. Ne pas redéfinir le besoin. Ne pas écrire le cahier. Ne pas passer en `✅ Validé`.
>
> **Cascade.** Une modification de ce plan entraîne **obligatoirement** une révision de la recette **si** le cahier existe.
>
> **Backlog.** Seul `imp-backlog` met à jour `BACKLOG.md`. Ce fichier ne l'écrit pas.

---

## D1 — Prochain paquet macOS à signature cohérente

| Spec | [S1](../specs/20261030162015-signature-macos-valide-specs.md) |
|:-----|:---|

> je veux que la vérification de signature réussisse sur le prochain paquet macOS, tout en restant distincte d'une distribution publique identifiée

### D1.1 — Signer le bundle macOS en ad hoc

**Statut** : ✅ Validé

#### Solution technique

Dans `src-tauri/tauri.conf.json`, ajouter `bundle.macOS.signingIdentity` avec la valeur `"-"`. C'est l'identité ad hoc de `codesign`. Au `tauri build`, le bundler signe alors le `.app` avant de construire le DMG. `hardenedRuntime` reste à son défaut, `true` : la signature porte le runtime durci.

Sans cette identité, `keychain()` ne reçoit ni certificat ni identité et ne signe pas. Le binaire garde la signature du linker (`adhoc,linker-signed`), `Contents/Resources` existe, et `codesign --verify` lève `errSecCSResourcesNotFound`. La signature ad hoc écrit `Contents/_CodeSignature/CodeResources` et fait réussir la vérification. `TeamIdentifier` reste vide, donc l'évaluation pour une distribution identifiée échoue encore.

La commande de build ne change pas. Aucun secret. La version 1.0.0 déjà publiée n'est pas reconstruite ni réémise.

Les suites automatiques du dépôt ne construisent pas ce bundle et ne lisent pas sa signature. Le contrôle de D1.1 est le scénario manuel du cahier.

#### Notes d'implémentation

`signingIdentity` est en place. Aucun test automatisé ne vise `tauri.conf.json` : contrôle ciblé NON_EXÉCUTÉ. La reconstruction locale du bundle est INDISPONIBLE : `cargo` et `rustc` sont absents de cette machine. Le scénario R1.1 reste à jouer là où la toolchain Rust est installée.

#### Code existant concerné

| Chemin | Rôle |
|:-------|:-----|
| `src-tauri/tauri.conf.json` | Configuration de bundle. Aucun bloc `macOS` aujourd'hui. |
| `.github/workflows/publish-release.yml` | Lance `npm run tauri build -- --bundles dmg` sans identité. La commande reste. |
| `.github/workflows/prepare-release.yml` | Même commande de bundle macOS expérimental. Elle reste. |

---

## D2 — Textes alignés sur ce niveau de signature

| Spec | [S2](../specs/20261030162015-signature-macos-valide-specs.md) |
|:-----|:---|

> je veux que la documentation et la note de la prochaine publication disent la vérité sur ce niveau de signature

### D2.1 — Aligner les textes sur la signature ad hoc

**Statut** : ✅ Validé

#### Solution technique

Deux phrases disent aujourd'hui que les installateurs ne sont pas signés. Après D1.1, c'est faux pour le paquet macOS du prochain build.

Dans `README.md`, remplacer « Les installateurs ne sont pas encore signés ni, pour macOS, notarisés. » par une phrase qui dit : le paquet macOS porte une signature ad hoc vérifiable ; il n'est pas notarié ; les installateurs Linux et Windows ne sont pas signés. La phrase suivante, sur l'absence de déploiement par une PR ou un push sur `main`, reste.

Dans `.github/workflows/publish-release.yml`, la note passée à `gh release create` dit « les installateurs ne sont pas signés ni notarisés ». La remplacer par : l'installateur macOS reste expérimental, son bundle porte une signature ad hoc vérifiable et n'est pas notarié, les installateurs Linux et Windows ne sont pas signés. Le reste de la note (génération après validation de `main`) reste.

`ACCEPTANCE.md`, `IMPLEMENTATION.md`, `SPEC.md`, `TECHNICAL.md` et `docs/DEVELOPMENT.md` disent que la validation fonctionnelle macOS est différée. Ces phrases restent vraies. On n'y touche pas.

#### Code existant concerné

| Chemin | Rôle |
|:-------|:-----|
| `README.md` | Phrase « Les installateurs ne sont pas encore signés ni, pour macOS, notarisés. » |
| `.github/workflows/publish-release.yml` | Note `--notes` de `gh release create`. |
