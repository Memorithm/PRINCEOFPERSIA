# Les objets ramassables : fioles, armes, bouclier.
#
# Chacun est dessiné en polygones dans les unités du monde, avec le même ombrage
# cel que les personnages : un ton de base, une ombre franche, un liseré.

class_name ItemArt

static func draw_item(ci: CanvasItem, kind: int, x: float, surf: float, bob: float) -> void:
	var p := Vector2(x, surf - 1.0 + bob)
	if Level.item_is_potion(kind):
		_bottle(ci, p, Level.item_colour(kind))
		return
	match kind:
		Level.Item.SWORD: _sword(ci, p, Color8(206, 214, 228), 13.0)
		Level.Item.SCIMITAR: _scimitar(ci, p)
		Level.Item.DAGGERS: _daggers(ci, p)
		Level.Item.WAND: _wand(ci, p)
		Level.Item.BUCKLER: _buckler(ci, p)

static func _bottle(ci: CanvasItem, p: Vector2, col: Color) -> void:
	var glass := Color8(214, 226, 236)
	# Panse, épaule, col, bouchon.
	var body := PackedVector2Array([
		Vector2(p.x - 3.2, p.y),
		Vector2(p.x - 3.6, p.y - 4.4),
		Vector2(p.x - 1.8, p.y - 7.4),
		Vector2(p.x - 1.1, p.y - 10.4),
		Vector2(p.x + 1.1, p.y - 10.4),
		Vector2(p.x + 1.8, p.y - 7.4),
		Vector2(p.x + 3.6, p.y - 4.4),
		Vector2(p.x + 3.2, p.y),
	])
	Shape.poly(ci, body, Geom.shade(glass, 0.42))
	# Le liquide s'arrête sous l'épaule.
	Shape.poly(ci, PackedVector2Array([
		Vector2(p.x - 3.0, p.y - 0.4),
		Vector2(p.x - 3.4, p.y - 4.2),
		Vector2(p.x - 1.9, p.y - 6.6),
		Vector2(p.x + 1.9, p.y - 6.6),
		Vector2(p.x + 3.4, p.y - 4.2),
		Vector2(p.x + 3.0, p.y - 0.4),
	]), col)
	Shape.poly(ci, PackedVector2Array([
		Vector2(p.x - 3.0, p.y - 0.4),
		Vector2(p.x - 3.4, p.y - 4.2),
		Vector2(p.x - 1.4, p.y - 4.4),
		Vector2(p.x - 1.2, p.y - 0.4),
	]), Geom.shade(col, 1.35))
	# Reflet vertical sur le verre.
	Shape.poly(ci, PackedVector2Array([
		Vector2(p.x + 1.5, p.y - 1.2),
		Vector2(p.x + 2.4, p.y - 4.0),
		Vector2(p.x + 1.6, p.y - 6.6),
		Vector2(p.x + 1.0, p.y - 4.0),
	]), Color(1, 1, 1, 0.35))
	ci.draw_rect(Rect2(p.x - 1.5, p.y - 12.0, 3.0, 1.9), Color8(122, 84, 52))

static func _sword(ci: CanvasItem, p: Vector2, col: Color, length: float) -> void:
	# Posée à plat sur le sol, pointe vers la droite.
	var a := Vector2(p.x - length * 0.5, p.y - 1.6)
	var b := Vector2(p.x + length * 0.5, p.y - 2.4)
	Shape.poly(ci, PackedVector2Array([
		a + Vector2(0, -1.1), Vector2(b.x - 2.0, b.y - 1.3), b,
		Vector2(b.x - 2.0, b.y + 1.0), a + Vector2(0, 1.1),
	]), Geom.shade(col, 0.72))
	Shape.poly(ci, PackedVector2Array([
		a + Vector2(0, -1.1), Vector2(b.x - 2.0, b.y - 1.3), b, Vector2(b.x - 2.2, b.y - 0.2),
	]), Geom.shade(col, 1.2))
	Shape.capsule(ci, a, a - Vector2(3.6, -0.4), 1.0, 1.1, Color8(96, 62, 36))
	Shape.capsule(ci, a + Vector2(0, -2.2), a + Vector2(0, 2.2), 0.8, 0.8, Color8(190, 154, 78))

static func _scimitar(ci: CanvasItem, p: Vector2) -> void:
	var col := Color8(226, 216, 180)
	var a := Vector2(p.x - 7.0, p.y - 1.6)
	var mid := Vector2(p.x + 1.0, p.y - 5.2)
	var tip := Vector2(p.x + 8.5, p.y - 3.0)
	Shape.poly(ci, PackedVector2Array([
		a, mid + Vector2(0, -1.4), tip, mid + Vector2(0.5, 1.4),
	]), Geom.shade(col, 0.7))
	Shape.poly(ci, PackedVector2Array([
		a, mid + Vector2(0, -1.4), tip, mid + Vector2(0, -0.2),
	]), Geom.shade(col, 1.22))
	Shape.capsule(ci, a, a - Vector2(3.4, -0.6), 1.1, 1.2, Color8(72, 46, 30))
	Shape.disc(ci, a - Vector2(4.0, -0.7), 1.3, Color8(206, 168, 84), 8)

static func _daggers(ci: CanvasItem, p: Vector2) -> void:
	for i in 3:
		var o := Vector2((i - 1) * 3.4, -float(i) * 1.1)
		var a := p + o + Vector2(-3.4, -1.2)
		var b := p + o + Vector2(3.0, -2.4)
		Shape.poly(ci, PackedVector2Array([
			a + Vector2(0, -0.8), b, a + Vector2(0, 0.8),
		]), Geom.shade(Color8(214, 222, 234), 0.9 + i * 0.12))
		Shape.capsule(ci, a, a - Vector2(2.2, -0.2), 0.8, 0.9, Color8(88, 58, 36))

static func _wand(ci: CanvasItem, p: Vector2) -> void:
	var a := Vector2(p.x - 6.0, p.y - 1.0)
	var b := Vector2(p.x + 5.0, p.y - 8.0)
	Shape.capsule(ci, a, b, 1.2, 0.9, Color8(104, 72, 44))
	Shape.disc(ci, b, 2.4, Color8(255, 178, 64))
	Shape.disc(ci, b, 1.2, Color8(255, 236, 190))

static func _buckler(ci: CanvasItem, p: Vector2) -> void:
	var face := Color8(148, 116, 78)
	var c := p + Vector2(0, -5.0)
	Shape.disc(ci, c, 5.6, Geom.shade(face, 0.8), 14)
	Shape.disc(ci, c, 4.6, face, 14)
	Shape.disc(ci, c + Vector2(-1.0, -1.0), 3.0, Geom.shade(face, 1.2), 12)
	Shape.disc(ci, c, 1.8, Color8(198, 202, 210), 10)

## Halo additif d'un objet, dessiné après la passe de lumière.
static func draw_item_glow(ci: CanvasItem, kind: int, x: float, surf: float,
		bob: float, pulse: float) -> void:
	var p := Vector2(x, surf - 6.0 + bob)
	var col := Level.item_colour(kind)
	var r := 7.0 + pulse * 1.4
	for i in 3:
		var k := 1.0 - float(i) / 3.0
		Shape.disc(ci, p, r * (0.5 + 0.5 * (i + 1)), Color(col.r, col.g, col.b, 0.10 * k), 12)

static func draw_dagger_flight(ci: CanvasItem, p: Vector2, spin: float) -> void:
	var d := Vector2(cos(spin), sin(spin))
	var n := d.orthogonal()
	Shape.poly(ci, PackedVector2Array([
		p - d * 3.4 + n * 0.7, p + d * 3.6, p - d * 3.4 - n * 0.7,
	]), Color8(226, 232, 240))
	Shape.capsule(ci, p - d * 3.4, p - d * 5.2, 0.8, 0.7, Color8(88, 58, 36))

static func draw_fireball(ci: CanvasItem, p: Vector2, r: float, wob: float) -> void:
	Shape.disc(ci, p, r * (1.8 + wob * 0.12), Color(1.0, 0.45, 0.1, 0.28), 12)
	Shape.disc(ci, p, r * (1.15 + wob * 0.08), Color8(255, 148, 44), 12)
	Shape.disc(ci, p, r * 0.6, Color8(255, 236, 176), 10)

static func draw_slash(ci: CanvasItem, at: Vector2, facing: float, t: float) -> void:
	if t >= 1.0:
		return
	var a := (1.0 - t) * 0.5
	var r := 8.0 + t * 12.0
	var pts := PackedVector2Array()
	var n := 9
	for i in n:
		var ang := lerpf(-0.7, 0.7, float(i) / (n - 1))
		pts.append(at + Vector2(cos(ang) * facing, sin(ang)) * r)
	for i in range(n - 1, -1, -1):
		var ang := lerpf(-0.7, 0.7, float(i) / (n - 1))
		pts.append(at + Vector2(cos(ang) * facing, sin(ang)) * (r - 2.6))
	Shape.poly(ci, pts, Color(1.0, 0.96, 0.86, a))

static func draw_shadow_aura(ci: CanvasItem, p: Vector2, r: float) -> void:
	for i in 3:
		var k := 1.0 - float(i) / 3.0
		Shape.disc(ci, p, r * (0.6 + 0.5 * i), Color(0.55, 0.3, 0.7, 0.10 * k), 14)
