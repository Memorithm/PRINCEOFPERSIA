## HUD de l'aventure : cœurs, gemmes, clés, fragments du sceau, reliques,
## nom du monde, messages, fondus de mort / flash / pause / victoire.
extends Control

var game: Node = null
const HEART_RED := Color(0.886, 0.235, 0.259)
const HEART_DIM := Color(0.275, 0.157, 0.188)
const GOLD := Color(0.91, 0.76, 0.39)


func _draw() -> void:
	if game == null:
		return
	var g := game
	var vp := size
	var u := vp.y / 720.0   # unité de mise en page

	# Bandeau supérieur.
	draw_rect(Rect2(0, 0, vp.x, 44 * u), Color(0.055, 0.04, 0.07, 0.62))

	# Cœurs.
	var full: int = g.inv.hp / 2
	var half: bool = g.inv.hp % 2 == 1
	var total: int = g.inv.hp_max / 2 + (1 if g.inv.hp_max % 2 == 1 else 0)
	var hx := 14.0 * u
	for i in total:
		var filled := 1.0
		if i >= full:
			filled = 0.45 if (i == full and half) else 0.0
		var beat := 1.0 + absf(sin(g.time * 2.4 + i)) * 0.06 if i < full else 1.0
		_heart(Vector2(hx, 22 * u), 13.0 * u * beat, HEART_RED if filled > 0 else HEART_DIM, filled)
		hx += 34.0 * u

	# Compteurs : gemme, clé.
	var font := ThemeDB.fallback_font
	var fs := int(20 * u)
	var ix := hx + 10.0 * u
	_gem(Vector2(ix, 22 * u), 11.0 * u)
	ix += 18.0 * u
	draw_string(font, Vector2(ix, 29 * u), str(g.inv.gems), HORIZONTAL_ALIGNMENT_LEFT, -1, fs, Color(0.47, 0.86, 0.9))
	ix += (str(g.inv.gems).length() * 12.0 + 8.0) * u
	_key(Vector2(ix, 22 * u), 11.0 * u)
	ix += 22.0 * u
	draw_string(font, Vector2(ix, 29 * u), str(g.inv.keys), HORIZONTAL_ALIGNMENT_LEFT, -1, fs, GOLD)
	ix += (str(g.inv.keys).length() * 12.0 + 14.0) * u

	# Les 4 fragments du sceau.
	for k in 4:
		var filled2: bool = g.inv.seals > k
		_seal(Vector2(ix, 22 * u), 12.0 * u, GOLD if filled2 else Color(0.38, 0.32, 0.2),
			g.time if filled2 else 0.0)
		ix += 30.0 * u
	if g.inv.mirror:
		_mirror(Vector2(ix, 22 * u), 11.0 * u, g.time)
		ix += 34.0 * u
	if g.inv.sword:
		_sword(Vector2(ix, 22 * u), 12.0 * u)

	# Nom du monde, à droite.
	var world_name: String = AdventureData.NAMES[g.cur]
	var nw := font.get_string_size(world_name, HORIZONTAL_ALIGNMENT_LEFT, -1, fs).x
	draw_string(font, Vector2(vp.x - nw - 14 * u, 29 * u), world_name, HORIZONTAL_ALIGNMENT_LEFT, -1, fs,
		Color(0.886, 0.847, 0.745))

	# Message courant.
	if g.msg != "" and g.msg_t > 0.0:
		var col := Color(0.94, 0.55, 0.38) if g.msg_warn else Color(0.957, 0.918, 0.804)
		var mw := font.get_string_size(g.msg, HORIZONTAL_ALIGNMENT_LEFT, -1, fs).x
		var mx := (vp.x - mw) * 0.5
		var my := vp.y - 52.0 * u
		draw_rect(Rect2(mx - 12 * u, my - 8 * u, mw + 24 * u, fs + 14 * u), Color(0.04, 0.03, 0.055, 0.66))
		draw_string(font, Vector2(mx, my + fs * 0.8), g.msg, HORIZONTAL_ALIGNMENT_LEFT, -1, fs, col)

	# Fondu de mort.
	if g.phase == "dying" or g.phase == "dead":
		var k: float = clampf(g.dead_t / 1.2, 0.0, 1.0)
		draw_rect(Rect2(Vector2.ZERO, vp), Color(0.12, 0.03, 0.03, k * 0.85))
		if g.phase == "dead" or g.dead_t > 0.9:
			var ts := int(42 * u)
			var t := "TU ES MORT"
			var tw := font.get_string_size(t, HORIZONTAL_ALIGNMENT_LEFT, -1, ts).x
			draw_string(font, Vector2((vp.x - tw) / 2, vp.y * 0.46), t, HORIZONTAL_ALIGNMENT_LEFT, -1, ts,
				Color(0.89, 0.33, 0.31))

	# Flash.
	if g.flash_t > 0.0:
		draw_rect(Rect2(Vector2.ZERO, vp), Color(g.flash_col.r, g.flash_col.g, g.flash_col.b, g.flash_t * 0.45))

	# Pause / victoire.
	if g.paused:
		_overlay(vp, "PAUSE", "P reprendre   R recommencer   Échap quitter", u)
	elif g.phase == "victory":
		_overlay(vp, "LE SCEAU EST BRISÉ", "Ganar n'est plus. Zahra est libre.   R rejouer", u)


func _overlay(vp: Vector2, title: String, sub: String, u: float) -> void:
	draw_rect(Rect2(Vector2.ZERO, vp), Color(0.03, 0.024, 0.047, 0.66))
	var font := ThemeDB.fallback_font
	var ts := int(52 * u)
	var ss := int(22 * u)
	var tw := font.get_string_size(title, HORIZONTAL_ALIGNMENT_LEFT, -1, ts).x
	draw_string(font, Vector2((vp.x - tw) / 2, vp.y * 0.44), title, HORIZONTAL_ALIGNMENT_LEFT, -1, ts, GOLD)
	var sw := font.get_string_size(sub, HORIZONTAL_ALIGNMENT_LEFT, -1, ss).x
	draw_string(font, Vector2((vp.x - sw) / 2, vp.y * 0.44 + 44 * u), sub, HORIZONTAL_ALIGNMENT_LEFT, -1, ss,
		Color(0.863, 0.824, 0.745))


func _heart(c: Vector2, r: float, col: Color, filled: float) -> void:
	var dim := col if filled > 0 else Color(0.275, 0.157, 0.188)
	draw_circle(c + Vector2(-r * 0.5, -r * 0.28), r * 0.58, dim)
	draw_circle(c + Vector2(r * 0.5, -r * 0.28), r * 0.58, dim)
	draw_colored_polygon(PackedVector2Array([
		c + Vector2(-r * 1.02, -r * 0.1), c + Vector2(r * 1.02, -r * 0.1), c + Vector2(0, r * 1.15)]), dim)
	if filled >= 1.0:
		draw_circle(c + Vector2(-r * 0.38, -r * 0.42), r * 0.2, Color(0.96, 0.51, 0.51))


func _gem(c: Vector2, r: float) -> void:
	draw_colored_polygon(PackedVector2Array([c + Vector2(0, -r), c + Vector2(r * 0.84, -r * 0.2),
		c + Vector2(0, r), c + Vector2(-r * 0.84, -r * 0.2)]), Color(0.314, 0.784, 0.824))
	draw_colored_polygon(PackedVector2Array([c + Vector2(0, -r), c + Vector2(r * 0.84, -r * 0.2),
		c + Vector2(0, -r * 0.1)]), Color(0.627, 0.941, 0.957))


func _key(c: Vector2, r: float) -> void:
	var col := Color(0.933, 0.784, 0.392)
	draw_circle(c + Vector2(-r, 0), r, col)
	draw_circle(c + Vector2(-r, 0), r * 0.4, Color(0.12, 0.094, 0.07))
	draw_line(c + Vector2(-0.4 * r, 0), c + Vector2(2.2 * r, 0), col, r * 0.28, true)
	draw_line(c + Vector2(1.4 * r, 0), c + Vector2(1.4 * r, r * 0.9), col, r * 0.24, true)


func _seal(c: Vector2, r: float, col: Color, t: float) -> void:
	var glow := 1.0 + 0.12 * sin(t * 3.0) if t > 0.0 else 1.0
	draw_colored_polygon(PackedVector2Array([c + Vector2(0, -r * glow), c + Vector2(r * 0.84, 0),
		c + Vector2(0, r * glow), c + Vector2(-r * 0.84, 0)]), col)
	if t > 0.0:
		draw_circle(c, r * 0.25, Color(1, 0.94, 0.75))


func _mirror(c: Vector2, r: float, t: float) -> void:
	draw_circle(c + Vector2(0, -r * 0.14), r, Color(0.588, 0.612, 0.675))
	draw_circle(c + Vector2(0, -r * 0.14), r * 0.73, Color(0.784, 0.839, 0.91))
	draw_line(c + Vector2(0, r * 0.2), c + Vector2(0, r * 0.51), Color(0.588, 0.612, 0.675), r * 0.16, true)
	if sin(t * 2.2) > 0.0:
		draw_line(c + Vector2(-r * 0.36, -r * 0.27), c + Vector2(r * 0.14, r * 0.05), Color.WHITE, r * 0.09, true)


func _sword(c: Vector2, r: float) -> void:
	draw_line(c + Vector2(-r * 0.29, r * 0.33), c + Vector2(r * 0.29, -r * 0.33), Color(0.94, 0.965, 1), r * 0.18, true)
	draw_line(c + Vector2(-r * 0.23, r * 0.42), c + Vector2(-r * 0.07, r * 0.25), Color(0.745, 0.62, 0.376), r * 0.13, true)
