# L'état du jeu et le pas de simulation.
#
# La simulation tourne à pas fixe (120 Hz) avec un accumulateur, quelle que soit
# la cadence d'affichage : c'est ce qui garantit qu'un saut franchit exactement la
# même distance sur une machine à 30 images/s et sur une à 240.
#
# Ordre des passes de rendu :
#   1. décor, objets, personnages, matière  (assombris par l'ambiance,
#      rallumés par les lumières de torche)
#   2. émissif : flammes, étincelles, halos, lames — peint par-dessus, jamais
#      assombri
#   3. étalonnage : vignette, grain, éclair de dégât, fondu

class_name World
extends Node2D

signal said(text: String, warn: bool)
signal level_finished
signal run_failed(cause: String)
signal run_won

enum Phase { PLAY, DYING, DEAD, LEAVING, LEVEL_DONE, VICTORY, TIME_UP }

const STEP := 1.0 / 120.0
const MAX_STEPS := 6

var lv: Level
var dyn: Dynamics
var idx := 0
var player: Prince
var guards: Array = []
var items: Array = []      ## [{kind, tx, ty, taken}]
var shots: Array = []      ## [{kind, p, v, life, spin, friendly}]
var fx: FX

var cam_room := Vector2i.ZERO
var cam_at := Vector2.ZERO
var cam_target := Vector2.ZERO
var shake := 0.0
var view_w := Geom.ROOM_W
var view_h := Geom.ROOM_H
## 1 = une salle entière comme sur Apple II ; au-delà, la caméra suit le prince.
var zoom := 1.0

var phase := Phase.PLAY
var phase_t := 0.0
var msg_text := ""
var msg_t := 0.0
var msg_warn := false
var clock := 900.0
var elapsed := 0.0
var rng := RandomNumberGenerator.new()
var carry := {"hp_max": 3, "sword": false, "scimitar": false, "buckler": false,
	"wand": false, "daggers": 0}
var deaths := 0
var kills := 0
var flash_t := 0.0
var flash_col := Color.BLACK
var slashes: Array = []    ## [[position, facing, âge]]
var advance := false

var _acc := 0.0
var _emissive: Node2D
var _figures: Node2D
var _matter: Node2D
var _lights: Array = []     ## réserve de PointLight2D
var _light_tex: Texture2D
var _modulate: CanvasModulate

# ---------------------------------------------------------------- mise en place

func _ready() -> void:
	fx = FX.new(0xC0FFEE)
	_light_tex = _make_light_texture()
	_modulate = CanvasModulate.new()
	add_child(_modulate)

	# Les figures ont leur propre calque : elles se peignent après le décor et
	# avant la matière (poussière, sang), et chacune pose d'abord sa silhouette
	# détourée.
	_figures = Node2D.new()
	_figures.set_script(load("res://scripts/nodes/figures.gd"))
	_figures.set("world", self)
	add_child(_figures)

	_matter = Node2D.new()
	_matter.set_script(load("res://scripts/nodes/matter.gd"))
	_matter.set("world", self)
	add_child(_matter)

func attach_emissive(n: Node2D) -> void:
	_emissive = n

func load_level(i: int, keep: Dictionary, seed_v: int) -> String:
	idx = i
	carry = keep.duplicate()
	rng.seed = seed_v
	lv = Level.new()
	var err := lv.parse(Levels.CAMPAIGN[clampi(i, 0, Levels.CAMPAIGN.size() - 1)])
	if err != "":
		return err
	dyn = Dynamics.new(lv.tw, lv.th)
	player = Prince.new(Vector2(Geom.cx(lv.start.x), Geom.surf(lv.start.y)),
		lv.start_face, carry)
	guards.clear()
	for m in lv.mobs:
		guards.append(Guard.new(m))
	items.clear()
	for it in lv.items:
		items.append({"kind": it["kind"], "tx": it["tx"], "ty": it["ty"], "taken": false})
	shots.clear()
	slashes.clear()
	fx.clear()
	phase = Phase.PLAY
	phase_t = 0.0
	clock = float(lv.time)
	elapsed = 0.0
	advance = false
	flash_t = 0.0
	shake = 0.0
	_init_dynamics()
	_modulate.color = lv.theme["ambient"]
	cam_room = Geom.room_of(lv.start.x, lv.start.y)
	centre_camera()
	say(lv.hint, 6.0, false)
	return ""

func _init_dynamics() -> void:
	for ty in lv.th:
		for tx in lv.tw:
			match lv.tile(tx, ty):
				Level.T.CHOMPER:
					# On décale la phase pour qu'une rangée de lames ondule.
					dyn.set_b(tx, ty, float((tx * 7 + ty * 13) % 21) / 21.0 * Geom.CHOMP_PERIOD)
				Level.T.SPIKES:
					dyn.set_a(tx, ty, 0.0)

func restart() -> void:
	var d := deaths
	var k := kills
	load_level(idx, carry, rng.randi())
	deaths = d
	kills = k

func say(text: String, t: float, warn: bool) -> void:
	msg_text = text
	msg_t = t
	msg_warn = warn
	said.emit(text, warn)

# ---------------------------------------------------------------- requêtes

func gate_passable(tx: int, ty: int) -> bool:
	return dyn.a(tx, ty) >= 0.55

## Un corps peut-il occuper cette case ?
func open(tx: int, ty: int) -> bool:
	var t := lv.tile(tx, ty)
	if Level.tile_solid(t):
		return false
	if t == Level.T.GATE:
		return gate_passable(tx, ty)
	return true

## Cette case offre-t-elle un appui ?
func supported(tx: int, ty: int) -> bool:
	if not lv.in_bounds(tx, ty):
		return false
	var here := lv.tile(tx, ty)
	if Level.tile_solid(here):
		return false
	return Level.tile_walkable(here) or Level.tile_solid(lv.tile(tx, ty + 1))

## Se tenir dans cette case est-il mortel en ce moment ?
func lethal(tx: int, ty: int) -> bool:
	match lv.tile(tx, ty):
		Level.T.SPIKES: return dyn.a(tx, ty) > 0.42
		Level.T.CHOMPER: return dyn.a(tx, ty) > 0.72
	return false

func view_rect() -> Rect2:
	return Rect2(cam_at, Vector2(view_w, view_h))

func set_view_size(w: float, h: float) -> void:
	view_w = maxf(w, Geom.TILE_W * 3.0)
	view_h = maxf(h, Geom.TILE_H * 1.2)

## Glisse `x` horizontalement, en butant contre la maçonnerie et les herses
## fermées.
func slide_x(x: float, y: float, dx: float) -> float:
	if dx == 0.0:
		return x
	var d := 1.0 if dx > 0.0 else -1.0
	var nx := x + dx
	var tx := Geom.tx_of(nx + d * Geom.BODY_HW)
	var blocked := false
	for h in [5.0, 15.0, 25.0]:
		if not open(tx, Geom.ty_of(y - h)):
			blocked = true
			break
	if not blocked:
		return nx
	if d > 0.0:
		var bound := float(tx) * Geom.TILE_W - Geom.BODY_HW - 0.02
		return maxf(minf(nx, bound), minf(x, bound))
	var bound2 := float(tx + 1) * Geom.TILE_W + Geom.BODY_HW + 0.02
	return minf(maxf(nx, bound2), maxf(x, bound2))

# ---------------------------------------------------------------- boucle

func step_frame(delta: float, inp: InputState) -> void:
	_acc += minf(delta, 0.25)
	var n := 0
	var first := true
	while _acc >= STEP and n < MAX_STEPS:
		_acc -= STEP
		n += 1
		_step(STEP, inp, first)
		first = false
	if n >= MAX_STEPS:
		_acc = 0.0
	queue_redraw()
	_figures.queue_redraw()
	_matter.queue_redraw()
	if _emissive:
		_emissive.queue_redraw()
	_sync_lights()

func _step(dt: float, inp_full: InputState, first: bool) -> void:
	# Les fronts montants n'appartiennent qu'au premier sous-pas de l'image :
	# sinon un appui unique déclencherait plusieurs sauts.
	var inp := inp_full
	if not first:
		inp = _without_edges(inp_full)
	elapsed += dt
	if phase == Phase.PLAY:
		clock -= dt
		if clock <= 0.0:
			clock = 0.0
			phase = Phase.TIME_UP
			run_failed.emit("Le sable a fini de couler.")

	_update_tiles(dt)
	match phase:
		Phase.PLAY:
			_update_player(dt, inp)
			_update_guards(dt)
			_update_shots(dt)
			_check_items()
		Phase.DYING:
			phase_t -= dt
			if phase_t <= 0.0:
				phase = Phase.DEAD
				run_failed.emit(player.cause)
			_update_player_dead(dt)
			_update_guards(dt)
		Phase.LEAVING:
			phase_t -= dt
			if phase_t <= 0.0:
				phase = Phase.LEVEL_DONE
				advance = true
				level_finished.emit()
		_:
			pass

	_spawn_ambient(dt)
	for s in slashes:
		s[2] += dt * 5.0
	slashes = slashes.filter(func(s): return s[2] < 1.0)
	fx.update(dt)
	_update_camera(dt)
	if msg_t > 0.0:
		msg_t = maxf(msg_t - dt, 0.0)
	flash_t = maxf(flash_t - dt * 2.6, 0.0)
	shake = maxf(shake - dt * 4.0, 0.0)

static func _without_edges(a: InputState) -> InputState:
	var b := InputState.new()
	b.left = a.left; b.right = a.right; b.up = a.up; b.down = a.down
	b.careful = a.careful; b.parry = a.parry
	return b

# ---------------------------------------------------------------- tuiles

func _update_tiles(dt: float) -> void:
	# Quelles cases portent un poids cette image ?
	var pressed: Array[Vector2i] = []
	if phase == Phase.PLAY or phase == Phase.DYING:
		pressed.append(player.foot_tile())
	for g in guards:
		if g.st != Guard.S.DEAD:
			pressed.append(g.foot_tile())

	# Dalles : verrouillent ou lancent une minuterie sur leur groupe.
	var raise_g: Array[int] = []
	var drop_g: Array[int] = []
	for c in pressed:
		var t := lv.tile(c.x, c.y)
		var grp := lv.group(c.x, c.y)
		if grp == 0:
			continue
		if t == Level.T.PLATE_RAISE:
			raise_g.append(grp)
		elif t == Level.T.PLATE_DROP:
			drop_g.append(grp)

	for ty in lv.th:
		for tx in lv.tw:
			var tile := lv.tile(tx, ty)
			match tile:
				Level.T.PLATE_RAISE, Level.T.PLATE_DROP:
					var on := pressed.has(Vector2i(tx, ty))
					dyn.set_a(tx, ty, Geom.approach(dyn.a(tx, ty), 1.0 if on else 0.0, dt * 7.0))
					dyn.set_flag(tx, ty, Dynamics.F_PRESSED, on)
				Level.T.GATE, Level.T.EXIT:
					var grp := lv.group(tx, ty)
					var latching := grp >= 40
					if grp != 0 and raise_g.has(grp):
						if latching:
							dyn.set_flag(tx, ty, Dynamics.F_LATCHED, true)
						dyn.set_b(tx, ty, Geom.GATE_HOLD)
					if grp != 0 and drop_g.has(grp):
						dyn.set_flag(tx, ty, Dynamics.F_LATCHED, false)
						dyn.set_b(tx, ty, 0.0)
					var held := dyn.has(tx, ty, Dynamics.F_LATCHED) or dyn.b(tx, ty) > 0.0
					if dyn.b(tx, ty) > 0.0:
						dyn.set_b(tx, ty, dyn.b(tx, ty) - dt)
					var rate := Geom.GATE_RISE if held else Geom.GATE_FALL
					dyn.set_a(tx, ty, Geom.approach(dyn.a(tx, ty), 1.0 if held else 0.0, dt * rate))
				Level.T.SPIKES:
					# S'arment quand quelque chose est dessus ou juste à côté,
					# puis jaillissent et restent sorties un moment.
					var near := false
					for c in pressed:
						if c.y == ty and absi(c.x - tx) <= 1:
							near = true
							break
					if near:
						dyn.set_flag(tx, ty, Dynamics.F_ARMED, true)
						dyn.set_b(tx, ty, 1.4)
					if dyn.b(tx, ty) > 0.0:
						dyn.set_b(tx, ty, dyn.b(tx, ty) - dt)
					else:
						dyn.set_flag(tx, ty, Dynamics.F_ARMED, false)
					var armed := dyn.has(tx, ty, Dynamics.F_ARMED)
					dyn.set_a(tx, ty, Geom.approach(dyn.a(tx, ty), 1.0 if armed else 0.0,
						dt * (9.0 if armed else 2.2)))
				Level.T.CHOMPER:
					var b := dyn.b(tx, ty) + dt
					if b >= Geom.CHOMP_PERIOD:
						b -= Geom.CHOMP_PERIOD
					dyn.set_b(tx, ty, b)
					# Longue phase ouverte, fermeture sèche : lisible et loyal.
					var f := b / Geom.CHOMP_PERIOD
					var a := 0.0
					if f < 0.62:
						a = 0.0
					elif f < 0.72:
						a = (f - 0.62) / 0.10
					elif f < 0.86:
						a = 1.0
					else:
						a = 1.0 - (f - 0.86) / 0.14
					dyn.set_a(tx, ty, a)
				Level.T.LOOSE:
					var on2 := pressed.has(Vector2i(tx, ty))
					if on2 and not dyn.has(tx, ty, Dynamics.F_TRIGGERED):
						dyn.set_flag(tx, ty, Dynamics.F_TRIGGERED, true)
						dyn.set_b(tx, ty, Geom.LOOSE_FUSE)
					if dyn.has(tx, ty, Dynamics.F_TRIGGERED):
						var nb := dyn.b(tx, ty) - dt
						dyn.set_b(tx, ty, nb)
						dyn.set_a(tx, ty, (Geom.LOOSE_FUSE - nb) / Geom.LOOSE_FUSE)
						if nb <= 0.0:
							_break_board(tx, ty)

func _break_board(tx: int, ty: int) -> void:
	lv.set_tile(tx, ty, Level.T.SPACE)
	dyn.clear(tx, ty)
	var col: Color = lv.theme["slab_face"]
	var x := Geom.cx(tx)
	var y := Geom.surf(ty)
	var land := y
	for k in range(1, 12):
		if supported(tx, ty + k):
			land = Geom.surf(ty + k)
			break
	fx.debris(Vector2(x, y + 2.0), 16, col, land)
	fx.dust(Vector2(x, y + 4.0), 12, 1.2, Geom.shade(col, 0.7))
	shake = 0.55
	# On laisse des gravats sur le sol qui l'a reçue.
	for k in range(1, 12):
		if supported(tx, ty + k):
			if lv.tile(tx, ty + k) == Level.T.FLOOR:
				lv.set_tile(tx, ty + k, Level.T.RUBBLE)
			break

func _spawn_ambient(dt: float) -> void:
	var view := view_rect().grow(Geom.TILE_W)
	var tx0 := Geom.tx_of(view.position.x)
	var tx1 := Geom.tx_of(view.end.x) + 1
	var ty0 := Geom.ty_of(view.position.y)
	var ty1 := Geom.ty_of(view.end.y) + 1
	var hue: Color = lv.theme["torch"]
	var rate := minf(dt * 46.0, 3.0)
	for ty in range(ty0, ty1):
		for tx in range(tx0, tx1):
			if lv.tile(tx, ty) != Level.T.TORCH:
				continue
			var p := TileArt.torch_flame_pos(tx, ty)
			var n := int(rate) + (1 if rng.randf() < fmod(rate, 1.0) else 0)
			for i in n:
				fx.flame(p, 26.0, 1.0, hue)
			if rng.randf() < dt * 3.0:
				fx.smoke(p + Vector2(0, -6.0), 0.7)

# ---------------------------------------------------------------- caméra

func centre_camera() -> void:
	_update_camera(1000.0)
	cam_at = cam_target

func _update_camera(dt: float) -> void:
	var ft := player.foot_tile()
	cam_room = Geom.room_of(ft.x, ft.y)
	var tgt: Vector2
	if zoom <= 1.001:
		# Cadrage d'origine : la salle où se tient le prince.
		var r := Geom.room_rect(cam_room)
		tgt = Vector2(r.position.x + (Geom.ROOM_W - view_w) * 0.5,
			r.position.y + (Geom.ROOM_H - view_h) * 0.5)
	else:
		# Grossi : on le suit, sans jamais regarder hors du niveau.
		tgt = Vector2(player.p.x - view_w * 0.5, player.p.y - 13.0 - view_h * 0.5)
		tgt.x = clampf(tgt.x, 0.0, maxf(lv.tw * Geom.TILE_W - view_w, 0.0))
		tgt.y = clampf(tgt.y, 0.0, maxf(lv.th * Geom.TILE_H - view_h, 0.0))
	cam_target = tgt
	# Lissage exponentiel, écrit pour que le résultat ne dépende pas du pas.
	cam_at = cam_at.lerp(cam_target, 1.0 - exp(-dt * 13.0))

## Centre de la vue, secousse comprise — ce que la Camera2D doit suivre.
func camera_centre() -> Vector2:
	var o := Vector2.ZERO
	if shake > 0.0:
		o = Vector2(Geom.noise1(elapsed * 42.0, 3) * shake * 3.4,
			Geom.noise1(elapsed * 37.0, 9) * shake * 2.6)
	return cam_at + Vector2(view_w, view_h) * 0.5 + o

# ---------------------------------------------------------------- le prince

## La corniche d'un niveau au-dessus, devant le prince, s'il peut s'y hisser.
func _climb_target() -> Vector2i:
	var ft := player.foot_tile()
	var front := Geom.tx_of(player.p.x + player.facing * (Geom.BODY_HW + 4.0))
	if front == ft.x:
		return Vector2i(-9999, 0)
	if not supported(front, ft.y - 1):
		return Vector2i(-9999, 0)
	if not open(ft.x, ft.y - 1):
		return Vector2i(-9999, 0)
	return Vector2i(front, ft.y - 1)

## Une corniche à agripper en vol.
##
## On essaie d'abord celle qui est devant, puis celle qui est *derrière* — ce
## second cas est le rattrapage emblématique de l'original : partir en courant
## d'un rebord, la touche de saisie maintenue, et se raccrocher à la lèvre qu'on
## vient de quitter en se retournant pour lui faire face.
func _grab_target(probe_y: float) -> Array:
	if probe_y < 0.0:
		return []
	var br := Geom.ty_of_feet(probe_y)
	var ly := br - 1
	var hang_y := Geom.surf(ly) + Geom.HANG_DROP
	if absf(probe_y - hang_y) > 15.0:
		return []
	for spec in [[player.facing, false], [-player.facing, true]]:
		var d: float = spec[0]
		var lx := Geom.tx_of(player.p.x + d * (Geom.BODY_HW + 4.0))
		# Quelque chose à saisir, et la place pour que le corps pende dessous.
		if supported(lx, ly) and open(lx, br):
			return [lx, ly, spec[1]]
	return []

func _start_hang(lx: int, ly: int, turn: bool) -> void:
	if turn:
		# On finit face à la lèvre qu'on a rattrapée.
		player.facing = -player.facing
	player.ledge = Vector2i(lx, ly)
	player.p.y = Geom.surf(ly) + Geom.HANG_DROP
	# Les mains sur la lèvre : on aligne le corps sur le bord de la case.
	player.p.x = float(lx) * Geom.TILE_W - 2.0 if player.facing > 0.0 \
		else float(lx + 1) * Geom.TILE_W + 2.0
	player.v = Vector2.ZERO
	player.fall_from = player.p.y
	player.enter(Prince.S.HANG)
	fx.dust(Vector2(player.p.x, player.p.y - Geom.HANG_DROP + 2.0), 4, 0.6,
		lv.theme["slab_face"])

func _begin_fall() -> void:
	if Prince.airborne(player.st):
		return
	player.fall_from = player.p.y
	player.enter(Prince.S.FALL)

func _land(surf: float) -> void:
	var drop := surf - player.fall_from
	player.p.y = surf
	player.v = Vector2.ZERO
	var ft := player.foot_tile()
	var hard := drop > Geom.FALL_SAFE
	fx.dust(Vector2(player.p.x, surf - 1.0), 14 if hard else 7, 1.5 if hard else 0.8,
		Geom.shade(lv.theme["slab_face"], 0.85))
	if hard:
		shake = 0.5
	if lethal(ft.x, ft.y):
		kill_player("Empalé.")
		return
	if drop >= Geom.FALL_LETHAL and player.float_t <= 0.0:
		kill_player("La chute était trop longue.")
		return
	if drop > Geom.FALL_SAFE and player.float_t <= 0.0:
		player.hp -= 1
		flash_t = 0.6
		flash_col = Color8(140, 20, 20)
		if player.hp <= 0:
			kill_player("Brisé par la chute.")
			return
	player.enter(Prince.S.LAND)

func kill_player(cause: String) -> void:
	if phase == Phase.DYING or phase == Phase.DEAD:
		return
	player.cause = cause
	player.st = Prince.S.DEAD
	player.t = 0.0
	player.v = Vector2.ZERO
	deaths += 1
	phase = Phase.DYING
	phase_t = 2.1
	flash_t = 1.0
	flash_col = Color8(120, 10, 12)
	shake = 0.8
	fx.blood(Vector2(player.p.x, player.p.y - 16.0), player.facing, 26, player.p.y)
	say(cause, 3.0, true)

func _update_player_dead(dt: float) -> void:
	player.t += dt
	var ft := player.foot_tile()
	if supported(ft.x, ft.y):
		return
	# On laisse le corps se poser sur le sol.
	player.v.y += Geom.GRAVITY * dt
	var y1 := player.p.y + player.v.y * dt
	var ty1 := Geom.ty_of_feet(y1)
	for t in range(ft.y, maxi(ty1, ft.y) + 1):
		if Geom.surf(t) <= y1 and supported(ft.x, t):
			player.p.y = Geom.surf(t)
			player.v.y = 0.0
			return
	player.p.y = y1

func _update_player(dt: float, inp: InputState) -> void:
	var pl := player
	pl.t += dt
	pl.blend_t += dt
	pl.invuln = maxf(pl.invuln - dt, 0.0)
	pl.float_t = maxf(pl.float_t - dt, 0.0)
	pl.swift_t = maxf(pl.swift_t - dt, 0.0)
	pl.throw_cd = maxf(pl.throw_cd - dt, 0.0)
	pl.buf_jump = maxf(pl.buf_jump - dt, 0.0)
	pl.buf_attack = maxf(pl.buf_attack - dt, 0.0)
	if inp.up_edge:
		pl.buf_jump = Geom.BUFFER
	if inp.attack:
		pl.buf_attack = Geom.BUFFER
	# On lisse l'orientation visible pour qu'un demi-tour soit un mouvement et
	# non un effet miroir.
	pl.facing_vis = Geom.approach(pl.facing_vis, pl.facing, dt / 0.075)

	# ---- les armes de jet partent de presque n'importe quel appui ---------
	if not Prince.locked(pl.st) and not Prince.airborne(pl.st):
		if inp.throw_it and pl.daggers > 0 and pl.throw_cd <= 0.0:
			pl.daggers -= 1
			pl.throw_cd = 0.42
			spawn_shot(0, Vector2(pl.p.x + pl.facing * 10.0, pl.p.y - 19.0), pl.facing, true)
			pl.enter(Prince.S.THROW)
		elif inp.cast and pl.wand and pl.charges > 0:
			pl.charges -= 1
			spawn_shot(1, Vector2(pl.p.x + pl.facing * 12.0, pl.p.y - 18.0), pl.facing, true)
			pl.enter(Prince.S.CAST)
			flash_t = 0.35
			flash_col = Color8(90, 50, 10)
		elif inp.cast and pl.wand:
			say("Le bâton est éteint.", 1.4, true)
		elif inp.throw_it and pl.sword and pl.daggers == 0:
			say("Plus de dagues.", 1.2, true)

	var total := pl.clip_total()
	var done := pl.t >= total
	var enemy := nearest_enemy()

	match pl.st:
		# -------------------------------------------------------- au repos
		Prince.S.STAND, Prince.S.READY:
			var ft := pl.foot_tile()
			if not supported(ft.x, ft.y):
				_begin_fall()
			elif inp.sheathe and pl.armed:
				pl.armed = false
				pl.enter(Prince.S.STAND)
			elif inp.attack or pl.buf_attack > 0.0:
				pl.buf_attack = 0.0
				if pl.armed:
					pl.enter(Prince.S.STRIKE)
				elif pl.melee != Prince.Melee.NONE:
					pl.armed = true
					pl.enter(Prince.S.READY)
					say("Épée au clair.", 1.2, false)
				else:
					say("Tu n'as pas d'arme.", 1.4, true)
			elif inp.parry and pl.armed:
				pl.enter(Prince.S.PARRY)
			elif inp.up or pl.buf_jump > 0.0:
				pl.buf_jump = 0.0
				var l := _climb_target()
				if l.x > -9000:
					pl.anchor = pl.p
					pl.ledge = l
					pl.enter(Prince.S.CLIMB)
				else:
					pl.v.y = Geom.JUMP_UP_VY
					pl.fall_from = pl.p.y
					pl.enter(Prince.S.JUMP_UP)
			elif inp.down:
				var d := _climb_down_target()
				if (inp.careful or inp.down_edge) and d.x > -9000:
					pl.anchor = pl.p
					pl.ledge = d
					pl.enter(Prince.S.CLIMB_DOWN)
				else:
					pl.enter(Prince.S.CROUCH_IN)
			elif inp.any_dir():
				var d := inp.dir()
				if d != pl.facing:
					# En garde, reculer ne veut pas dire tourner le dos.
					pl.enter(Prince.S.RETREAT if (pl.armed and enemy >= 0) else Prince.S.TURN)
				elif inp.careful:
					_begin_step()
				elif pl.armed and enemy >= 0:
					pl.enter(Prince.S.ADVANCE)
				else:
					pl.enter(Prince.S.RUN_START)
			else:
				var want := Prince.S.READY if (pl.armed and enemy >= 0) else Prince.S.STAND
				if pl.st != want:
					pl.enter(want)

		# -------------------------------------------------------- demi-tour
		Prince.S.TURN:
			if pl.t >= total * 0.55 and not pl.struck:
				pl.facing = -pl.facing
				pl.struck = true
			if done:
				pl.enter(Prince.S.STAND)

		# -------------------------------------------------------- course
		Prince.S.RUN_START:
			_step_run(dt, pl.speed() * minf(pl.t / total, 1.0))
			if done:
				pl.enter(Prince.S.RUN)
			if inp.any_dir() and inp.dir() != pl.facing:
				pl.enter(Prince.S.RUN_STOP)
		Prince.S.RUN:
			var sp := pl.speed()
			var before := pl.p.x
			_step_run(dt, sp)
			var stuck := absf(pl.p.x - before) < sp * dt * 0.3
			if inp.up:
				pl.v.y = Geom.JUMP_RUN_VY
				pl.v.x = pl.facing * Geom.JUMP_RUN_VX
				pl.fall_from = pl.p.y
				pl.enter(Prince.S.JUMP_RUN)
			elif inp.down:
				pl.enter(Prince.S.CROUCH_IN)
			elif not inp.any_dir() or inp.dir() != pl.facing or stuck:
				pl.enter(Prince.S.RUN_STOP)
		Prince.S.RUN_STOP:
			_step_run(dt, pl.speed() * (1.0 - minf(pl.t / total, 1.0)) * 0.8)
			# Un saut demandé en pleine glissade part dès que les pieds sont
			# sous lui, au lieu d'être avalé.
			if done or (pl.buf_jump > 0.0 and pl.t > total * 0.45):
				pl.enter(Prince.S.STAND)
		Prince.S.STEP:
			var f := minf(pl.t / total, 1.0)
			var nx := pl.anchor.x + (pl.step_to - pl.anchor.x) * Geom.ease_out(f)
			pl.p.x = slide_x(pl.p.x, pl.p.y, nx - pl.p.x)
			if done:
				pl.enter(Prince.S.STAND)

		# -------------------------------------------------------- accroupi
		Prince.S.CROUCH_IN:
			if done:
				pl.enter(Prince.S.CROUCH)
		Prince.S.CROUCH:
			if not inp.down:
				pl.enter(Prince.S.CROUCH_OUT)
			elif inp.down_edge:
				var d := _climb_down_target()
				if d.x > -9000:
					pl.anchor = pl.p
					pl.ledge = d
					pl.enter(Prince.S.CLIMB_DOWN)
		Prince.S.CROUCH_OUT:
			if done:
				pl.enter(Prince.S.STAND)

		# -------------------------------------------------------- en l'air
		Prince.S.JUMP_UP, Prince.S.JUMP_RUN, Prince.S.FALL:
			_airborne_step(dt, inp)
		Prince.S.LAND:
			if done or ((pl.buf_jump > 0.0 or inp.any_dir()) and pl.t > total * 0.5):
				pl.enter(Prince.S.STAND)

		# -------------------------------------------------------- corniches
		Prince.S.HANG:
			if inp.up:
				pl.anchor = pl.p
				pl.enter(Prince.S.CLIMB)
			elif inp.down:
				pl.fall_from = pl.p.y
				pl.enter(Prince.S.FALL)
		Prince.S.CLIMB:
			var f := Geom.ease_out(minf(pl.t / total, 1.0))
			var ty := Geom.surf(pl.ledge.y)
			var tgt_x := Geom.cx(pl.ledge.x) - pl.facing * 4.0
			pl.p.y = pl.anchor.y + (ty - pl.anchor.y) * f
			pl.p.x = pl.anchor.x + (tgt_x - pl.anchor.x) * clampf(f * 1.6 - 0.4, 0.0, 1.0)
			if done:
				pl.p = Vector2(tgt_x, ty)
				pl.fall_from = ty
				pl.enter(Prince.S.STAND)
		Prince.S.CLIMB_DOWN:
			var f := minf(pl.t / total, 1.0)
			var hy := Geom.surf(pl.ledge.y) + Geom.HANG_DROP
			var hx := float(pl.ledge.x) * Geom.TILE_W - 2.0 if pl.facing > 0.0 \
				else float(pl.ledge.x + 1) * Geom.TILE_W + 2.0
			pl.p.y = pl.anchor.y + (hy - pl.anchor.y) * Geom.ease_out(f)
			pl.p.x = pl.anchor.x + (hx - pl.anchor.x) * f
			if done:
				pl.p = Vector2(hx, hy)
				pl.fall_from = hy
				pl.enter(Prince.S.HANG)

		# -------------------------------------------------------- escrime
		Prince.S.ADVANCE, Prince.S.RETREAT:
			var sign_d := pl.facing if pl.st == Prince.S.ADVANCE else -pl.facing
			var f := minf(pl.t / total, 1.0)
			# Fente en cloche, pour que le pas se lise comme une action d'escrime.
			var sp := sin(f * PI) * 46.0
			var nx := slide_x(pl.p.x, pl.p.y, sign_d * sp * dt)
			# On ne quitte jamais la corniche sur laquelle on se bat.
			if supported(Geom.tx_of(nx), pl.foot_tile().y):
				pl.p.x = nx
			if done:
				pl.enter(Prince.S.READY)
		Prince.S.STRIKE:
			var f := pl.t / total
			if not pl.struck and f >= 0.30 and f < 0.62:
				pl.struck = true
				_resolve_player_strike()
			if done:
				pl.enter(Prince.S.READY)
		Prince.S.PARRY:
			if not inp.parry:
				pl.enter(Prince.S.READY)

		# -------------------------------------------------------- divers
		Prince.S.HURT:
			pl.p.x = slide_x(pl.p.x, pl.p.y, pl.v.x * dt)
			pl.v.x *= 1.0 - 6.0 * dt
			if done:
				pl.enter(Prince.S.READY if pl.armed else Prince.S.STAND)
		Prince.S.DRINK, Prince.S.THROW, Prince.S.CAST:
			if done:
				pl.enter(Prince.S.READY if pl.armed else Prince.S.STAND)
		_:
			pass

	# ---- dangers sous les pieds ---------------------------------------
	if pl.st != Prince.S.DEAD and pl.st != Prince.S.HANG and pl.st != Prince.S.LEAVING \
			and not Prince.airborne(pl.st):
		var ft := pl.foot_tile()
		if lethal(ft.x, ft.y):
			dyn.set_flag(ft.x, ft.y, Dynamics.F_BLOODY, true)
			kill_player("Les pointes ont jailli." if lv.tile(ft.x, ft.y) == Level.T.SPIKES
				else "Les lames se sont refermées.")

	# ---- la sortie -----------------------------------------------------
	if phase == Phase.PLAY and not Prince.airborne(pl.st):
		var ft := pl.foot_tile()
		if lv.tile(ft.x, ft.y) == Level.T.EXIT and dyn.a(ft.x, ft.y) > 0.55:
			pl.st = Prince.S.LEAVING
			pl.t = 0.0
			phase = Phase.LEAVING
			phase_t = 1.4
			say("La voie est libre.", 2.0, false)

	if pl.st == Prince.S.RUN and rng.randf() < dt * 12.0:
		fx.dust(Vector2(pl.p.x - pl.facing * 5.0, pl.p.y), 1, 0.35,
			Geom.shade(lv.theme["slab_face"], 0.8))

func _begin_step() -> void:
	var pl := player
	pl.anchor = pl.p
	var want := pl.p.x + pl.facing * (Geom.TILE_W * 0.42)
	# Un pas prudent s'arrête au bord au lieu de le franchir.
	var ty := Geom.ty_of_feet(pl.p.y)
	var ntx := Geom.tx_of(want + pl.facing * Geom.BODY_HW)
	var safe := want
	if not (supported(ntx, ty) or not open(ntx, ty)):
		var edge := float(ntx) * Geom.TILE_W - Geom.BODY_HW - 1.0 if pl.facing > 0.0 \
			else float(ntx + 1) * Geom.TILE_W + Geom.BODY_HW + 1.0
		safe = minf(want, edge) if pl.facing > 0.0 else maxf(want, edge)
	pl.step_to = safe
	pl.enter(Prince.S.STEP)

## La corniche à laquelle le prince se suspendrait s'il descendait ici.
func _climb_down_target() -> Vector2i:
	var pl := player
	var ft := pl.foot_tile()
	var front := Geom.tx_of(pl.p.x + pl.facing * (Geom.BODY_HW + 4.0))
	for lx in [front, ft.x]:
		if supported(lx, ft.y):
			continue
		if not open(lx, ft.y + 1):
			continue
		# Encore faut-il avoir quelque chose sous ses propres pieds.
		if supported(ft.x, ft.y):
			return ft
	return Vector2i(-9999, 0)

## Course horizontale et vérification du bord. Le cycle avance de la distance
## réellement parcourue : c'est ce qui empêche les pieds de patiner quand il
## accélère ou boit une potion de célérité.
func _step_run(dt: float, sp: float) -> void:
	var pl := player
	var x0 := pl.p.x
	pl.p.x = slide_x(x0, pl.p.y, pl.facing * sp * dt)
	pl.gait += absf(pl.p.x - x0) / Geom.STRIDE_PX * Geom.RUN_CYCLE
	var ft := pl.foot_tile()
	if not supported(ft.x, ft.y):
		_begin_fall()

func _airborne_step(dt: float, inp: InputState) -> void:
	var pl := player
	var base := Geom.GRAVITY_JUMP if pl.st == Prince.S.JUMP_RUN else Geom.GRAVITY
	pl.v.y += (base * 0.45 if pl.float_t > 0.0 else base) * dt
	# Un peu de pilotage en l'air, comme dans l'original.
	if inp.any_dir() and pl.st != Prince.S.JUMP_UP:
		var want := inp.dir() * Geom.JUMP_RUN_VX
		pl.v.x += signf(want - pl.v.x) * 60.0 * dt
	var y0 := pl.p.y
	var y1 := y0 + pl.v.y * dt
	pl.p.x = slide_x(pl.p.x, y0, pl.v.x * dt)

	# Attraper une corniche au passage.
	if (inp.up or inp.careful or pl.st == Prince.S.JUMP_UP) and pl.v.y > -60.0:
		var l := _grab_target(y1)
		if not l.is_empty():
			_start_hang(l[0], l[1], l[2])
			return

	if pl.v.y < 0.0:
		# Se cogner au plafond.
		var head := y1 - 27.0
		var tx := Geom.tx_of(pl.p.x)
		if not open(tx, Geom.ty_of(head)):
			pl.v.y = 20.0
			pl.p.y = float(Geom.ty_of(head) + 1) * Geom.TILE_H + 27.5
			return
		pl.p.y = y1
		return

	var tx := Geom.tx_of(pl.p.x)
	var ty0 := Geom.ty_of_feet(y0)
	var ty1 := Geom.ty_of_feet(y1)
	for t in range(ty0, maxi(ty1, ty0) + 1):
		var s := Geom.surf(t)
		if s >= y0 - 0.01 and s <= y1 and supported(tx, t):
			_land(s)
			return
	pl.p.y = y1
	if pl.st != Prince.S.FALL and pl.v.y > 40.0:
		pl.st = Prince.S.FALL
		pl.t = 0.0
	if pl.p.y > float(lv.th + 2) * Geom.TILE_H:
		kill_player("Englouti par le vide.")

# ---------------------------------------------------------------- objets

func _check_items() -> void:
	if phase != Phase.PLAY or Prince.airborne(player.st):
		return
	var ft := player.foot_tile()
	var got := -1
	for i in items.size():
		var it: Dictionary = items[i]
		if not it["taken"] and it["tx"] == ft.x and it["ty"] == ft.y:
			got = i
			break
	if got < 0:
		return
	items[got]["taken"] = true
	var kind: int = items[got]["kind"]
	match kind:
		Level.Item.POTION_HEAL:
			player.hp = mini(player.hp + 1, player.hp_max)
			flash_t = 0.7; flash_col = Color8(150, 30, 40)
			player.enter(Prince.S.DRINK)
		Level.Item.POTION_LIFE:
			player.hp_max += 1
			player.hp = player.hp_max
			flash_t = 0.9; flash_col = Color8(180, 60, 110)
			player.enter(Prince.S.DRINK)
		Level.Item.POTION_FLOAT:
			player.float_t = 24.0
			flash_t = 0.7; flash_col = Color8(40, 120, 170)
			player.enter(Prince.S.DRINK)
		Level.Item.POTION_SWIFT:
			player.swift_t = 22.0
			flash_t = 0.7; flash_col = Color8(170, 150, 40)
			player.enter(Prince.S.DRINK)
		Level.Item.POTION_POISON:
			flash_t = 0.9; flash_col = Color8(60, 150, 50)
			player.enter(Prince.S.DRINK)
			player.hp -= 1
			if player.hp <= 0:
				kill_player("Le poison t'emporte.")
				return
		Level.Item.SWORD:
			player.sword = true
			if player.melee == Prince.Melee.NONE:
				player.melee = Prince.Melee.SWORD
			player.armed = true
			player.enter(Prince.S.READY)
		Level.Item.SCIMITAR:
			player.scimitar = true
			player.melee = Prince.Melee.SCIMITAR
			player.armed = true
			player.enter(Prince.S.READY)
		Level.Item.DAGGERS:
			player.daggers = mini(player.daggers + 5, 12)
		Level.Item.WAND:
			player.wand = true
			player.charges = mini(player.charges + 8, 12)
		Level.Item.BUCKLER:
			player.buckler = true
	fx.sparks(Vector2(Geom.cx(ft.x), Geom.surf(ft.y) - 6.0), 12, 0.9)
	say(Level.item_label(kind), 2.4, kind == Level.Item.POTION_POISON)
	carry = player.carry()

# ---------------------------------------------------------------- combat

## Le garde vivant le plus proche qui compte en ce moment.
func nearest_enemy() -> int:
	var best := -1
	var bd := INF
	for i in guards.size():
		var g: Guard = guards[i]
		if g.st == Guard.S.DEAD or not g.hostile():
			continue
		if absf(g.p.y - player.p.y) > Geom.TILE_H * 0.6:
			continue
		var d := absf(g.p.x - player.p.x)
		if d > Geom.TILE_W * 3.2:
			continue
		if d < bd:
			bd = d
			best = i
	return best

func _resolve_player_strike() -> void:
	var pl := player
	var reach := Prince.melee_reach(pl.melee)
	var tip := Vector2(pl.p.x + pl.facing * reach, pl.p.y - 18.0)
	fx.sparks(tip, 3, 0.5)
	slashes.append([tip, pl.facing, 0.0])
	var hit := -1
	for i in guards.size():
		var g: Guard = guards[i]
		if g.st == Guard.S.DEAD or not g.hostile():
			continue
		if absf(g.p.y - pl.p.y) > Geom.TILE_H * 0.6:
			continue
		var dx := (g.p.x - pl.p.x) * pl.facing
		if dx > 2.0 and dx < reach + 8.0:
			hit = i
			break
	if hit < 0:
		return
	var g: Guard = guards[hit]
	var parrying := g.st == Guard.S.PARRY and g.facing != pl.facing
	if parrying and rng.randf() >= Prince.melee_pierce(pl.melee):
		fx.sparks(Vector2((pl.p.x + g.p.x) * 0.5, pl.p.y - 19.0), 18, 1.6)
		shake = 0.35
		g.cool = 0.16
		return
	damage_guard(hit, Prince.melee_damage(pl.melee), pl.facing)

## Applique des dégâts à un garde.
func damage_guard(gi: int, dmg: int, dir: float) -> void:
	if gi < 0 or gi >= guards.size():
		return
	var g: Guard = guards[gi]
	if g.st == Guard.S.DEAD or not g.hostile():
		return
	var floor_y := g.p.y
	g.hp -= dmg
	g.stagger = 0.34
	g.facing = -dir
	var at := Vector2(g.p.x, g.p.y - 17.0)
	if g.hp <= 0:
		g.st = Guard.S.DEAD
		g.t = 0.0
		kills += 1
		say("%s vaincu !" % Level.mob_name(g.kind), 1.8, false)
	else:
		g.st = Guard.S.HURT
		g.t = 0.0
	if g.kind == Level.Mob.SKELETON:
		fx.debris(at, 8, Color8(222, 216, 196), floor_y)
	else:
		fx.blood(at, -dir, 16 if dmg > 1 else 10, floor_y)
	fx.sparks(at, 6, 0.8)
	shake = 0.3

func hurt_player(dmg: int, dir: float) -> void:
	if player.invuln > 0.0 or phase != Phase.PLAY:
		return
	player.hp -= dmg
	player.invuln = 0.9
	player.facing = -dir
	fx.blood(Vector2(player.p.x, player.p.y - 17.0), -dir, 12, player.p.y)
	shake = 0.6
	flash_t = 0.7
	flash_col = Color8(150, 20, 24)
	if player.hp <= 0:
		kill_player("Tu es tombé sous la lame.")
	else:
		player.st = Prince.S.HURT
		player.t = 0.0
		player.v.x = -dir * 40.0

# ---------------------------------------------------------------- projectiles

const SHOT_DAGGER := 0
const SHOT_FIREBALL := 1

func spawn_shot(kind: int, from: Vector2, dir: float, friendly: bool) -> void:
	var sp := 210.0 if kind == SHOT_DAGGER else 150.0
	shots.append({
		"kind": kind, "p": from,
		"v": Vector2(dir * sp, -8.0 if kind == SHOT_DAGGER else 0.0),
		"life": 2.6, "spin": 0.0, "friendly": friendly,
	})

func _update_shots(dt: float) -> void:
	var dead: Array[int] = []
	for s in shots:
		s["life"] -= dt
		s["spin"] += dt * 22.0 * signf(s["v"].x)
		if s["kind"] == SHOT_DAGGER:
			s["v"].y += 120.0 * dt
		s["p"] += s["v"] * dt

	for i in shots.size():
		var s: Dictionary = shots[i]
		var tx := Geom.tx_of(s["p"].x)
		var ty := Geom.ty_of(s["p"].y)
		if not open(tx, ty) or s["life"] <= 0.0:
			dead.append(i)
			if s["kind"] == SHOT_FIREBALL:
				fx.sparks(s["p"], 16, 1.5)
				shake = 0.4
			else:
				fx.sparks(s["p"], 5, 0.7)
			continue
		if s["friendly"]:
			for gi in guards.size():
				var g: Guard = guards[gi]
				if g.st == Guard.S.DEAD or not g.hostile():
					continue
				if absf(g.p.x - s["p"].x) < 11.0 and absf(g.p.y - 15.0 - s["p"].y) < 17.0:
					dead.append(i)
					fx.sparks(s["p"], 8, 1.0)
					damage_guard(gi, 1 if s["kind"] == SHOT_DAGGER else 2, signf(s["v"].x))
					break
		else:
			if absf(player.p.x - s["p"].x) < 10.0 and absf(player.p.y - 15.0 - s["p"].y) < 16.0:
				dead.append(i)
				fx.sparks(s["p"], 8, 1.0)
				var dir := signf(s["v"].x)
				# Un bouclier détourne les traits.
				if player.buckler and player.facing != dir:
					say("Le bouclier dévie le trait !", 1.6, false)
				else:
					hurt_player(1 if s["kind"] == SHOT_DAGGER else 2, dir)

	dead.sort()
	dead.reverse()
	var seen := -1
	for i in dead:
		if i == seen:
			continue
		seen = i
		if i < shots.size():
			shots.remove_at(i)
	# Les boules de feu traînent une flamme.
	for s in shots:
		if s["kind"] == SHOT_FIREBALL:
			fx.flame(s["p"], 6.0, 0.8, Color8(255, 150, 50))

# ---------------------------------------------------------------- gardes

func _update_guards(dt: float) -> void:
	var pl := player
	var player_alive := phase != Phase.DYING and phase != Phase.DEAD
	for i in guards.size():
		var g: Guard = guards[i]
		g.t += dt
		g.blend_t += dt
		g.cool = maxf(g.cool - dt, 0.0)
		g.stagger = maxf(g.stagger - dt, 0.0)
		g.facing_vis = Geom.approach(g.facing_vis, g.facing, dt / 0.075)
		var total := g.clip_total()
		var done := g.t >= total

		if g.st == Guard.S.DEAD:
			# Le corps s'affaisse sur le sol et y reste.
			var ftd := g.foot_tile()
			if not supported(ftd.x, ftd.y):
				g.v.y += Geom.GRAVITY * dt
				g.p.y += g.v.y * dt
			continue

		# ---- gravité / plancher disparu -------------------------------
		var ft := g.foot_tile()
		if not supported(ft.x, ft.y):
			if g.st != Guard.S.FALLING:
				g.enter(Guard.S.FALLING)
			g.v.y += Geom.GRAVITY * dt
			var y1 := g.p.y + g.v.y * dt
			var ty1 := Geom.ty_of_feet(y1)
			var landed := -1
			for t in range(ft.y, maxi(ty1, ft.y) + 1):
				var s := Geom.surf(t)
				if s >= g.p.y - 0.01 and s <= y1 and supported(ft.x, t):
					landed = t
					break
			if landed >= 0:
				g.p.y = Geom.surf(landed)
				g.v.y = 0.0
				g.enter(Guard.S.IDLE)
				fx.dust(Vector2(g.p.x, g.p.y), 8, 1.0, lv.theme["slab_face"])
				if lethal(ft.x, landed):
					g.hp = 0
					g.enter(Guard.S.DEAD)
					fx.blood(Vector2(g.p.x, g.p.y - 12.0), 1.0, 18, g.p.y)
			else:
				g.p.y = y1
				if g.p.y > float(lv.th + 2) * Geom.TILE_H:
					g.hp = 0
					g.st = Guard.S.DEAD
			continue
		if lethal(ft.x, ft.y):
			g.hp = 0
			g.enter(Guard.S.DEAD)
			dyn.set_flag(ft.x, ft.y, Dynamics.F_BLOODY, true)
			fx.blood(Vector2(g.p.x, g.p.y - 12.0), 1.0, 20, g.p.y)
			continue

		# ---- vigilance --------------------------------------------------
		var same_row := absf(g.p.y - pl.p.y) < Geom.TILE_H * 0.6
		var dx := pl.p.x - g.p.x
		var dist := absf(dx)
		var pft := pl.foot_tile()
		var same_room := Geom.room_of(ft.x, ft.y) == Geom.room_of(pft.x, pft.y)
		var engaged := g.hostile() and player_alive and same_row and same_room \
			and dist < Geom.TILE_W * 3.4
		g.alert = minf(g.alert + dt, 3.0) if engaged else maxf(g.alert - dt * 0.5, 0.0)

		# ---- l'Ombre n'est un ennemi que si on en fait un ---------------
		if g.kind == Level.Mob.SHADOW and not pl.armed and dist < 15.0 and same_row \
				and player_alive:
			_merge_shadow(i)
			continue

		if engaged:
			g.facing = 1.0 if dx >= 0.0 else -1.0
			var reach := Prince.melee_reach(g.melee())
			match g.st:
				Guard.S.IDLE, Guard.S.PATROL, Guard.S.READY, Guard.S.FALLING:
					g.st = Guard.S.READY
					if g.cool <= 0.0:
						g.cool = g.react() * rng.randf_range(0.7, 1.3)
						if dist > reach + 6.0:
							g.enter(Guard.S.ADVANCE)
						elif pl.st == Prince.S.STRIKE and rng.randf() < g.parry_p():
							g.enter(Guard.S.PARRY)
						elif rng.randf() < g.strike_p():
							g.enter(Guard.S.STRIKE)
							g.struck = false
						elif dist < reach * 0.7 and rng.randf() < 0.3:
							g.enter(Guard.S.RETREAT)
				Guard.S.ADVANCE, Guard.S.RETREAT:
					var sign_d := g.facing if g.st == Guard.S.ADVANCE else -g.facing
					var f := minf(g.t / total, 1.0)
					var sp := sin(f * PI) * g.walk_speed() * 1.5
					var nx := g.p.x + sign_d * sp * dt
					var ntx := Geom.tx_of(nx)
					if supported(ntx, ft.y) and open(ntx, ft.y) and not lethal(ntx, ft.y):
						g.p.x = nx
					if done:
						g.enter(Guard.S.READY)
				Guard.S.STRIKE:
					var f := g.t / total
					if not g.struck and f >= 0.30 and f < 0.62:
						g.struck = true
						_resolve_guard_strike(i)
					if done:
						g.enter(Guard.S.READY)
				Guard.S.PARRY:
					if done:
						g.enter(Guard.S.READY)
				Guard.S.HURT:
					if g.stagger <= 0.0 and done:
						g.enter(Guard.S.READY)
					var nx := g.p.x - g.facing * 24.0 * dt
					if supported(Geom.tx_of(nx), ft.y) and open(Geom.tx_of(nx), ft.y):
						g.p.x = nx
		else:
			# ---- ronde ---------------------------------------------------
			match g.st:
				Guard.S.HURT:
					if done:
						g.enter(Guard.S.IDLE)
				Guard.S.PATROL:
					var sp := g.walk_speed() * 0.6
					var nx := g.p.x + g.dir * sp * dt
					var ntx := Geom.tx_of(nx + g.dir * Geom.BODY_HW)
					var ok := supported(ntx, ft.y) and open(ntx, ft.y) \
						and not lethal(ntx, ft.y) \
						and absf(nx - g.home.x) < Geom.TILE_W * 1.8
					if ok:
						g.gait += absf(nx - g.p.x) / Geom.GUARD_STRIDE * Geom.WALK_CYCLE
						g.p.x = nx
						g.facing = g.dir
					else:
						g.dir = -g.dir
						g.enter(Guard.S.IDLE)
						g.idle = rng.randf_range(0.8, 2.4)
				_:
					g.st = Guard.S.IDLE
					g.idle -= dt
					if g.idle <= 0.0:
						g.idle = rng.randf_range(1.4, 3.6)
						if g.patrols():
							g.enter(Guard.S.PATROL)
							g.dir = -1.0 if rng.randf() < 0.5 else 1.0

	# On retire les gardes tombés hors du monde.
	guards = guards.filter(func(g): return g.p.y < float(lv.th + 4) * Geom.TILE_H)

func _merge_shadow(i: int) -> void:
	if i >= guards.size():
		return
	var g: Guard = guards[i]
	var at := g.p
	g.st = Guard.S.DEAD
	g.hp = 0
	g.p.y = -9999.0
	player.hp_max += 1
	player.hp = player.hp_max
	carry = player.carry()
	flash_t = 1.0
	flash_col = Color8(70, 60, 130)
	fx.sparks(Vector2(at.x, at.y - 18.0), 40, 1.6)
	say("Vous n'êtes qu'un. Ta vigueur grandit.", 4.0, false)

func _resolve_guard_strike(gi: int) -> void:
	var g: Guard = guards[gi]
	var pl := player
	if phase == Phase.DYING or phase == Phase.DEAD:
		return
	var reach := Prince.melee_reach(g.melee())
	var dx := (pl.p.x - g.p.x) * g.facing
	slashes.append([Vector2(g.p.x + g.facing * reach, g.p.y - 18.0), g.facing, 0.0])
	if not (dx > 0.0 and dx < reach + 8.0) or absf(pl.p.y - g.p.y) > Geom.TILE_H * 0.6:
		return
	# Parade : un blocage explicite marche toujours si l'on fait face au coup ;
	# un bouclier laisse une chance même sans bloquer.
	var facing_it := pl.facing != g.facing
	var blocked := false
	if pl.st == Prince.S.PARRY and facing_it:
		blocked = true
	elif pl.buckler and facing_it:
		blocked = rng.randf() < 0.4
	if blocked and rng.randf() >= Prince.melee_pierce(g.melee()):
		fx.sparks(Vector2((pl.p.x + g.p.x) * 0.5, pl.p.y - 19.0), 20, 1.7)
		shake = 0.4
		g.cool = 0.22
		say("Paré !", 0.8, false)
		return
	hurt_player(Prince.melee_damage(g.melee()), g.facing)

## Fraction de vie d'un adversaire de marque, pour la jauge de l'interface.
func boss() -> Array:
	for g in guards:
		if g.st == Guard.S.DEAD:
			continue
		if g.kind == Level.Mob.JAFFAR or g.kind == Level.Mob.VIZIER:
			return [Level.mob_name(g.kind), clampf(float(g.hp) / g.hp_max, 0.0, 1.0)]
	return []

# ---------------------------------------------------------------- lumières

static func _make_light_texture() -> Texture2D:
	var g := Gradient.new()
	g.offsets = PackedFloat32Array([0.0, 0.32, 0.68, 1.0])
	g.colors = PackedColorArray([
		Color(1, 1, 1, 1), Color(1, 1, 1, 0.66),
		Color(1, 1, 1, 0.20), Color(1, 1, 1, 0.0),
	])
	var t := GradientTexture2D.new()
	t.gradient = g
	t.fill = GradientTexture2D.FILL_RADIAL
	t.fill_from = Vector2(0.5, 0.5)
	t.fill_to = Vector2(1.0, 0.5)
	t.width = 128
	t.height = 128
	return t

func _light(i: int) -> PointLight2D:
	while _lights.size() <= i:
		var l := PointLight2D.new()
		l.texture = _light_tex
		l.blend_mode = Light2D.BLEND_MODE_ADD
		l.shadow_enabled = false
		add_child(l)
		_lights.append(l)
	return _lights[i]

func _sync_lights() -> void:
	if lv == null:
		return
	var srcs := TileArt.collect_lights(lv, dyn, view_rect().grow(Geom.TILE_W * 2.0), elapsed)
	srcs.append_array(fx.emitter_lights())
	for s in shots:
		if s["kind"] == SHOT_FIREBALL:
			srcs.append([s["p"], 46.0, Color8(255, 150, 60), 1.1])
	# Le prince porte une très faible lumière propre : il ne disparaît jamais
	# complètement dans le noir.
	srcs.append([Vector2(player.p.x, player.p.y - 15.0), 52.0, Color8(196, 202, 226), 0.42])
	var n := mini(srcs.size(), 48)
	for i in n:
		var s: Array = srcs[i]
		var l := _light(i)
		l.visible = true
		l.position = s[0]
		l.texture_scale = maxf(s[1], 1.0) / 64.0
		l.color = s[2]
		l.energy = s[3]
	for i in range(n, _lights.size()):
		_lights[i].visible = false

# ---------------------------------------------------------------- rendu

func _draw() -> void:
	if lv == null:
		return
	var view := view_rect()
	# Fond lointain, pour qu'aucun pixel ne reste noir.
	draw_rect(view.grow(Geom.TILE_W * 3.0), Geom.shade(lv.theme["back_dk"], 0.7))
	TileArt.draw_environment(self, lv, dyn, view, elapsed)
	for it in items:
		if it["taken"]:
			continue
		var bob := sin(elapsed * 2.2 + it["tx"]) * 0.7
		ItemArt.draw_item(self, it["kind"], Geom.cx(it["tx"]), Geom.surf(it["ty"]), bob)

## Les personnages, peints dans le CanvasGroup détouré.
func draw_figures(ci: CanvasItem) -> void:
	if lv == null:
		return
	for g in guards:
		if g.p.y < -1000.0:
			continue
		_draw_actor(ci, g.prop(), g.style(), g.pose(), g.p, g.facing_vis, g.blade())

	var pstyle := Skel.prince_style()
	if player.swift_t > 0.0:
		pstyle.sash = Color8(240, 200, 60)
		pstyle.sash_dk = Color8(160, 120, 20)
	if player.float_t > 0.0:
		pstyle.cloth = Color8(226, 238, 250)
		pstyle.cloth_dk = Color8(150, 178, 210)
	# Invulnérabilité : on clignote en sautant des images plutôt qu'en baissant
	# l'opacité — un personnage fait de formes qui se recouvrent devient illisible
	# dès qu'il est translucide, chaque forme se voyant à travers la précédente.
	var blink := player.invuln > 0.0 and fmod(elapsed * 11.0, 1.0) < 0.42
	if not blink:
		_draw_actor(ci, Skel.prince_prop(), pstyle, player.pose(), player.p,
			player.facing_vis, player.blade())

func _draw_actor(ci: CanvasItem, prop: Skel.Prop, style: Skel.Style, pose: Skel.Pose,
		feet: Vector2, facing: float, blade: int) -> void:
	var f := Skel.solve(pose, prop, feet, facing)
	# Ombre portée au sol, sous les pieds.
	var sw := 9.0 * prop.scale * maxf(prop.girth, 0.8)
	var sh := PackedVector2Array()
	for i in 12:
		var a := TAU * i / 12.0
		sh.append(Vector2(feet.x + cos(a) * sw, feet.y - 0.5 + sin(a) * 2.4))
	Shape.poly(ci, sh, Color(0, 0, 0, 0.42))
	Skel.draw_figure(ci, f, style, pose, blade)

## Passe émissive : tout ce qui émet de la lumière, peint après l'ambiance.
func draw_emissive(ci: CanvasItem) -> void:
	if lv == null:
		return
	for it in items:
		if it["taken"]:
			continue
		var bob := sin(elapsed * 2.2 + it["tx"]) * 0.7
		var pulse := sin(elapsed * 3.0 + it["ty"])
		ItemArt.draw_item_glow(ci, it["kind"], Geom.cx(it["tx"]), Geom.surf(it["ty"]),
			bob, pulse)
	for g in guards:
		if g.kind == Level.Mob.SHADOW and g.st != Guard.S.DEAD:
			ItemArt.draw_shadow_aura(ci, Vector2(g.p.x, g.p.y - 16.0), 24.0)
	fx.draw_light(ci)
	for s in shots:
		if s["kind"] == SHOT_DAGGER:
			ItemArt.draw_dagger_flight(ci, s["p"], s["spin"])
		else:
			ItemArt.draw_fireball(ci, s["p"], 4.6, sin(elapsed * 22.0))
	for s in slashes:
		ItemArt.draw_slash(ci, s[0], s[1], s[2])
