# Représentation d'un niveau et lecture des cartes ASCII.
#
# Un niveau est une grille de salles de 10 x 3 tuiles ; il peut faire n'importe
# quel nombre de salles de large et de haut. Chaque carte s'écrit en deux couches
# alignées : le terrain, et les liaisons (groupes reliant dalles, herses et
# portes ; chiffre d'adresse pour les gardes).

class_name Level
extends RefCounted

enum T {
	SPACE,       ## Rien — on tombe au travers.
	FLOOR,       ## Sol praticable.
	WALL,        ## Maçonnerie pleine.
	PILLAR,      ## Colonne : solide, mais on tient debout dessus.
	LOOSE,       ## Planche descellée : cède sous le poids.
	RUBBLE,      ## Ce qu'une planche descellée laisse derrière elle.
	SPIKES,      ## Piège à pointes.
	CHOMPER,     ## Lames de porte qui claquent en cadence.
	GATE,        ## Herse.
	PLATE_RAISE, ## Dalle de pression qui lève les herses liées.
	PLATE_DROP,  ## Dalle qui les laisse retomber.
	TORCH,       ## Torche murale (la case reste libre).
	MIRROR,      ## Miroir encadré.
	WINDOW,      ## Fenêtre à barreaux : projette un rai de lumière.
	ARCH,        ## Arc décoratif.
	BONES,       ## Restes d'un visiteur précédent.
	EXIT,        ## La sortie.
}

enum Item {
	POTION_HEAL, POTION_LIFE, POTION_FLOAT, POTION_POISON, POTION_SWIFT,
	SWORD, DAGGERS, WAND, BUCKLER, SCIMITAR,
}

enum Mob { GUARD, FAT, SKELETON, SHADOW, VIZIER, JAFFAR, PRINCESS }

static func tile_solid(t: int) -> bool:
	return t == T.WALL or t == T.PILLAR

static func tile_walkable(t: int) -> bool:
	return t in [T.FLOOR, T.LOOSE, T.RUBBLE, T.SPIKES, T.PLATE_RAISE,
		T.PLATE_DROP, T.BONES, T.EXIT, T.GATE, T.CHOMPER]

static func item_is_potion(k: int) -> bool:
	return k <= Item.POTION_SWIFT

static func item_colour(k: int) -> Color:
	match k:
		Item.POTION_HEAL: return Color8(214, 40, 62)
		Item.POTION_LIFE: return Color8(232, 74, 148)
		Item.POTION_FLOAT: return Color8(72, 186, 226)
		Item.POTION_POISON: return Color8(120, 216, 92)
		Item.POTION_SWIFT: return Color8(238, 206, 74)
		_: return Color8(200, 206, 214)

static func item_label(k: int) -> String:
	match k:
		Item.POTION_HEAL: return "Potion de vie"
		Item.POTION_LIFE: return "Élixir de vigueur"
		Item.POTION_FLOAT: return "Potion de plume"
		Item.POTION_POISON: return "Poison !"
		Item.POTION_SWIFT: return "Potion de célérité"
		Item.SWORD: return "Épée"
		Item.DAGGERS: return "Dagues de jet"
		Item.WAND: return "Bâton de flamme"
		Item.BUCKLER: return "Bouclier"
		Item.SCIMITAR: return "Cimeterre du Vizir"
	return ""

static func mob_name(k: int) -> String:
	match k:
		Mob.GUARD: return "Garde"
		Mob.FAT: return "Geôlier"
		Mob.SKELETON: return "Squelette"
		Mob.SHADOW: return "L'Ombre"
		Mob.VIZIER: return "Le Vizir"
		Mob.JAFFAR: return "Jaffar"
		Mob.PRINCESS: return "La Princesse"
	return ""

static func mob_hostile(k: int) -> bool:
	return k != Mob.PRINCESS

# ---------------------------------------------------------------- données

var name := ""
var hint := ""
var theme := {}
var tw := 0
var th := 0
var rw := 0
var rh := 0
var tiles: PackedInt32Array
var groups: PackedByteArray
var items: Array = []   ## [{kind, tx, ty}]
var mobs: Array = []    ## [{kind, tx, ty, skill, facing}]
var start := Vector2i.ZERO
var start_face := 1.0
var exit_at := Vector2i.ZERO
var time := 900

const _TILE_OF := {
	" ": T.SPACE, ".": T.SPACE, "=": T.FLOOR, "#": T.WALL, "|": T.PILLAR,
	"b": T.LOOSE, ":": T.RUBBLE, "^": T.SPIKES, "V": T.CHOMPER, "G": T.GATE,
	"p": T.PLATE_RAISE, "o": T.PLATE_DROP, "t": T.TORCH, "m": T.MIRROR,
	"w": T.WINDOW, "A": T.ARCH, "n": T.BONES,
}
const _ITEM_OF := {
	"h": Item.POTION_HEAL, "H": Item.POTION_LIFE, "f": Item.POTION_FLOAT,
	"x": Item.POTION_POISON, "q": Item.POTION_SWIFT, "s": Item.SWORD,
	"D": Item.DAGGERS, "F": Item.WAND, "C": Item.BUCKLER, "M": Item.SCIMITAR,
}
const _MOB_OF := {
	"g": Mob.GUARD, "z": Mob.FAT, "k": Mob.SKELETON, "S": Mob.SHADOW,
	"J": Mob.JAFFAR, "Y": Mob.VIZIER, "P": Mob.PRINCESS,
}

static func _group_of(c: String) -> int:
	if c >= "1" and c <= "9":
		return c.unicode_at(0) - 48
	if c >= "a" and c <= "z":
		return 10 + c.unicode_at(0) - 97
	if c >= "A" and c <= "Z":
		return 40 + c.unicode_at(0) - 65
	return 0

## Lit une définition du tableau `Levels.CAMPAIGN`. Renvoie une chaîne d'erreur
## vide si tout va bien.
func parse(def: Dictionary) -> String:
	name = def["name"]
	hint = def["hint"]
	theme = Themes.by_name(def["theme"])
	time = def["time"]
	var rows: Array = def["rows"]
	var links: Array = def["links"]
	th = rows.size()
	if th == 0:
		return "%s : carte vide" % name
	tw = rows[0].length()
	if th % Geom.ROOM_TH != 0:
		return "%s : %d rangées, doit être un multiple de %d" % [name, th, Geom.ROOM_TH]
	if tw % Geom.ROOM_TW != 0:
		return "%s : %d colonnes, doit être un multiple de %d" % [name, tw, Geom.ROOM_TW]
	rw = tw / Geom.ROOM_TW
	rh = th / Geom.ROOM_TH

	tiles.resize(tw * th)
	groups.resize(tw * th)
	items.clear()
	mobs.clear()
	var has_start := false
	var has_exit := false

	for y in th:
		var row: String = rows[y]
		var link: String = links[y] if y < links.size() else ""
		if row.length() != tw:
			return "%s : rangée %d fait %d colonnes, attendu %d" % [name, y, row.length(), tw]
		for x in tw:
			var ch := row[x]
			var lc := link[x] if x < link.length() else "."
			var t := -1
			var item := -1
			var mob := -1
			if _TILE_OF.has(ch):
				t = _TILE_OF[ch]
			elif ch == "X":
				t = T.EXIT
				exit_at = Vector2i(x, y)
				has_exit = true
			elif ch == "@":
				t = T.FLOOR
				start = Vector2i(x, y)
				has_start = true
				if lc == "<":
					start_face = -1.0
			elif _ITEM_OF.has(ch):
				t = T.FLOOR
				item = _ITEM_OF[ch]
			elif _MOB_OF.has(ch):
				t = T.FLOOR
				mob = _MOB_OF[ch]
			else:
				return "%s : caractère inconnu %s en (%d, %d)" % [name, ch, x, y]

			var i := y * tw + x
			tiles[i] = t
			if t in [T.GATE, T.PLATE_RAISE, T.PLATE_DROP, T.EXIT]:
				groups[i] = _group_of(lc)
			if item >= 0:
				items.append({"kind": item, "tx": x, "ty": y})
			if mob >= 0:
				var skill := 3
				if lc >= "0" and lc <= "9":
					skill = lc.unicode_at(0) - 48
				mobs.append({
					"kind": mob, "tx": x, "ty": y, "skill": skill,
					"facing": 1.0 if lc == ">" else -1.0,
				})

	if not has_start:
		return "%s : pas de case de départ '@'" % name
	if not has_exit:
		return "%s : pas de sortie 'X'" % name
	return ""

func in_bounds(tx: int, ty: int) -> bool:
	return tx >= 0 and ty >= 0 and tx < tw and ty < th

## Hors carte, c'est de la roche : rien ne peut sortir du niveau.
func tile(tx: int, ty: int) -> int:
	if not in_bounds(tx, ty):
		return T.WALL
	return tiles[ty * tw + tx]

func group(tx: int, ty: int) -> int:
	if not in_bounds(tx, ty):
		return 0
	return groups[ty * tw + tx]

func set_tile(tx: int, ty: int, t: int) -> void:
	if in_bounds(tx, ty):
		tiles[ty * tw + tx] = t

func room_count() -> int:
	return rw * rh

## Salles contenant au moins une case non pleine — celles où l'on peut être.
func playable_rooms() -> int:
	var n := 0
	for ry in rh:
		for rx in rw:
			for y in Geom.ROOM_TH:
				var found := false
				for x in Geom.ROOM_TW:
					var t := tile(rx * Geom.ROOM_TW + x, ry * Geom.ROOM_TH + y)
					if t != T.WALL and t != T.SPACE:
						n += 1
						found = true
						break
				if found:
					break
	return n
