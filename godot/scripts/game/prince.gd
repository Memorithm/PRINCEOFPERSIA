# Le prince : machine à états, physique, interactions.
#
# Le répertoire de mouvements suit l'original de près — courir, glisser pour
# s'arrêter, faire demi-tour sur place, sauts debout et élancés, s'accroupir, pas
# prudent, se suspendre à une corniche, se hisser, descendre en rappel, et une
# garde d'escrime avec avance, retraite, botte et parade — avec les armes bonus
# posées par-dessus.

class_name Prince
extends RefCounted

enum S {
	STAND, TURN, RUN_START, RUN, RUN_STOP, STEP,
	CROUCH_IN, CROUCH, CROUCH_OUT,
	JUMP_UP, JUMP_RUN, FALL, LAND,
	HANG, CLIMB, CLIMB_DOWN,
	READY, ADVANCE, RETREAT, STRIKE, PARRY,
	HURT, DEAD, DRINK, THROW, CAST, LEAVING,
}

enum Melee { NONE, SWORD, SCIMITAR }

static func airborne(s: int) -> bool:
	return s == S.JUMP_UP or s == S.JUMP_RUN or s == S.FALL

## États pendant lesquels le prince ne peut rien entreprendre d'autre.
static func locked(s: int) -> bool:
	return s in [S.TURN, S.RUN_START, S.RUN_STOP, S.STEP, S.CROUCH_IN, S.CROUCH_OUT,
		S.LAND, S.CLIMB, S.CLIMB_DOWN, S.STRIKE, S.ADVANCE, S.RETREAT,
		S.HURT, S.DEAD, S.DRINK, S.THROW, S.CAST, S.LEAVING]

static func melee_damage(m: int) -> int:
	return [0, 1, 2][m]

## Distance à laquelle la pointe porte devant les hanches.
static func melee_reach(m: int) -> float:
	return [0.0, 25.0, 29.0][m]

static func melee_swing(m: int) -> float:
	return 0.78 if m == Melee.SCIMITAR else 1.0

## Probabilité qu'une parade adverse cède face à cette arme.
static func melee_pierce(m: int) -> float:
	return 0.35 if m == Melee.SCIMITAR else 0.0

static func melee_blade(m: int) -> int:
	return [Skel.Blade.NONE, Skel.Blade.SWORD, Skel.Blade.SCIMITAR][m]

static func melee_label(m: int) -> String:
	return ["à mains nues", "épée", "cimeterre"][m]

var p := Vector2.ZERO
var v := Vector2.ZERO
var facing := 1.0
var st := S.STAND
var t := 0.0
var hp := 3
var hp_max := 3
var armed := false
var melee := Melee.NONE
var sword := false
var scimitar := false
var buckler := false
var wand := false
var daggers := 0
var charges := 0
## Hauteur à laquelle la chute en cours a commencé.
var fall_from := 0.0
var float_t := 0.0
var swift_t := 0.0
var invuln := 0.0
var ledge := Vector2i.ZERO
var anchor := Vector2.ZERO
## Ce coup a déjà été résolu.
var struck := false
var step_to := 0.0
var throw_cd := 0.0
var cause := ""

# ---- présentation ----------------------------------------------------
## Pose figée à l'instant du changement d'état, dont on sort en fondu, pour
## qu'aucune transition ne claque.
var blend_from: Skel.Pose
var blend_t := Geom.BLEND
## Orientation lissée. Le signe reflète la figure, l'amplitude la comprime, si
## bien qu'un demi-tour passe par une pose de profil au lieu de basculer.
var facing_vis := 1.0
## Phase du cycle de course, avancée par la *distance parcourue* plutôt que par
## le temps, pour que les pieds ne patinent jamais quelle que soit la vitesse.
var gait := 0.0
## Touches mises en attente : une action demandée pendant une animation bloquée
## se déclenche dès que celle-ci lâche prise.
var buf_jump := 0.0
var buf_attack := 0.0

func _init(at: Vector2, face: float, carry: Dictionary) -> void:
	p = at
	facing = face
	facing_vis = face
	hp = carry.get("hp_max", 3)
	hp_max = hp
	sword = carry.get("sword", false)
	scimitar = carry.get("scimitar", false)
	buckler = carry.get("buckler", false)
	wand = carry.get("wand", false)
	daggers = carry.get("daggers", 0)
	melee = Melee.SCIMITAR if scimitar else (Melee.SWORD if sword else Melee.NONE)
	charges = 8 if wand else 0
	fall_from = at.y
	anchor = at
	step_to = at.x
	blend_from = Skel.rest()

func foot_tile() -> Vector2i:
	return Vector2i(Geom.tx_of(p.x), Geom.ty_of_feet(p.y))

func speed() -> float:
	return Geom.RUN_SPEED * (1.35 if swift_t > 0.0 else 1.0)

func carry() -> Dictionary:
	return {"hp_max": hp_max, "sword": sword, "scimitar": scimitar,
		"buckler": buckler, "wand": wand, "daggers": daggers}

## Le clip qui pilote l'état courant, et à quelle vitesse le jouer.
func clip_and_rate() -> Array:
	match st:
		S.STAND: return [Anim.get_clip("stand"), 1.0]
		S.TURN: return [Anim.get_clip("turn"), 1.0]
		S.RUN_START: return [Anim.get_clip("run_start"), 1.0]
		S.RUN: return [Anim.get_clip("run"), 1.3 if swift_t > 0.0 else 1.0]
		S.RUN_STOP: return [Anim.get_clip("run_stop"), 1.0]
		S.STEP: return [Anim.get_clip("step"), 1.0]
		S.CROUCH_IN: return [Anim.get_clip("crouch_in"), 1.0]
		S.CROUCH: return [Anim.get_clip("crouch"), 1.0]
		S.CROUCH_OUT: return [Anim.get_clip("crouch_in"), -1.0]
		S.JUMP_UP: return [Anim.get_clip("jump_up"), 1.0]
		S.JUMP_RUN: return [Anim.get_clip("jump_run"), 1.0]
		S.FALL: return [Anim.get_clip("fall"), 1.0]
		S.LAND: return [Anim.get_clip("land"), 1.0]
		S.HANG: return [Anim.get_clip("hang"), 1.0]
		S.CLIMB: return [Anim.get_clip("climb"), 1.0]
		S.CLIMB_DOWN: return [Anim.get_clip("climb"), -1.0]
		S.READY: return [Anim.get_clip("sword_ready"), 1.0]
		S.ADVANCE: return [Anim.get_clip("sword_adv"), 1.0]
		S.RETREAT: return [Anim.get_clip("sword_ret"), 1.0]
		S.STRIKE: return [Anim.get_clip("sword_strike"), melee_swing(melee)]
		S.PARRY: return [Anim.get_clip("sword_parry"), 1.0]
		S.HURT: return [Anim.get_clip("hurt"), 1.0]
		S.DEAD: return [Anim.get_clip("dead"), 1.0]
		S.DRINK: return [Anim.get_clip("drink"), 1.0]
		S.THROW: return [Anim.get_clip("throw"), 1.0]
		S.CAST: return [Anim.get_clip("cast"), 1.0]
		S.LEAVING: return [Anim.get_clip("bow"), 1.0]
	return [Anim.get_clip("stand"), 1.0]

func clip_total() -> float:
	var cr := clip_and_rate()
	var c: Anim.Clip = cr[0]
	return c.total() / maxf(absf(cr[1]), 0.01)

## Pose du moment, en fondu depuis celle sur laquelle l'état précédent s'est
## arrêté.
func pose() -> Skel.Pose:
	var cr := clip_and_rate()
	var c: Anim.Clip = cr[0]
	var rate: float = cr[1]
	# Les cycles avancent avec la distance, le reste avec le temps.
	var ct: float = gait if st == S.RUN else t
	var raw: Skel.Pose
	if rate < 0.0:
		raw = c.sample(maxf(c.total() - ct * -rate, 0.0))
	else:
		raw = c.sample(ct * rate)
	if blend_t >= Geom.BLEND:
		return raw
	return blend_from.blend(raw, Geom.smoothstep01(blend_t / Geom.BLEND))

func blade() -> int:
	if st == S.THROW:
		return Skel.Blade.DAGGER
	if st == S.CAST:
		return Skel.Blade.WAND
	if armed:
		return melee_blade(melee)
	return Skel.Blade.NONE

func enter(s: int) -> void:
	if st == s:
		return
	blend_from = pose()
	blend_t = 0.0
	st = s
	t = 0.0
	struck = false
