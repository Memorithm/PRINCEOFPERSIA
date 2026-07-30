# Métrique du monde et constantes de simulation.
#
# Tout est exprimé en « pixels d'art » : une tuile fait 32 x 40, une salle
# 10 x 3 tuiles exactement comme sur Apple II. La caméra Godot met ces unités à
# l'échelle de la fenêtre, donc rien dans le jeu ne connaît la résolution.
#
#            tx*32                      (tx+1)*32
#   ty*40    +-------------------------------+
#            |                               |   espace libre de la case
#            |     un personnage debout ici  |
# (ty+1)*40  +===============================+  <- surface d'appui
#            |///// dalle du sol (9 px) /////|     dessinée dans la case du bas
#
# La dalle d'une rangée sert de corniche à la rangée du dessous : c'est ainsi
# que l'art original est découpé.

class_name Geom

const TILE_W := 32.0
const TILE_H := 40.0
## Épaisseur de la dalle sous la surface d'appui d'une case.
const FLOOR_H := 9.0

const ROOM_TW := 10
const ROOM_TH := 3
const ROOM_W := TILE_W * ROOM_TW
const ROOM_H := TILE_H * ROOM_TH

## De combien le corps descend sous une corniche à laquelle on est suspendu.
## Doit valoir exactement la portée bras tendus du prince (Skel.Prop.reach_up),
## sinon les mains flottent au-dessus de la prise.
const HANG_DROP := 31.0

# ---------------------------------------------------------------- physique

const GRAVITY := 430.0
const RUN_SPEED := 80.0
const JUMP_UP_VY := -178.0
## Un saut élancé doit franchir trois cases depuis n'importe quel point de la
## dernière tuile : l'arc est long et plat plutôt qu'au pixel près, et il utilise
## sa propre gravité, plus douce, pour ne pas monter dans le plafond.
const JUMP_RUN_VY := -160.0
const JUMP_RUN_VX := 126.0
const GRAVITY_JUMP := 300.0
## Demi-largeur du corps pour les collisions murales.
const BODY_HW := 6.5
## En dessous de cette hauteur, une chute ne coûte rien.
const FALL_SAFE := 58.0
## À partir de cette hauteur, elle tue.
const FALL_LETHAL := 112.0

const GATE_HOLD := 7.0
const GATE_RISE := 5.0
const GATE_FALL := 0.9
const CHOMP_PERIOD := 2.1
const LOOSE_FUSE := 0.62

# ---------------------------------------------------------------- animation

## Durée d'un fondu entre deux poses.
const BLEND := 0.085
## Distance couverte par un cycle de course complet.
const STRIDE_PX := 36.0
const RUN_CYCLE := 0.45
const GUARD_STRIDE := 21.0
const WALK_CYCLE := 1.02
## Durée pendant laquelle une touche reste en mémoire tampon.
const BUFFER := 0.18

# ---------------------------------------------------------------- géométrie

## Surface d'appui (y monde) d'un personnage dans la rangée `ty`.
static func surf(ty: int) -> float:
	return float(ty + 1) * TILE_H - FLOOR_H

## Centre x de la colonne `tx`.
static func cx(tx: int) -> float:
	return (float(tx) + 0.5) * TILE_W

static func tx_of(x: float) -> int:
	return int(floor(x / TILE_W))

## Rangée dont l'intérieur contient y.
static func ty_of(y: float) -> int:
	return int(floor(y / TILE_H))

## Rangée occupée par un personnage dont les pieds sont en y.
static func ty_of_feet(y: float) -> int:
	return int(floor(y / TILE_H))

static func room_of(tx: int, ty: int) -> Vector2i:
	return Vector2i(floori(float(tx) / ROOM_TW), floori(float(ty) / ROOM_TH))

## Rectangle monde d'une salle.
static func room_rect(room: Vector2i) -> Rect2:
	return Rect2(room.x * ROOM_W, room.y * ROOM_H, ROOM_W, ROOM_H)

# ---------------------------------------------------------------- outils

static func approach(cur: float, target: float, step: float) -> float:
	if cur < target:
		return minf(cur + step, target)
	return maxf(cur - step, target)

static func ease_out(t: float) -> float:
	var u := clampf(t, 0.0, 1.0)
	return 1.0 - pow(1.0 - u, 3.0)

static func smoothstep01(t: float) -> float:
	var u := clampf(t, 0.0, 1.0)
	return u * u * (3.0 - 2.0 * u)

## Multiplie la luminosité d'une couleur sans toucher à son alpha.
static func shade(c: Color, k: float) -> Color:
	return Color(minf(c.r * k, 1.0), minf(c.g * k, 1.0), minf(c.b * k, 1.0), c.a)

## Désature vers le gris.
static func desat(c: Color, amount: float) -> Color:
	var l := c.r * 0.299 + c.g * 0.587 + c.b * 0.114
	return c.lerp(Color(l, l, l, c.a), amount)

## Bruit 1D lisse et déterministe — flamme des torches, tremblement de caméra.
static func noise1(t: float, seed_i: int) -> float:
	var i: float = floor(t)
	var f: float = t - i
	var a: float = _hashf(int(i), seed_i) * 2.0 - 1.0
	var b: float = _hashf(int(i) + 1, seed_i) * 2.0 - 1.0
	return lerpf(a, b, smoothstep01(f))

static func _hashf(x: int, y: int) -> float:
	var h: int = (x * 0x9E3779B1) ^ (y * 0x85EBCA77)
	h = h ^ (h >> 15)
	h = h * 0x2545F491
	h = h ^ (h >> 13)
	return float(absi(h) % 16777216) / 16777216.0

## Hachage stable 2D — même tuile, même appareil de briques.
static func hashf(x: int, y: int, salt: int) -> float:
	return _hashf(x * 31 + salt, y)
