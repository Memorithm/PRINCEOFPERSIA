# Prince of Persia — la légende du miroir d'argent (Godot 4)

L'aventure vue du dessus, **entièrement en Godot 4.3** : quinze mondes liés,
quête des quatre fragments du Sceau, rendu vectoriel lisse (MSAA 2×), lumières
additives temps réel, HUD vectoriel.

## Lancer

```sh
godot --path adventure-godot
```

(ou ouvrir le dossier comme projet dans l'éditeur Godot 4.3+ et appuyer sur ▶)

## Commandes

| Touche | Effet |
|---|---|
| `←` `↑` `→` `↓` ou `ZQSD`/`WASD` | marcher (diagonales comprises) |
| `Espace` / `X` | frapper — maintenir pour enchaîner les coups |
| `+` / `-` | élargir / rapprocher la vue |
| `P` | pause · `R` / `Entrée` recommencer le monde |
| `Échap` / `Q` | quitter |

## La quête

Le Vizir Ganar a brisé le Sceau de Lumière en **4 fragments**, confiés aux boss
des donjons (Brute, Garde royal, Golem de cristal, Pharaon). Reunis, ils ouvrent
la porte dorée du Sanctuaire du Miroir → Tour du Vizir → Trône des Ténèbres.
Le **Miroir d'argent** (Palais) réveille les dalles `M` qui basculent entre la
Vallée et son Ombre ; l'**Épée d'argent** (Mines) frappe deux fois plus fort ;
la **Fée** de l'Oasis soigne ; un **conteneur de cœur** se cache au nord du lac.

## Architecture

```
project.godot    config 4.3, entrées clavier, MSAA 2D
main.tscn        scène racine
main.gd          simulation (pas fixe 120 Hz) + rendu vectoriel + lumières
hud.gd           HUD : cœurs, compteurs, fragments, messages, fondus
maps_data.gd     les 15 mondes — GÉNÉRÉS depuis src/adventure (Rust)
```

`maps_data.gd` est régénéré depuis le moteur Rust (cartes, portails, départs,
palettes) — la source de vérité reste `tools/genmaps.py` :

```sh
python3 tools/genmaps.py   # régénère src/adventure/world_data.rs
# puis reporter cartes/portails/palettes dans adventure-godot/maps_data.gd
```
