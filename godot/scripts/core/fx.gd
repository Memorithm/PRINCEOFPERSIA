# Particules : poussière, sang, gravats, étincelles, flamme, fumée.
#
# Deux familles, dessinées à deux moments différents : la *matière* (poussière,
# sang, gravats) vit dans le monde éclairé, l'*émissif* (flamme, étincelles) est
# peint par-dessus la passe de lumière et n'est donc jamais assombri par
# l'ambiance de la salle.

class_name FX
extends RefCounted

enum Kind { DUST, BLOOD, DEBRIS, SPARK, FLAME, SMOKE }

const MAX_PARTICLES := 1400

var _p: Array = []
var _rng := RandomNumberGenerator.new()

func _init(seed_v: int = 12345) -> void:
	_rng.seed = seed_v

func _push(d: Dictionary) -> void:
	if _p.size() >= MAX_PARTICLES:
		return
	_p.append(d)

func clear() -> void:
	_p.clear()

func count() -> int:
	return _p.size()

func update(dt: float) -> void:
	var keep: Array = []
	keep.resize(0)
	for q in _p:
		q.t += dt
		if q.t >= q.life:
			continue
		var g: float = q.grav
		q.v.y += g * dt
		q.v *= 1.0 - q.drag * dt
		q.p += q.v * dt
		if q.has_floor and q.p.y >= q.floor_y:
			q.p.y = q.floor_y
			q.v.y = -q.v.y * 0.28
			q.v.x *= 0.6
			if absf(q.v.y) < 6.0:
				q.v = Vector2.ZERO
				q.grav = 0.0
				q.has_floor = false
		keep.append(q)
	_p = keep

# ---------------------------------------------------------------- émetteurs

func dust(at: Vector2, n: int, power: float, col: Color) -> void:
	for i in n:
		_push({
			"kind": Kind.DUST, "p": at + _rv(2.5),
			"v": Vector2(_rng.randf_range(-22.0, 22.0), _rng.randf_range(-26.0, -4.0)) * power,
			"life": _rng.randf_range(0.35, 0.75) * (0.6 + power), "t": 0.0,
			"col": col, "size": _rng.randf_range(0.8, 2.1),
			"grav": 40.0, "drag": 2.4, "has_floor": false, "floor_y": 0.0,
		})

func blood(at: Vector2, dir: float, n: int, floor_y: float) -> void:
	for i in n:
		_push({
			"kind": Kind.BLOOD, "p": at + _rv(3.0),
			"v": Vector2(dir * _rng.randf_range(10.0, 90.0), _rng.randf_range(-90.0, 10.0)),
			"life": _rng.randf_range(0.9, 2.2), "t": 0.0,
			"col": Color8(158, 22, 28).lerp(Color8(96, 10, 16), _rng.randf()),
			"size": _rng.randf_range(0.9, 2.4),
			"grav": 380.0, "drag": 0.3, "has_floor": true, "floor_y": floor_y,
		})

func debris(at: Vector2, n: int, col: Color, floor_y: float) -> void:
	for i in n:
		_push({
			"kind": Kind.DEBRIS, "p": at + _rv(6.0),
			"v": Vector2(_rng.randf_range(-70.0, 70.0), _rng.randf_range(-120.0, -20.0)),
			"life": _rng.randf_range(1.0, 2.4), "t": 0.0,
			"col": Geom.shade(col, _rng.randf_range(0.7, 1.15)),
			"size": _rng.randf_range(1.2, 3.2),
			"grav": 420.0, "drag": 0.2, "has_floor": true, "floor_y": floor_y,
		})

func sparks(at: Vector2, n: int, power: float) -> void:
	for i in n:
		var a := _rng.randf_range(0.0, TAU)
		var s := _rng.randf_range(30.0, 150.0) * power
		_push({
			"kind": Kind.SPARK, "p": at,
			"v": Vector2(cos(a), sin(a)) * s,
			"life": _rng.randf_range(0.18, 0.5), "t": 0.0,
			"col": Color8(255, 226, 160), "size": _rng.randf_range(0.6, 1.5),
			"grav": 120.0, "drag": 3.0, "has_floor": false, "floor_y": 0.0,
		})

func flame(at: Vector2, rise: float, power: float, hue: Color) -> void:
	_push({
		"kind": Kind.FLAME, "p": at + _rv(1.6),
		"v": Vector2(_rng.randf_range(-6.0, 6.0), -rise * _rng.randf_range(0.6, 1.2)),
		"life": _rng.randf_range(0.22, 0.5) * power, "t": 0.0,
		"col": hue, "size": _rng.randf_range(1.6, 3.4) * power,
		"grav": -18.0, "drag": 1.6, "has_floor": false, "floor_y": 0.0,
	})

func smoke(at: Vector2, power: float) -> void:
	_push({
		"kind": Kind.SMOKE, "p": at + _rv(2.0),
		"v": Vector2(_rng.randf_range(-5.0, 5.0), _rng.randf_range(-22.0, -10.0)),
		"life": _rng.randf_range(0.8, 1.8) * power, "t": 0.0,
		"col": Color8(60, 56, 62), "size": _rng.randf_range(2.0, 4.5),
		"grav": -10.0, "drag": 1.1, "has_floor": false, "floor_y": 0.0,
	})

func _rv(r: float) -> Vector2:
	return Vector2(_rng.randf_range(-r, r), _rng.randf_range(-r, r))

# ---------------------------------------------------------------- dessin

## Poussière, sang, gravats : de la matière, donc éclairée comme le décor.
func draw_matter(ci: CanvasItem) -> void:
	for q in _p:
		if q.kind == Kind.FLAME or q.kind == Kind.SPARK:
			continue
		var f: float = 1.0 - q.t / q.life
		var c: Color = q.col
		var s: float = q.size * (0.45 + 0.55 * f) if q.kind != Kind.SMOKE else q.size * (0.6 + 1.2 * (1.0 - f))
		var alpha: float = f if q.kind != Kind.SMOKE else f * 0.45
		ci.draw_rect(Rect2(q.p - Vector2(s, s) * 0.5, Vector2(s, s)),
			Color(c.r, c.g, c.b, alpha))

## Flamme et étincelles : peintes après la lumière, jamais assombries.
func draw_light(ci: CanvasItem) -> void:
	for q in _p:
		if q.kind != Kind.FLAME and q.kind != Kind.SPARK:
			continue
		var f: float = 1.0 - q.t / q.life
		if q.kind == Kind.SPARK:
			var s: float = q.size * f
			ci.draw_rect(Rect2(q.p - Vector2(s, s) * 0.5, Vector2(s, s)),
				Color(1.0, 0.92, 0.7, minf(f * 1.4, 1.0)))
			continue
		# Une flamme passe du blanc au cœur à sa teinte, puis au rouge sombre.
		var c: Color = q.col
		var hot := Color(1.0, 0.96, 0.82)
		var col := hot.lerp(c, clampf((1.0 - f) * 1.9, 0.0, 1.0))
		col = col.lerp(Color(0.45, 0.10, 0.04), clampf((1.0 - f - 0.55) * 2.2, 0.0, 1.0))
		var s: float = q.size * (0.35 + 0.9 * f)
		Shape.disc(ci, q.p, s, Color(col.r, col.g, col.b, minf(f * 1.5, 0.95)), 7)

## Les flammes éclairent leurs environs : renvoie [[position, rayon, couleur,
## intensité], ...] pour la passe de lumière.
func emitter_lights() -> Array:
	var out: Array = []
	var n := 0
	for q in _p:
		if q.kind != Kind.FLAME:
			continue
		n += 1
		if n % 5 != 0:
			continue
		var f: float = 1.0 - q.t / q.life
		out.append([q.p, 26.0 * f, q.col, 0.22 * f])
	return out
