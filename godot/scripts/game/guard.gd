# Les gardes : ronde, engagement, escrime.
#
# L'adresse (0–9, fixée par garde dans la couche de liaisons du niveau) pilote la
# fréquence des bottes, la fiabilité des parades et le temps de réaction — les
# trois mêmes réglages que l'original.

class_name Guard
extends RefCounted

enum S { IDLE, PATROL, READY, ADVANCE, RETREAT, STRIKE, PARRY, HURT, DEAD, FALLING }

var kind := Level.Mob.GUARD
var p := Vector2.ZERO
var v := Vector2.ZERO
var facing := -1.0
var hp := 3
var hp_max := 3
var skill := 3
var st := S.IDLE
var t := 0.0
var home := Vector2.ZERO
var dir := -1.0
## Temps avant la prochaine décision d'escrime.
var cool := 0.4
## Depuis combien de temps ce garde a conscience du prince.
var alert := 0.0
var stagger := 0.0
var struck := false
var idle := 0.0
var blend_from: Skel.Pose
var blend_t := Geom.BLEND
var facing_vis := -1.0
var gait := 0.0

func _init(spec: Dictionary) -> void:
	kind = spec["kind"]
	p = Vector2(Geom.cx(spec["tx"]), Geom.surf(spec["ty"]))
	facing = spec["facing"]
	facing_vis = facing
	dir = facing
	skill = spec["skill"]
	home = p
	blend_from = Skel.rest()
	match kind:
		Level.Mob.GUARD: hp = 3 + skill / 4
		Level.Mob.FAT: hp = 5
		Level.Mob.SKELETON: hp = 4
		Level.Mob.SHADOW: hp = 4
		Level.Mob.VIZIER: hp = 7
		Level.Mob.JAFFAR: hp = 9
		Level.Mob.PRINCESS: hp = 1
	hp_max = hp

func foot_tile() -> Vector2i:
	return Vector2i(Geom.tx_of(p.x), Geom.ty_of_feet(p.y))

func hostile() -> bool:
	return Level.mob_hostile(kind)

func melee() -> int:
	match kind:
		Level.Mob.FAT, Level.Mob.VIZIER, Level.Mob.JAFFAR:
			return Prince.Melee.SCIMITAR
		Level.Mob.PRINCESS:
			return Prince.Melee.NONE
	return Prince.Melee.SWORD

func patrols() -> bool:
	return kind not in [Level.Mob.JAFFAR, Level.Mob.SHADOW, Level.Mob.VIZIER, Level.Mob.PRINCESS]

func strike_p() -> float:
	return 0.18 + skill * 0.072

func parry_p() -> float:
	return 0.12 + skill * 0.082

func react() -> float:
	return maxf(0.78 - skill * 0.058, 0.16)

func walk_speed() -> float:
	match kind:
		Level.Mob.FAT: return 26.0
		Level.Mob.JAFFAR: return 46.0
		Level.Mob.VIZIER: return 48.0
		Level.Mob.SKELETON: return 40.0
		Level.Mob.PRINCESS: return 18.0
	return 34.0

func prop() -> Skel.Prop:
	var base := Skel.prince_prop()
	match kind:
		Level.Mob.GUARD: return base.scaled(1.02, 1.10)
		Level.Mob.FAT: return base.scaled(0.98, 1.55)
		Level.Mob.SKELETON: return base.scaled(1.0, 0.74)
		Level.Mob.SHADOW: return base.scaled(1.0, 1.0)
		Level.Mob.VIZIER: return base.scaled(1.04, 1.06)
		Level.Mob.JAFFAR: return base.scaled(1.06, 1.14)
		Level.Mob.PRINCESS: return base.scaled(0.97, 0.92)
	return base

func style() -> Skel.Style:
	var s := Skel.prince_style()
	match kind:
		# Un turban, un gilet ouvert sur des bras nus, un pantalon olive bouffant
		# et des bottes lacées.
		Level.Mob.GUARD:
			s.skin = Color8(198, 146, 100); s.skin_dk = Color8(132, 88, 56)
			s.sash = Color8(120, 44, 40); s.sash_dk = Color8(72, 24, 24)
			s.hair = Color8(32, 24, 22); s.boot = Color8(146, 116, 76)
			s.trouser = Color8(130, 120, 56); s.baggy = 0.9
			s.head_wrap = Color8(190, 88, 40); s.vest = Color8(112, 70, 42)
			s.belt = true; s.band = null
		# Le geôlier : plus large, turban pâle, cimeterre plus lourd.
		Level.Mob.FAT:
			s.skin = Color8(206, 158, 114); s.skin_dk = Color8(140, 94, 62)
			s.sash = Color8(158, 44, 40); s.sash_dk = Color8(94, 24, 26)
			s.boot = Color8(118, 84, 50); s.trouser = Color8(150, 132, 92)
			s.baggy = 0.95; s.head_wrap = Color8(214, 206, 186)
			s.vest = Color8(96, 76, 52); s.belt = true; s.band = null
		Level.Mob.SKELETON:
			s.bones = true; s.head_wrap = null; s.band = null
		# Magenta, sans poids, un long ruban derrière : l'apparition, pas un
		# recoloriage du prince.
		Level.Mob.SHADOW:
			s.skin = Color8(196, 128, 178); s.skin_dk = Color8(112, 60, 108)
			s.sash = Color8(228, 150, 196); s.sash_dk = Color8(150, 74, 126)
			s.hair = Color8(52, 26, 62); s.boot = Color8(94, 58, 104)
			s.trouser = Color8(224, 206, 232); s.baggy = 0.85
			s.head_wrap = Color8(96, 44, 108); s.scarf = Color8(232, 128, 176)
			s.band = null; s.outline = Color8(28, 12, 34)
		# Le Vizir : robe crème, cheveux clairs, pas de turban.
		Level.Mob.VIZIER:
			s.skin = Color8(214, 168, 128); s.skin_dk = Color8(148, 102, 70)
			s.cloth = Color8(232, 224, 200); s.cloth_dk = Color8(158, 146, 120)
			s.sash = Color8(178, 156, 96); s.sash_dk = Color8(110, 94, 52)
			s.hair = Color8(196, 158, 84); s.boot = Color8(122, 96, 62)
			s.trouser = Color8(214, 206, 184); s.baggy = 0.5
			s.bare_chest = false; s.robe = 1.0; s.head_wrap = null; s.band = null
		Level.Mob.JAFFAR:
			s.skin = Color8(186, 138, 100); s.skin_dk = Color8(128, 86, 58)
			s.cloth = Color8(64, 46, 104); s.cloth_dk = Color8(30, 22, 54)
			s.sash = Color8(212, 172, 76); s.sash_dk = Color8(140, 106, 34)
			s.trouser = Color8(52, 38, 84); s.baggy = 0.4
			s.bare_chest = false; s.head_wrap = Color8(34, 26, 58)
			s.plume = Color8(206, 58, 58); s.robe = 1.0; s.band = null
		# Coiffe bleue, corsage rouge, pantalon blanc, souliers rouges.
		Level.Mob.PRINCESS:
			s.skin = Color8(238, 194, 154); s.skin_dk = Color8(168, 114, 82)
			s.cloth = Color8(198, 46, 62); s.cloth_dk = Color8(122, 24, 38)
			s.sash = Color8(226, 206, 132); s.sash_dk = Color8(154, 132, 70)
			s.hair = Color8(38, 28, 34); s.boot = Color8(198, 40, 56)
			s.trouser = Color8(240, 238, 232); s.baggy = 0.75
			s.bare_chest = false; s.robe = 0.34
			s.head_wrap = Color8(72, 118, 196); s.band = null
	return s

func clip_and_rate() -> Array:
	match st:
		S.IDLE: return [Anim.get_clip("stand"), 1.0]
		S.PATROL: return [Anim.get_clip("walk"), 1.0]
		S.READY: return [Anim.get_clip("sword_ready"), 1.0]
		S.ADVANCE: return [Anim.get_clip("sword_adv"), 1.0]
		S.RETREAT: return [Anim.get_clip("sword_ret"), 1.0]
		S.STRIKE: return [Anim.get_clip("sword_strike"), Prince.melee_swing(melee())]
		S.PARRY: return [Anim.get_clip("sword_parry"), 1.0]
		S.HURT: return [Anim.get_clip("hurt"), 1.0]
		S.DEAD: return [Anim.get_clip("dead"), 1.0]
		S.FALLING: return [Anim.get_clip("fall"), 1.0]
	return [Anim.get_clip("stand"), 1.0]

func clip_total() -> float:
	var cr := clip_and_rate()
	var c: Anim.Clip = cr[0]
	return c.total() / maxf(absf(cr[1]), 0.01)

func pose() -> Skel.Pose:
	var cr := clip_and_rate()
	var c: Anim.Clip = cr[0]
	var u: float = gait if st == S.PATROL else t
	var raw := c.sample(u * cr[1])
	if blend_t >= Geom.BLEND:
		return raw
	return blend_from.blend(raw, Geom.smoothstep01(blend_t / Geom.BLEND))

func blade() -> int:
	if st in [S.READY, S.ADVANCE, S.RETREAT, S.STRIKE, S.PARRY]:
		return Prince.melee_blade(melee())
	if kind == Level.Mob.SKELETON:
		return Skel.Blade.SWORD
	return Skel.Blade.NONE

## Change d'état en figeant la pose sortante, pour que le passage se fonde.
func enter(s: int) -> void:
	if st == s:
		return
	blend_from = pose()
	blend_t = 0.0
	st = s
	t = 0.0
