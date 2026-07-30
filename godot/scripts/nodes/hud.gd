# L'interface : fioles de vie, sablier, nom du niveau, messages, jauge de duel,
# et les écrans de titre, de pause et de fin.
#
# Tout est dessiné à la main plutôt que composé de Control : à cette densité, une
# seule routine de dessin se relit mieux qu'une arborescence de nœuds, et
# l'ensemble reste net à toutes les résolutions.

class_name HUD
extends Control

var world: World
var screen := "title"       ## title | play | pause | dead | won | intro
var intro_t := 0.0
var menu_index := 0

var _font: Font
var _title_font: Font

func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	_font = ThemeDB.fallback_font
	_title_font = ThemeDB.fallback_font
	set_anchors_preset(Control.PRESET_FULL_RECT)

func _process(_dt: float) -> void:
	# Un Control posé nu dans un CanvasLayer ne se redimensionne pas tout seul.
	size = get_viewport_rect().size
	queue_redraw()

func _draw() -> void:
	var vp := size
	if vp.x < 8.0 or vp.y < 8.0:
		return
	match screen:
		"title": _draw_title(vp)
		"intro": _draw_intro(vp)
		"play": _draw_play(vp)
		"pause": _draw_play(vp); _draw_pause(vp)
		"dead": _draw_play(vp); _draw_dead(vp)
		"won": _draw_won(vp)

func _txt(s: String, at: Vector2, sz_in: int, col: Color, centred := false) -> void:
	# Une fenêtre peut avoir une hauteur nulle le temps d'une image au démarrage :
	# une taille de police de zéro fait hurler le serveur de texte.
	var sz := maxi(sz_in, 6)
	var w := _font.get_string_size(s, HORIZONTAL_ALIGNMENT_LEFT, -1, sz).x
	var p := at
	if centred:
		p.x -= w * 0.5
	# Une ombre portée d'un pixel : sans elle, le texte clair disparaît sur les
	# murs clairs du palais.
	draw_string(_font, p + Vector2(1.5, 1.5), s, HORIZONTAL_ALIGNMENT_LEFT, -1, sz,
		Color(0, 0, 0, 0.75))
	draw_string(_font, p, s, HORIZONTAL_ALIGNMENT_LEFT, -1, sz, col)

# ---------------------------------------------------------------- en jeu

func _draw_play(vp: Vector2) -> void:
	if world == null or world.lv == null:
		return
	var bar_h := vp.y * 0.155
	var top := vp.y - bar_h
	draw_rect(Rect2(0, top, vp.x, bar_h), Color(0.04, 0.035, 0.055, 0.88))
	draw_rect(Rect2(0, top, vp.x, 2.0), Color(0.55, 0.45, 0.28, 0.5))

	var pad := vp.x * 0.025
	var mid := top + bar_h * 0.42
	var s := int(bar_h * 0.26)

	# --- fioles de vie ---------------------------------------------------
	var pl := world.player
	var fx0 := pad
	for i in pl.hp_max:
		var full := i < pl.hp
		_flask(Vector2(fx0 + i * (bar_h * 0.30), mid), bar_h * 0.24, full)
	_txt("%d/%d" % [pl.hp, pl.hp_max],
		Vector2(fx0 + pl.hp_max * (bar_h * 0.30) + 6.0, mid + s * 0.35), s,
		Color(0.85, 0.82, 0.78))

	# --- sablier ---------------------------------------------------------
	var m := int(world.clock) / 60
	var sec := int(world.clock) % 60
	var urgent := world.clock < 60.0
	var tcol := Color(0.95, 0.4, 0.35) if urgent else Color(0.92, 0.88, 0.80)
	_txt("%d:%02d" % [m, sec], Vector2(vp.x * 0.5, mid + s * 0.4), int(s * 1.25),
		tcol, true)

	# --- arme et munitions ------------------------------------------------
	var right := vp.x - pad
	var bits: Array[String] = []
	bits.append(Prince.melee_label(pl.melee) if pl.armed else "rengainée")
	if pl.daggers > 0:
		bits.append("%d dagues" % pl.daggers)
	if pl.wand:
		bits.append("%d charges" % pl.charges)
	if pl.buckler:
		bits.append("bouclier")
	if pl.swift_t > 0.0:
		bits.append("célérité %ds" % int(pl.swift_t))
	if pl.float_t > 0.0:
		bits.append("plume %ds" % int(pl.float_t))
	var line := " · ".join(bits)
	var lw := _font.get_string_size(line, HORIZONTAL_ALIGNMENT_LEFT, -1, maxi(s, 6)).x
	_txt(line, Vector2(right - lw, mid + s * 0.35), s, Color(0.80, 0.78, 0.72))

	# --- niveau ------------------------------------------------------------
	var sub := int(s * 0.82)
	_txt("%d. %s" % [world.idx + 1, world.lv.name],
		Vector2(pad, top + bar_h * 0.84), sub, Color(0.62, 0.58, 0.55))
	var room_s := "salle %d,%d" % [world.cam_room.x + 1, world.cam_room.y + 1]
	var rw := _font.get_string_size(room_s, HORIZONTAL_ALIGNMENT_LEFT, -1, maxi(sub, 6)).x
	_txt(room_s, Vector2(right - rw, top + bar_h * 0.84), sub, Color(0.50, 0.47, 0.45))

	# --- jauge de duel ------------------------------------------------------
	var b := world.boss()
	if not b.is_empty():
		var bw := vp.x * 0.34
		var bx := (vp.x - bw) * 0.5
		var by := vp.y * 0.045
		draw_rect(Rect2(bx - 2, by - 2, bw + 4, 12), Color(0, 0, 0, 0.6))
		draw_rect(Rect2(bx, by, bw * b[1], 8), Color(0.78, 0.20, 0.22))
		_txt(b[0], Vector2(vp.x * 0.5, by - 6), int(s * 0.9), Color(0.9, 0.82, 0.78), true)

	# --- message -----------------------------------------------------------
	if world.msg_t > 0.0:
		var a := clampf(world.msg_t, 0.0, 1.0)
		var col := Color(0.98, 0.62, 0.5, a) if world.msg_warn else Color(0.94, 0.90, 0.80, a)
		# Les messages vont en haut : au-dessus de la barre, ils se posaient en
		# travers du personnage, qui est exactement ce qu'on regarde en les lisant.
		_txt(world.msg_text, Vector2(vp.x * 0.5, vp.y * 0.085), int(s * 1.05), col, true)

func _flask(c: Vector2, r: float, full: bool) -> void:
	var glass := Color(0.72, 0.76, 0.80, 0.75)
	var body := PackedVector2Array([
		c + Vector2(-r * 0.62, r),
		c + Vector2(-r * 0.72, -r * 0.1),
		c + Vector2(-r * 0.28, -r * 0.75),
		c + Vector2(-r * 0.22, -r * 1.15),
		c + Vector2(r * 0.22, -r * 1.15),
		c + Vector2(r * 0.28, -r * 0.75),
		c + Vector2(r * 0.72, -r * 0.1),
		c + Vector2(r * 0.62, r),
	])
	Shape.poly(self, body, Color(0.14, 0.13, 0.18, 0.9))
	if full:
		Shape.poly(self, PackedVector2Array([
			c + Vector2(-r * 0.52, r * 0.9),
			c + Vector2(-r * 0.62, -r * 0.08),
			c + Vector2(-r * 0.24, -r * 0.62),
			c + Vector2(r * 0.24, -r * 0.62),
			c + Vector2(r * 0.62, -r * 0.08),
			c + Vector2(r * 0.52, r * 0.9),
		]), Color(0.82, 0.16, 0.24))
	draw_polyline(body + PackedVector2Array([body[0]]), glass, 1.5, true)

# ---------------------------------------------------------------- écrans

func _panel(vp: Vector2, h: float) -> Rect2:
	var r := Rect2(vp.x * 0.14, (vp.y - h) * 0.5, vp.x * 0.72, h)
	draw_rect(Rect2(Vector2.ZERO, vp), Color(0.02, 0.02, 0.04, 0.78))
	draw_rect(r, Color(0.07, 0.06, 0.09, 0.95))
	draw_rect(Rect2(r.position, Vector2(r.size.x, 2.0)), Color(0.72, 0.58, 0.30))
	draw_rect(Rect2(r.position + Vector2(0, r.size.y - 2.0), Vector2(r.size.x, 2.0)),
		Color(0.72, 0.58, 0.30))
	return r

const MENU_ITEMS := ["Commencer la partie", "Commandes", "Quitter"]

func _draw_title(vp: Vector2) -> void:
	draw_rect(Rect2(Vector2.ZERO, vp), Color(0.03, 0.028, 0.05))
	var cx := vp.x * 0.5
	_txt("PRINCE OF PERSIA", Vector2(cx, vp.y * 0.26), int(vp.y * 0.085),
		Color(0.92, 0.78, 0.42), true)
	_txt("six niveaux inédits · armes bonus · moteur Godot",
		Vector2(cx, vp.y * 0.335), int(vp.y * 0.026), Color(0.62, 0.58, 0.56), true)
	for i in MENU_ITEMS.size():
		var sel := i == menu_index
		var col := Color(0.98, 0.86, 0.52) if sel else Color(0.60, 0.58, 0.56)
		var label: String = ("› " if sel else "  ") + MENU_ITEMS[i]
		_txt(label, Vector2(cx, vp.y * (0.50 + i * 0.075)), int(vp.y * 0.038), col, true)
	_txt("d'après le jeu de Jordan Mechner (1989)",
		Vector2(cx, vp.y * 0.90), int(vp.y * 0.022), Color(0.42, 0.40, 0.40), true)

func _draw_intro(vp: Vector2) -> void:
	draw_rect(Rect2(Vector2.ZERO, vp), Color(0.02, 0.02, 0.035))
	if world == null or world.lv == null:
		return
	var cx := vp.x * 0.5
	_txt("NIVEAU %d" % (world.idx + 1), Vector2(cx, vp.y * 0.38), int(vp.y * 0.032),
		Color(0.70, 0.60, 0.40), true)
	_txt(world.lv.name, Vector2(cx, vp.y * 0.47), int(vp.y * 0.062),
		Color(0.94, 0.88, 0.74), true)
	_txt("%d salles · %d minutes" % [world.lv.playable_rooms(), world.lv.time / 60],
		Vector2(cx, vp.y * 0.55), int(vp.y * 0.026), Color(0.58, 0.55, 0.54), true)

func _draw_pause(vp: Vector2) -> void:
	var r := _panel(vp, vp.y * 0.62)
	var cx := vp.x * 0.5
	var y := r.position.y + vp.y * 0.07
	var s := int(vp.y * 0.028)
	_txt("PAUSE", Vector2(cx, y), int(vp.y * 0.05), Color(0.94, 0.84, 0.52), true)
	y += vp.y * 0.075
	for line in [
		"← →   courir            Maj + ← →   pas prudent",
		"↑     sauter / grimper  ↓           s'accroupir, descendre",
		"Espace  frapper         Z           parer",
		"T     lancer une dague  F           bâton de flamme",
		"C     rengainer         V           cadrage salle / suivi",
		"R     recommencer       Échap       reprendre",
	]:
		_txt(line, Vector2(cx, y), s, Color(0.78, 0.76, 0.72), true)
		y += s * 1.65

func _draw_dead(vp: Vector2) -> void:
	var r := _panel(vp, vp.y * 0.34)
	var cx := vp.x * 0.5
	_txt("TU ES MORT", Vector2(cx, r.position.y + vp.y * 0.09), int(vp.y * 0.055),
		Color(0.90, 0.32, 0.30), true)
	if world:
		_txt(world.player.cause, Vector2(cx, r.position.y + vp.y * 0.155),
			int(vp.y * 0.028), Color(0.70, 0.62, 0.60), true)
	_txt("Espace : reprendre au début du niveau", Vector2(cx, r.position.y + vp.y * 0.245),
		int(vp.y * 0.026), Color(0.62, 0.60, 0.58), true)

func _draw_won(vp: Vector2) -> void:
	draw_rect(Rect2(Vector2.ZERO, vp), Color(0.03, 0.025, 0.045))
	var cx := vp.x * 0.5
	_txt("LE SABLIER EST BRISÉ", Vector2(cx, vp.y * 0.36), int(vp.y * 0.06),
		Color(0.95, 0.84, 0.50), true)
	if world:
		_txt("%d morts · %d ennemis vaincus" % [world.deaths, world.kills],
			Vector2(cx, vp.y * 0.46), int(vp.y * 0.03), Color(0.66, 0.62, 0.60), true)
	_txt("Espace : revenir au titre", Vector2(cx, vp.y * 0.58), int(vp.y * 0.026),
		Color(0.55, 0.53, 0.52), true)
