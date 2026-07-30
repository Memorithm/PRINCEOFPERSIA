# Prince of Persia — version Godot

Le jeu tourne sur **Godot 4.3**. Aucun asset binaire : décor, personnages,
interface et bruitages sont produits par le code.

## Lancer

```sh
godot --path godot            # depuis la racine du dépôt
```

ou bien ouvrir `godot/project.godot` dans l'éditeur et appuyer sur F5.

## Commandes

| touche | action | touche | action |
|---|---|---|---|
| ← → | courir | Maj + ← → | pas prudent |
| ↑ | sauter, se hisser | ↓ | s'accroupir, descendre en rappel |
| Espace / X | frapper | Z / Alt | parer |
| T | lancer une dague | F | bâton de flamme |
| C | rengainer | V | cadrage salle / suivi |
| R | recommencer le niveau | Échap / P | pause |
| F11 | plein écran | | |

Maintenir **Maj** en courant vers un vide fait attraper la corniche que l'on
vient de quitter : c'est le rattrapage emblématique de l'original.

## Architecture

```
scripts/core/    géométrie et constantes, thèmes, cartes, analyse de niveau,
                 état animé des cases, instantané des commandes, particules
scripts/art/     silhouettes et ombrage cel, squelettes, clips d'animation,
                 décor, objets
scripts/game/    le prince, les gardes, le monde (simulation + rendu)
scripts/nodes/   point d'entrée, interface, calques, bruitages
shaders/         étalonnage final (vignette, grain, éclair, fondu)
```

Le monde est en **pixels d'art** : une tuile fait 32 × 40, une salle 10 × 3
tuiles, exactement comme sur Apple II. La caméra Godot met ces unités à l'échelle
de la fenêtre, si bien qu'aucune routine de dessin ne connaît la résolution.

La simulation tourne à **pas fixe de 120 Hz** avec un accumulateur : un saut
franchit la même distance sur une machine à 30 images/s et sur une à 240.

## Rendu

Trois calques, dans cet ordre :

1. **le monde** — décor, objets, personnages, matière. Assombri par un
   `CanvasModulate` réglé sur l'ambiance du niveau, rallumé par un `PointLight2D`
   par torche ;
2. **l'émissif** — flammes, étincelles, halos, traits de lame. Dans son propre
   `CanvasLayer` qui suit la caméra mais échappe à l'ambiance : une flamme reste
   une flamme dans une salle noire ;
3. **l'étalonnage** — vignette, grain, éclair de dégât, fondu.

Les personnages sont dessinés, pas assemblés : chaque os porte une silhouette
écrite (`scripts/art/shape.gd`) qui enfle au deltoïde et se pince au coude, chacun
est ombré en cel avec une ombre propre à bord franc, têtes et mains sont des
polygones écrits à la main, et un contour dessiné sépare les formes de même
couleur qui se recouvrent. La silhouette entière est repeinte dilatée avant la
figure, ce qui détache un personnage sombre d'un mur sombre.

## Test d'intégration

```sh
godot --path godot --headless -- --selftest
```

Charge les six niveaux, vérifie leur cohérence et joue quinze secondes de
simulation sur chacun avec des commandes tirées au sort. Sort en erreur si un
niveau ne se charge pas.

## Captures

```sh
godot --path godot --rendering-driver opengl3 -- \
      --shot art.png --level 3 --at 24,7 --frames 90 --follow
```
