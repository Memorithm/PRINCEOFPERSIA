## Prince of Persia — la légende du miroir d'argent (aventure vue du dessus).
##
## Portage Godot 4 du moteur Rust (src/adventure) : même simulation (pas fixe
## 120 Hz, 8 directions, IA des 10 espèces, quête des 4 fragments), rendu
## vectoriel lisse via draw_* + MSAA, lumières additives, HUD vectoriel.
extends Node2D

const AD = AdventureData
const TILE := 24.0  # == AdventureData.TILE

# ---------------------------------------------------------------- tuiles
enum T { FLOOR, GRASS, FLOWER, WALL, TREE, BUSH, WATER, BRIDGE, PIT, STATUE, BRAZIER, DOOR, SEALED, PORTAL, CAVE, MIRROR }

# ---------------------------------------------------------------- êtres
enum F { CRAB, GRUNT, BAT, DJINN, KNIGHT, OGRE, BOSS, GOLEM, PHARAOH, GANAR }
enum PK { HEART, GEM, KEY, MIRROR, SWORD, HEART_CONTAINER }

enum Dir { DOWN, UP, LEFT, RIGHT }

# ---------------------------------------------------------------- réglages
const PLAYER_SPEED := 92.0
const PLAYER_RADIUS := 7.0
const ATTACK_TIME := 0.18
const ATTACK_COOLDOWN := 0.26
const SWORD_REACH := 18.0
const SWORD_ARC := 16.0
const FOE_KNOCKBACK := 170.0
const SHOT_SPEED_STONE := 95.0
const SHOT_SPEED_FIRE := 78.0
const HURT_INVULN := 1.0
const KNOCKBACK := 130.0
const SIM_DT := 1.0 / 120.0
const SEAL_NEEDED := 4
const VIEWS_TALL := [6.0, 8.0, 10.0, 13.0, 16.0]

# ---------------------------------------------------------------- état
var worlds: Array = []            # {tw, th, tiles:Array[int], spawns, pickups, npc, start}
var cur := AD.LIGHT_REALM
var inv := {hp = 6, hp_max = 6, gems = 0, keys = 0, mirror = false, sword = false, seals = 0}
var pl := {p = Vector2.ZERO, facing = Dir.DOWN, moving = false, anim = 0.0,
	attack_t = 0.0, cooldown = 0.0, invuln = 0.0, knock = Vector2.ZERO}
var foes: Array = []
var shots: Array = []
var parts: Array = []             # particules {p, v, life, max, col, size}
var cam := Vector2.ZERO
var cam_shake := 0.0
var phase := "play"               # play | dying | dead | victory
var dead_t := 0.0
var paused := false
var msg: String = ""
var msg_t := 0.0
var msg_warn := false
var rng := RandomNumberGenerator.new()
var time := 0.0
var elapsed := 0.0
var kills := 0
var deaths := 0
var ganar_dead := false
var warp_grace := 0.0
var seal_cd := 0.0
var hint_ix := 0
var hint_cd := 0.0
var zoom_ix := 2
var acc := 0.0

var cam_node: Camera2D
var light_layer: Node2D
var hud: Control


func _ready() -> void:
	rng.seed = 0x5EED
	for i in AD.WORLD_COUNT:
		worlds.append(_parse_world(i))
	var cam2d := Camera2D.new()
	cam2d.name = "Cam"
	add_child(cam2d)
	cam_node = cam2d
	light_layer = Node2D.new()
	light_layer.name = "Lights"
	var mat := CanvasItemMaterial.new()
	mat.blend_mode = CanvasItemMaterial.BLEND_MODE_ADD
	light_layer.material = mat
	add_child(light_layer)
	light_layer.draw.connect(_draw_lights)
	var layer := CanvasLayer.new()
	layer.name = "HUDLayer"
	add_child(layer)
	hud = preload("res://hud.gd").new()
	hud.game = self
	layer.add_child(hud)
	var bg: Color = AD.CLEAR_COLORS[cur]
	RenderingServer.set_default_clear_color(bg)
	_enter_world(AD.LIGHT_REALM, null)
	_say("La Vallée d'Ispahan — retrouve la princesse Zahra !", 5.0, false)


# ---------------------------------------------------------------- parsing
func _tile_from_char(c: String) -> int:
	match c:
		".", "P":
			return T.FLOOR
		",":
			return T.GRASS
		'"':
			return T.FLOWER
		"#", "R":
			return T.WALL
		"T":
			return T.TREE
		"B":
			return T.BUSH
		"~":
			return T.WATER
		"%":
			return T.BRIDGE
		":":
			return T.PIT
		"S":
			return T.STATUE
		"t":
			return T.BRAZIER
		"D":
			return T.DOOR
		"L":
			return T.SEALED
		"X":
			return T.PORTAL
		"C":
			return T.CAVE
		"M":
			return T.MIRROR
	return -1


func _foe_from_char(c: String) -> int:
	match c:
		"o": return F.CRAB
		"m": return F.GRUNT
		"b": return F.BAT
		"z": return F.DJINN
		"n": return F.KNIGHT
		"1": return F.OGRE
		"2": return F.BOSS
		"3": return F.GOLEM
		"4": return F.PHARAOH
		"9": return F.GANAR
	return -1


func _pk_from_char(c: String) -> int:
	match c:
		"h": return PK.HEART
		"g": return PK.GEM
		"k": return PK.KEY
		"i": return PK.MIRROR
		"e": return PK.SWORD
		"H": return PK.HEART_CONTAINER
	return -1


func _parse_world(ix: int) -> Dictionary:
	var rows: Array = AD.MAPS[ix]
	var th := rows.size()
	var tw := rows[0].length()
	var tiles := []
	tiles.resize(tw * th)
	var spawns: Array = []
	var pickups: Array = []
	var npc = null
	var start := Vector2(AD.STARTS[ix].x + 0.5, AD.STARTS[ix].y + 0.5) * TILE
	for y in th:
		var row: String = rows[y]
		for x in tw:
			var c := row[x]
			var t := _tile_from_char(c)
			var wx := (x + 0.5) * TILE
			var wy := (y + 0.5) * TILE
			if t >= 0:
				tiles[y * tw + x] = t
				continue
			tiles[y * tw + x] = T.FLOOR
			var fk := _foe_from_char(c)
			if fk >= 0:
				spawns.append({"kind": fk, "home": Vector2(wx, wy)})
			elif _pk_from_char(c) >= 0:
				pickups.append({"kind": _pk_from_char(c), "p": Vector2(wx, wy), "taken": false, "bob": rng.randf() * 6.0})
			elif c == "P":
				start = Vector2(wx, wy)
			elif c == "V" or c == "F" or c == "Y":
				npc = {"ch": c, "p": Vector2(wx, wy)}
	return {"tw": tw, "th": th, "tiles": tiles, "spawns": spawns,
		"pickups": pickups, "npc": npc, "start": start, "ix": ix}


# ---------------------------------------------------------------- accès monde
func world() -> Dictionary:
	return worlds[cur]


func tile(tx: int, ty: int) -> int:
	var w := world()
	if tx < 0 or ty < 0 or tx >= w.tw or ty >= w.th:
		return T.WALL
	return w.tiles[ty * w.tw + tx]


func set_tile(tx: int, ty: int, t: int) -> void:
	var w := world()
	if tx >= 0 and ty >= 0 and tx < w.tw and ty < w.th:
		w.tiles[ty * w.tw + tx] = t


func solid(t: int) -> bool:
	return t == T.WALL or t == T.TREE or t == T.BUSH or t == T.WATER \
		or t == T.PIT or t == T.STATUE or t == T.BRAZIER or t == T.DOOR or t == T.SEALED


func theme() -> String:
	return AD.THEMES[cur]


func pal() -> Array:
	return AD.PALETTES[cur]


func pcol(i: int) -> Color:
	return AD.PALETTES[cur][i]


# ---------------------------------------------------------------- vie du jeu
func _enter_world(ix: int, at = null) -> void:
	cur = clampi(ix, 0, AD.WORLD_COUNT - 1)
	var w := world()
	pl.p = at if at != null else w.start
	pl.knock = Vector2.ZERO
	pl.invuln = 0.8
	warp_grace = 0.45
	foes.clear()
	if not (ganar_dead):
		for s in w.spawns:
			foes.append(_make_foe(s.kind, s.home))
	else:
		for s in w.spawns:
			if s.kind != F.GANAR:
				foes.append(_make_foe(s.kind, s.home))
	shots.clear()
	RenderingServer.set_default_clear_color(AD.CLEAR_COLORS[cur])
	_snap_camera()


func _make_foe(kind: int, home: Vector2) -> Dictionary:
	return {kind = kind, home = home, p = home, hp = _foe_max_hp(kind),
		dir = Vector2(0, 1), t = rng.randf_range(0.0, 1.0), cd = rng.randf_range(0.4, 1.6),
		hurt_t = 0.0, knock = Vector2.ZERO, alive = true, anim = rng.randf() * 10.0}


func _foe_max_hp(kind: int) -> int:
	match kind:
		F.CRAB, F.BAT: return 1
		F.GRUNT, F.DJINN: return 2
		F.KNIGHT: return 2
		F.OGRE: return 8
		F.BOSS: return 12
		F.GOLEM, F.PHARAOH: return 14
		F.GANAR: return 16
	return 1


func _foe_touch(kind: int) -> int:
	return 2 if kind >= F.OGRE else 1


func _foe_name(kind: int) -> String:
	match kind:
		F.CRAB: return "un Craboroc"
		F.GRUNT: return "un Sbire"
		F.BAT: return "une Chauve-Kese"
		F.DJINN: return "un Djinn des flots"
		F.KNIGHT: return "un Garde noir"
		F.OGRE: return "la Brute des geôles"
		F.BOSS: return "le Garde noir royal"
		F.GOLEM: return "le Golem de cristal"
		F.PHARAOH: return "le Pharaon momifié"
		F.GANAR: return "le Vizir Ganar"
	return "???"


func _say(text: String, t: float, warn: bool) -> void:
	msg = text
	msg_t = t
	msg_warn = warn


func respawn() -> void:
	inv.hp = inv.hp_max
	phase = "play"
	dead_t = 0.0
	_enter_world(cur, null)


func _grow_heart() -> void:
	inv.hp_max += 2
	inv.hp = inv.hp_max


# ---------------------------------------------------------------- boucle
func _process(delta: float) -> void:
	delta = minf(delta, 0.1)
	time += delta
	if Input.is_action_just_pressed("zoom_in"):
		zoom_ix = mini(zoom_ix + 1, VIEWS_TALL.size() - 1)
	if Input.is_action_just_pressed("zoom_out"):
		zoom_ix = maxi(zoom_ix - 1, 0)
	if Input.is_action_just_pressed("pause"):
		paused = not paused
	if Input.is_action_just_pressed("restart") and phase != "victory":
		respawn()
	if phase == "victory":
		if Input.is_action_just_pressed("restart") or Input.is_action_just_pressed("attack"):
			get_tree().reload_current_scene()
		queue_redraw()
		hud.queue_redraw()
		return
	if Input.is_action_just_pressed("quit"):
		get_tree().quit()

	if not paused:
		if phase == "play":
			acc = minf(acc + delta, 0.25)
			while acc >= SIM_DT:
				_step(SIM_DT, Input.is_action_pressed("attack"))
				acc -= SIM_DT
		elif phase == "dying":
			dead_t += delta
			if dead_t > 1.4:
				respawn()
	flash_t = maxf(flash_t - delta * 2.2, 0.0)
	elapsed += delta
	warp_grace = maxf(warp_grace - delta, 0.0)
	seal_cd = maxf(seal_cd - delta, 0.0)
	hint_cd = maxf(hint_cd - delta, 0.0)
	msg_t -= delta
	if msg_t <= 0.0:
		msg = ""
	cam_shake = maxf(cam_shake - delta * 1.6, 0.0)
	_update_particles(delta)
	_update_camera(delta)
	queue_redraw()
	light_layer.queue_redraw()
	hud.queue_redraw()


func _step(dt: float, attack: bool) -> void:
	_update_player(dt, attack)
	_update_foes(dt)
	_update_shots(dt)
	_check_pickups()
	_check_tiles()
	_check_npc()
	if phase == "play" and inv.hp <= 0:
		inv.hp = 0
		phase = "dying"
		dead_t = 0.0
		deaths += 1


# ---------------------------------------------------------------- joueur
func _dir_vec(d: int) -> Vector2:
	match d:
		Dir.DOWN: return Vector2(0, 1)
		Dir.UP: return Vector2(0, -1)
		Dir.LEFT: return Vector2(-1, 0)
		Dir.RIGHT: return Vector2(1, 0)
	return Vector2(0, 1)


func _try_move(p: Vector2, dx: float, dy: float) -> Vector2:
	var r := PLAYER_RADIUS
	var cand := p + Vector2(dx, dy)
	if not _hit_solid(cand, r):
		return cand
	if dx == 0.0 and dy == 0.0:
		return p
	var nx := p.x
	var ny := p.y
	if dx != 0.0:
		nx = _slide_axis(cand.x + r, dx) if dx > 0.0 else _slide_axis(cand.x - r, dx)
	if dy != 0.0:
		ny = _slide_axis(cand.y + r, dy) if dy > 0.0 else _slide_axis(cand.y - r, dy)
	var probe := Vector2(nx if dx != 0.0 else p.x, ny if dy != 0.0 else p.y)
	if not _hit_solid(probe, r):
		return probe
	return p


func _slide_axis(edge: float, d: float) -> float:
	# Colle au bord de la tuile franchie, rayon déduit.
	var tile_edge := (floorf(edge / TILE)) * TILE if d > 0.0 else (floorf(edge / TILE) + 1.0) * TILE
	return tile_edge - PLAYER_RADIUS * signf(d) - 0.01 * signf(d)


func _hit_solid(p: Vector2, r: float) -> bool:
	var x0 := int(floorf((p.x - r) / TILE))
	var x1 := int(floorf((p.x + r) / TILE))
	var y0 := int(floorf((p.y - r) / TILE))
	var y1 := int(floorf((p.y + r) / TILE))
	for ty in range(y0, y1 + 1):
		for tx in range(x0, x1 + 1):
			if solid(tile(tx, ty)):
				return true
	return false


func _update_player(dt: float, attack: bool) -> void:
	pl.attack_t = maxf(pl.attack_t - dt, 0.0)
	pl.cooldown = maxf(pl.cooldown - dt, 0.0)
	pl.invuln = maxf(pl.invuln - dt, 0.0)
	pl.knock *= exp(-dt * 9.0)
	if pl.knock.length() > 4.0:
		var k := pl.knock * dt
		pl.p = _try_move(pl.p, k.x, 0.0)
		pl.p = _try_move(pl.p, 0.0, k.y)

	var mx := Input.get_action_strength("move_right") - Input.get_action_strength("move_left")
	var my := Input.get_action_strength("move_down") - Input.get_action_strength("move_up")
	var want := Vector2(mx, my)
	pl.moving = want.length() > 0.01 and pl.attack_t <= ATTACK_TIME * 0.45
	if pl.moving:
		var d := want.normalized() * PLAYER_SPEED * dt
		if absf(mx) >= absf(my) and mx != 0.0:
			pl.facing = Dir.RIGHT if mx > 0.0 else Dir.LEFT
		elif my != 0.0:
			pl.facing = Dir.DOWN if my > 0.0 else Dir.UP
		pl.p = _try_move(pl.p, d.x, 0.0)
		pl.p = _try_move(pl.p, 0.0, d.y)
		pl.anim += dt * 9.0
		if rng.randf() < dt * 6.0:
			var col := Color(0.59, 0.5, 0.38)
			if theme() in ["Valley", "Shadow", "Forest", "Swamp", "Oasis"]:
				col = Color(0.59, 0.5, 0.38)
			elif theme() in ["Desert", "Pass"]:
				col = Color(0.75, 0.65, 0.45)
			_dust(pl.p + Vector2(0, 6), 1, 0.35, col)

	var swung := attack and pl.cooldown <= 0.0
	if swung:
		pl.attack_t = ATTACK_TIME
		pl.cooldown = ATTACK_COOLDOWN
		_resolve_sword()


func _resolve_sword() -> void:
	var dmg := 2 if inv.sword else 1
	var f := _dir_vec(pl.facing)
	var tip := pl.p + f * SWORD_REACH
	for i in foes.size():
		var foe: Dictionary = foes[i]
		if not foe.alive:
			continue
		var d: float = foe.p.distance_to(tip)
		var near: float = foe.p.distance_to(pl.p)
		if d < SWORD_ARC or near < SWORD_ARC + 4.0:
			var frontal: bool = f.dot(foe.dir.normalized() if foe.dir.length() > 0.1 else Vector2(0, 1)) < -0.55 \
				and (foe.kind == F.KNIGHT or foe.kind == F.BOSS)
			if frontal:
				_sparks(foe.p + Vector2(0, -2), 6, 0.7, Color(0.8, 0.8, 0.85))
				continue
			foe.hp -= dmg
			foe.hurt_t = 0.18
			foe.dir = -f
			var kb := 0.45 if _is_boss(foe.kind) else 1.0
			foe.knock = f * FOE_KNOCKBACK * kb
			foe.t = maxf(foe.t, 0.28)
			_sparks(foe.p + Vector2(0, -4), 12 if dmg > 1 else 8, 1.0, Color(1, 0.95, 0.7))
			if foe.hp <= 0:
				_kill_foe(i)


func _is_boss(kind: int) -> bool:
	return kind >= F.OGRE


func _kill_foe(ix: int) -> void:
	var foe: Dictionary = foes[ix]
	var kind: int = foe.kind
	var fp: Vector2 = foe.p
	foe.alive = false
	kills += 1
	cam_shake = maxf(cam_shake, 0.35)
	_sparks(fp, 16, 1.4, Color(1, 0.9, 0.6))
	_dust(fp, 8, 1.0, Color(0.63, 0.59, 0.55))
	match kind:
		F.OGRE:
			inv.seals = maxi(inv.seals, 1)
			_grow_heart()
			_say("La Brute s'effondre — fragment du sceau 1/4 ! (+1 cœur)", 4.5, false)
			_flash(Color(0.94, 0.78, 0.35))
		F.BOSS:
			if cur == AD.PALACE:
				inv.seals = maxi(inv.seals, 2)
				_grow_heart()
				_say("Le Garde royal tombe — fragment du sceau 2/4 ! (+1 cœur)", 4.5, false)
			else:
				_grow_heart()
				inv.gems += 10
				_say("Le garde d'élite tombe ! (+1 cœur, +10 gemmes)", 4.0, false)
			_flash(Color(0.94, 0.78, 0.35))
		F.GOLEM:
			inv.seals = maxi(inv.seals, 3)
			_grow_heart()
			_say("Le Golem se fissure — fragment du sceau 3/4 ! (+1 cœur)", 4.5, false)
			_flash(Color(0.63, 0.86, 1.0))
			cam_shake = 0.9
		F.PHARAOH:
			inv.seals = 4
			_grow_heart()
			_say("LE SCEAU EST RECOMPOSÉ (4/4) ! Direction le Sanctuaire…", 5.0, false)
			_flash(Color(1.0, 0.84, 0.43))
			cam_shake = 0.9
		F.GANAR:
			ganar_dead = true
			_say("GANAR EST DÉTRUIT ! La princesse est libérée…", 6.0, false)
			_flash(Color(1.0, 0.9, 0.47))
			cam_shake = 1.2
		_:
			if rng.randf() < 0.22:
				var kind2 := PK.HEART if rng.randf() < 0.5 else PK.GEM
				worlds[cur].pickups.append({kind = kind2, p = fp, taken = false, bob = 0.0})


func _flash(c: Color) -> void:
	flash_col = c
	flash_t = 0.5

var flash_col := Color(0, 0, 0)
var flash_t := 0.0


# ---------------------------------------------------------------- IA
func _update_foes(dt: float) -> void:
	var pl_p: Vector2 = pl.p
	var invuln: bool = pl.invuln > 0.0
	var pending: Array = []
	var contacts: Array = []
	var spawn_bats: Array = []

	for foe in foes:
		if not foe.alive:
			continue
		foe.anim += dt * (14.0 if foe.kind == F.BAT else 6.0)
		foe.hurt_t = maxf(foe.hurt_t - dt, 0.0)
		foe.knock *= exp(-dt * 9.0)
		foe.cd -= dt
		foe.t -= dt
		var to_pl: Vector2 = pl_p - foe.p
		var dist: float = to_pl.length()
		var speed := 0.0
		match foe.kind:
			F.CRAB: speed = 26.0
			F.GRUNT: speed = 24.0
			F.BAT: speed = 46.0
			F.KNIGHT: speed = 30.0
			F.OGRE: speed = 40.0
			F.BOSS: speed = 26.0
			F.GOLEM: speed = 17.0
			F.PHARAOH: speed = 34.0
		var vel := Vector2.ZERO
		match foe.kind:
			F.CRAB:
				if foe.t <= 0.0:
					foe.t = rng.randf_range(0.6, 1.6)
					var dirs := [Vector2(1, 0), Vector2(-1, 0), Vector2(0, 1), Vector2(0, -1)]
					foe.dir = Vector2.ZERO if rng.randf() < 0.25 else dirs[rng.randi_range(0, 3)]
				vel = foe.dir * speed
				if foe.cd <= 0.0 and dist < 150.0 and (absf(to_pl.x) < 12.0 or absf(to_pl.y) < 12.0):
					foe.cd = rng.randf_range(2.0, 3.2)
					pending.append({"p": foe.p + to_pl.normalized() * 6.0, "v": to_pl.normalized() * SHOT_SPEED_STONE, "fire": false, "life": 2.2})
			F.GRUNT, F.OGRE:
				var aggro := dist < (150.0 if foe.kind == F.OGRE else 110.0)
				if aggro:
					if foe.t <= 0.0:
						foe.t = rng.randf_range(0.35, 0.6)
						foe.dir = to_pl.normalized()
					vel = foe.dir * (speed if foe.kind == F.OGRE else speed * 2.2)
				elif foe.t <= 0.0:
					foe.t = rng.randf_range(0.8, 1.8)
					var a := rng.randf_range(0.0, TAU)
					foe.dir = Vector2(sin(a), cos(a))
					if rng.randf() < 0.3:
						foe.dir = Vector2.ZERO
					vel = foe.dir * speed
			F.BAT:
				if dist < 190.0 and dist > 1.0:
					var wob := (foe.p - pl_p).normalized().orthogonal() * sin(foe.anim * 0.9) * 0.8
					vel = (to_pl.normalized() + wob).normalized() * speed
				elif foe.t <= 0.0:
					foe.t = rng.randf_range(0.7, 1.4)
					var a2 := rng.randf_range(0.0, TAU)
					foe.dir = Vector2(sin(a2), cos(a2))
				if vel.length() < 1.0:
					vel = foe.dir * speed * 0.6
			F.DJINN:
				if foe.cd <= 0.0 and dist < 170.0:
					foe.cd = rng.randf_range(2.4, 3.4)
					pending.append({"p": foe.p + Vector2(0, -6), "v": to_pl.normalized() * SHOT_SPEED_FIRE, "fire": true, "life": 3.0})
			F.KNIGHT, F.BOSS:
				if dist > 2.0:
					if absf(to_pl.x) > absf(to_pl.y):
						vel = Vector2(signf(to_pl.x), 0) * speed
					else:
						vel = Vector2(0, signf(to_pl.y)) * speed
					foe.dir = vel.normalized()
				if foe.kind == F.BOSS and foe.cd <= 0.0 and dist < 130.0:
					foe.cd = rng.randf_range(2.6, 3.6)
					foe.t = 0.35
				if foe.kind == F.BOSS and foe.t > 0.0:
					vel = foe.dir * speed * 3.4
			F.GOLEM:
				if dist < 170.0:
					if foe.t <= 0.0:
						foe.t = rng.randf_range(0.5, 0.8)
						foe.dir = to_pl.normalized()
					vel = foe.dir * speed
				if foe.cd <= 0.0 and dist < 120.0:
					foe.cd = rng.randf_range(3.2, 4.2)
					cam_shake = 0.6
					for k in 8:
						var a3 := k * TAU / 8.0
						pending.append({"p": foe.p + Vector2(sin(a3), cos(a3)) * 7.0,
							"v": Vector2(sin(a3), cos(a3)) * SHOT_SPEED_STONE, "fire": false, "life": 1.6})
			F.PHARAOH:
				if dist < 180.0:
					if foe.t <= 0.0:
						foe.t = rng.randf_range(0.4, 0.7)
						foe.dir = to_pl.normalized()
					vel = foe.dir * speed
				elif foe.t <= 0.0:
					foe.t = rng.randf_range(0.9, 1.6)
					var a4 := rng.randf_range(0.0, TAU)
					foe.dir = Vector2(sin(a4), cos(a4))
				if foe.cd <= 0.0 and dist < 160.0:
					foe.cd = rng.randf_range(2.2, 3.0)
					var a5 := rng.randf_range(0.0, TAU)
					var target: Vector2 = foe.home + Vector2(sin(a5), cos(a5)) * Vector2(TILE * 2.2, TILE * 1.6)
					if _free_at(target, 8.0):
						_sparks(foe.p, 10, 0.8, Color(0.6, 1, 0.7))
						foe.p = target
						_sparks(target, 10, 0.8, Color(0.6, 1, 0.7))
					pending.append({"p": foe.p + Vector2(0, -6), "v": to_pl.normalized() * SHOT_SPEED_FIRE, "fire": true, "life": 2.6})
					if rng.randf() < 0.35:
						spawn_bats.append(foe.p + Vector2(TILE, 0))
			F.GANAR:
				if foe.t <= 0.0:
					foe.t = 3.0
					var a6 := rng.randf_range(0.0, TAU)
					var r := rng.randf_range(TILE * 1.5, TILE * 4.0)
					var target2: Vector2 = foe.home + Vector2(sin(a6) * r, cos(a6) * r * 0.7)
					if _free_at(target2, 8.0):
						foe.p = target2
					var base := to_pl.normalized()
					for ang in [-0.42, 0.0, 0.42]:
						var d := base.rotated(ang)
						pending.append({"p": foe.p + d * 8.0 + Vector2(0, -6), "v": d * SHOT_SPEED_FIRE, "fire": true, "life": 3.2})
					if rng.randf() < 0.45:
						spawn_bats.append(foe.p + Vector2(0, -TILE))
						spawn_bats.append(foe.p + Vector2(0, TILE))

		# Déplacement (le recul de la lame domine tant qu'il vit).
		var mv := foe.knock if foe.knock.length() > 4.0 else vel
		if mv.length() > 0.5:
			# Attention : foe.p renvoie une COPIE (Vector2 est une valeur) —
			# il faut réaffecter le vecteur entier au dictionnaire.
			var fp: Vector2 = foe.p
			var nx: float = fp.x + mv.x * dt
			if _free_at(Vector2(nx, fp.y), 6.0):
				fp.x = nx
			elif foe.kind == F.BAT:
				foe.dir = Vector2(-foe.dir.x, foe.dir.y)
			var ny: float = fp.y + mv.y * dt
			if _free_at(Vector2(fp.x, ny), 6.0):
				fp.y = ny
			elif foe.kind == F.BAT:
				foe.dir = Vector2(foe.dir.x, -foe.dir.y)
			foe.p = fp

		if dist < PLAYER_RADIUS + 7.0 and not invuln and phase == "play":
			contacts.append([_foe_touch(foe.kind), (pl_p - foe.p).normalized()])

	for s in pending:
		shots.append(s)
	for c in contacts:
		if pl.invuln <= 0.0 and phase == "play":
			_hurt_player(c[0], c[1])
	for at in spawn_bats:
		var bats := 0
		for f2 in foes:
			if f2.alive and f2.kind == F.BAT:
				bats += 1
		if bats < 4:
			foes.append(_make_foe(F.BAT, at))
			_sparks(at, 8, 0.9, Color(0.7, 0.6, 0.9))


func _free_at(p: Vector2, r: float) -> bool:
	for off in [Vector2(-r, -r), Vector2(r, -r), Vector2(-r, r), Vector2(r, r)]:
		var q := p + off
		if solid(tile(int(floorf(q.x / TILE)), int(floorf(q.y / TILE)))):
			return false
	return true


func _hurt_player(dmg: int, away: Vector2) -> void:
	if phase != "play" or pl.invuln > 0.0:
		return
	inv.hp -= dmg
	pl.invuln = HURT_INVULN
	pl.knock = away * KNOCKBACK
	flash_col = Color(0.7, 0.12, 0.1)
	flash_t = 0.6
	_sparks(pl.p, 10, 1.0, Color(1, 0.4, 0.3))
	cam_shake = 0.7


# ---------------------------------------------------------------- projectiles
func _update_shots(dt: float) -> void:
	for s in shots:
		s.life -= dt
		s.p += s.v * dt
		if s.fire and rng.randf() < dt * 30.0:
			_flame(s.p, 4.0, 0.5)
	var dead: Array = []
	for i in shots.size():
		var s: Dictionary = shots[i]
		var t := tile(int(floorf(s.p.x / TILE)), int(floorf(s.p.y / TILE)))
		if solid(t) or s.life <= 0.0:
			dead.append(i)
		elif s.p.distance_to(pl.p) < PLAYER_RADIUS + 3.5 and pl.invuln <= 0.0 and phase == "play":
			dead.append(i)
			_hurt_player(1, (pl.p - s.p).normalized())
	var keep: Array = []
	for i in shots.size():
		if not dead.has(i):
			keep.append(shots[i])
	shots = keep


# ---------------------------------------------------------------- objets
func _check_pickups() -> void:
	for pk in worlds[cur].pickups:
		if pk.taken:
			continue
		if (pk.p as Vector2).distance_to(pl.p) <= 11.0:
			pk.taken = true
			_sparks(pl.p + Vector2(0, -4), 6, 0.7, Color(1, 1, 0.8))
			match pk.kind:
				PK.HEART:
					inv.hp = mini(inv.hp + 2, inv.hp_max)
					_say("+1 cœur", 1.2, false)
				PK.HEART_CONTAINER:
					_grow_heart()
					_say("UN CONTENEUR DE CŒUR ! Ta vitalité grandit.", 4.0, false)
					_flash(Color(1, 0.47, 0.55))
				PK.GEM:
					inv.gems += 1
				PK.KEY:
					inv.keys += 1
					_say("Une petite clé !", 1.6, false)
				PK.MIRROR:
					inv.mirror = true
					_say("LE MIROIR D'ARGENT ! Les dalles d'argent résonnent…", 5.0, false)
					_flash(Color(0.78, 0.86, 1))
				PK.SWORD:
					inv.sword = true
					_say("L'ÉPÉE D'ARGENT ! Ta lame tranche deux fois plus fort.", 4.0, false)
					_flash(Color(0.86, 0.9, 0.98))


func _check_tiles() -> void:
	var tx := int(floorf(pl.p.x / TILE))
	var ty := int(floorf(pl.p.y / TILE))
	var t := tile(tx, ty)
	if warp_grace <= 0.0 and (t == T.PORTAL or t == T.CAVE):
		var dest = _portal_at(tx, ty)
		if dest != null:
			var dw: int = dest[2]
			var at := Vector2((dest[3] + 0.5) * TILE, (dest[4] + 0.5) * TILE)
			_enter_world(dw, at)
			_say(dest[5], 2.6, false)
			flash_col = Color(0.08, 0.06, 0.12)
			flash_t = 0.4
			return
	if warp_grace <= 0.0 and t == T.MIRROR:
		if inv.mirror:
			var dw2 := AD.DARK_REALM if cur == AD.LIGHT_REALM else AD.LIGHT_REALM
			_enter_world(dw2, pl.p)
			_say("Le miroir te hurls dans l'OMBRE…" if dw2 == AD.DARK_REALM else "Le miroir te rend à la LUMIÈRE.", 3.0, false)
			flash_col = Color(0.75, 0.82, 1)
			flash_t = 0.8
		elif hint_cd <= 0.0:
			hint_cd = 3.0
			_say("Une dalle d'argent froid… quelque chose manque.", 2.4, false)
	if t == T.DOOR or t == T.SEALED:
		var pressing: bool = (pl.p - Vector2((tx + 0.5) * TILE, (ty + 0.5) * TILE)).length() < TILE * 0.75
		if pressing:
			if t == T.DOOR:
				if inv.keys > 0:
					inv.keys -= 1
					set_tile(tx, ty, T.FLOOR)
					_sparks(Vector2((tx + 0.5) * TILE, (ty + 0.5) * TILE), 10, 0.9, Color(0.9, 0.9, 0.6))
					_say("La clé tourne… clic.", 2.0, false)
				elif hint_cd <= 0.0:
					hint_cd = 2.5
					_say("Porte verrouillée — il faut une clé.", 2.0, true)
			else:
				if inv.seals >= SEAL_NEEDED:
					set_tile(tx, ty, T.FLOOR)
					_sparks(Vector2((tx + 0.5) * TILE, (ty + 0.5) * TILE), 18, 1.3, Color(1, 0.84, 0.43))
					cam_shake = 0.8
					_say("Les quatre fragments vibrent… LE SCEAU SE BRISE !", 4.0, false)
					_flash(Color(1, 0.84, 0.43))
				elif seal_cd <= 0.0:
					seal_cd = 3.0
					_say("Le sceau résiste — fragments : %d/%d" % [inv.seals, SEAL_NEEDED], 2.6, true)


func _portal_at(tx: int, ty: int):
	for p in AD.PORTALS[cur]:
		if p[0] == tx and p[1] == ty:
			return p
	return null


func _check_npc() -> void:
	var npc = world().npc
	if npc == null:
		return
	var d: float = (pl.p - (npc.p as Vector2)).length()
	if d < 22.0 and hint_cd <= 0.0:
		match npc.ch:
			"V":
				var hints := [
					"Erfan : Ganar a enlevé la princesse Zahra !",
					"Erfan : Le puits du village descend vers les Geôles.",
					"Erfan : Derrière la cascade dort un passage sombre…",
					"Erfan : Les boss des donjons gardent les 4 fragments du sceau.",
					"Erfan : L'Épée d'argent brille au fond des Mines de Cuivre.",
					"Erfan : Le Miroir d'argent sommeille au Palais de Sable.",
				]
				_say(hints[hint_ix % hints.size()], 4.5, false)
				hint_ix += 1
				hint_cd = 6.0
			"F":
				if inv.hp < inv.hp_max:
					inv.hp = inv.hp_max
					_sparks(pl.p + Vector2(0, -6), 14, 1.1, Color(0.63, 0.94, 0.86))
					_flash(Color(0.63, 0.94, 0.86))
					_say("La Fée du lac murmure… tes blessures se referment.", 4.0, false)
					hint_cd = 4.0
				else:
					var fh := [
						"La Fée : le Palmier Bleu veille sur les voyageurs…",
						"La Fée : la Tour du Vizir perce les nuages au nord de l'Ombre.",
						"La Fée : repose-toi, prince ; le lac guérit qui s'approche.",
					]
					_say(fh[hint_ix % fh.size()], 4.5, false)
					hint_ix += 1
					hint_cd = 6.0
			"Y":
				if ganar_dead:
					_say("Zahra : Mon héros ! …", 3.0, false)
					phase = "victory"
				else:
					_say("Zahra : Le sceau de Ganar me retient ici… brise-le !", 3.5, false)
					hint_cd = 5.0


# ---------------------------------------------------------------- caméra
func _snap_camera() -> void:
	_update_camera(1000.0)


func _update_camera(dt: float) -> void:
	var w := world()
	var z := _zoom()
	var vw := get_viewport_rect().size.x / z
	var vh := get_viewport_rect().size.y / z
	var target := Vector2(
		clampf(pl.p.x - vw * 0.5, 0.0, maxf(w.tw * TILE - vw, 0.0)),
		clampf(pl.p.y - vh * 0.5, 0.0, maxf(w.th * TILE - vh, 0.0)))
	var k := 1.0 - exp(-dt * 7.0)
	cam = cam.lerp(target, k)
	cam_node.position = cam + Vector2(vw, vh) * 0.5
	cam_node.zoom = Vector2(z, z)


func cam_draw() -> Vector2:
	if cam_shake <= 0.001:
		return cam
	return cam + Vector2(sin(time * 47.0) * cam_shake * 3.0, cos(time * 53.0) * cam_shake * 3.0)


func _zoom() -> float:
	return get_viewport_rect().size.y / (VIEWS_TALL[zoom_ix] * TILE)


# ---------------------------------------------------------------- particules
func _sparks(at: Vector2, n: int, life: float, col: Color) -> void:
	for i in n:
		var a := rng.randf_range(0.0, TAU)
		var sp := rng.randf_range(20.0, 90.0)
		parts.append({p = at, v = Vector2(sin(a), cos(a)) * sp, life = life * rng.randf_range(0.5, 1.0),
			max = life, col = col, size = rng.randf_range(1.0, 2.4), grav = 40.0})


func _dust(at: Vector2, n: int, life: float, col: Color) -> void:
	for i in n:
		parts.append({p = at, v = Vector2(rng.randf_range(-14, 14), rng.randf_range(-20, -4)),
			life = life * rng.randf_range(0.6, 1.0), max = life, col = col,
			size = rng.randf_range(1.5, 3.0), grav = -6.0})


func _flame(at: Vector2, size: float, life: float) -> void:
	parts.append({p = at, v = Vector2(rng.randf_range(-8, 8), rng.randf_range(-14, -2)),
		life = life * rng.randf_range(0.5, 1.0), max = life, col = Color(1, 0.63, 0.24),
		size = size * rng.randf_range(0.5, 1.0), grav = -30.0})


func _update_particles(dt: float) -> void:
	var keep: Array = []
	for pt in parts:
		pt.life -= dt
		if pt.life > 0.0:
			pt.v = pt.v + Vector2(0, pt.grav * dt)
			pt.p = pt.p + pt.v * dt
			keep.append(pt)
	parts = keep


# ---------------------------------------------------------------- hash & bruit
func hf(x: int, y: int, salt: int) -> float:
	var n := sin(float(x) * 127.1 + float(y) * 311.7 + float(salt) * 74.7) * 43758.5453
	return fposmod(n, 1.0)


func drift(x: float, y: float, seed_v: float) -> float:
	return clampf(0.5 + 0.25 * sin(x * 0.021 + y * 0.029 + seed_v) \
		+ 0.25 * sin(x * 0.043 - y * 0.037 + seed_v * 1.7), 0.0, 1.0)


func mix(a: Color, b: Color, t: float) -> Color:
	return a.lerp(b, clampf(t, 0.0, 1.0))


# ================================================================ RENDU
func _poly(pts: PackedVector2Array, col: Color) -> void:
	var colors := PackedColorArray()
	colors.resize(pts.size())
	colors.fill(col)
	draw_polygon(pts, colors)


func _circle(c: Vector2, r: float, col: Color, n: int = 14) -> void:
	var pts := PackedVector2Array()
	pts.resize(n)
	for i in n:
		var a := TAU * i / n
		pts[i] = c + Vector2(cos(a), sin(a)) * r
	_poly(pts, col)


func _ellipse(c: Vector2, rx: float, ry: float, col: Color, n: int = 12) -> void:
	var pts := PackedVector2Array()
	pts.resize(n)
	for i in n:
		var a := TAU * i / n
		pts[i] = c + Vector2(cos(a) * rx, sin(a) * ry)
	_poly(pts, col)


func _shadow(c: Vector2, rx: float, ry: float, alpha: float = 1.0) -> void:
	_ellipse(c, rx * 1.25, ry * 1.25, Color(0, 0, 0, 0.14 * alpha))
	_ellipse(c, rx * 0.92, ry * 0.92, Color(0, 0, 0, 0.24 * alpha))


func _draw() -> void:
	var off := cam_draw()
	var z := _zoom()
	var vw := get_viewport_rect().size.x / z
	var vh := get_viewport_rect().size.y / z
	var tx0 := int(floorf(off.x / TILE)) - 1
	var ty0 := int(floorf(off.y / TILE)) - 1
	var tx1 := int(ceilf((off.x + vw) / TILE)) + 1
	var ty1 := int(ceilf((off.y + vh) / TILE)) + 1
	var w := world()
	for ty in range(ty0, ty1 + 1):
		for tx in range(tx0, tx1 + 1):
			if tx < 0 or ty < 0 or tx >= w.tw or ty >= w.th:
				draw_rect(Rect2(tx * TILE, ty * TILE, TILE, TILE), pcol(5).darkened(0.45))
			else:
				_draw_tile(tx, ty)

	# Acteurs : objets au sol, puis tri par y pour le peintre.
	_draw_pickups()
	var order: Array = []
	for foe in foes:
		if foe.alive:
			order.append([foe.p.y, foe])
	if world().npc != null:
		order.append([(world().npc.p as Vector2).y, world().npc])
	order.append([pl.p.y, pl])
	order.sort_custom(func(a, b): return a[0] < b[0])
	for entry in order:
		var e = entry[1]
		if e is Dictionary and e.has("kind") and e.has("alive"):
			_draw_foe(e)
		elif e is Dictionary and e.has("ch"):
			_draw_npc(e)
		else:
			_draw_prince()
	_draw_shots()
	_draw_particles()


# ---------------------------------------------------------------- tuiles
func _draw_tile(tx: int, ty: int) -> void:
	var p := pal()
	var t := tile(tx, ty)
	var x := tx * TILE
	var y := ty * TILE
	var h := hf(tx, ty, 0)
	var cxm := x + TILE * 0.5

	match t:
		T.GRASS, T.FLOWER:
			var d := drift(x, y, 11.0)
			var base := mix(p[0], p[1], d).lerp(p[2], absf(d - 0.5) * 0.5)
			draw_rect(Rect2(x, y, TILE + 0.5, TILE + 0.5), base)
			var tufts := 3 + int(h * 3.0) % 3
			for k in tufts:
				var fx := x + hf(tx, ty, 10 + k) * TILE
				var fy := y + hf(tx, ty, 20 + k) * TILE
				var lean := (hf(tx, ty, 30 + k) - 0.5) * 3.0
				draw_line(Vector2(fx, fy), Vector2(fx + lean, fy - 4.0),
					p[2] if k % 2 == 0 else p[10], 1.1, true)
			if t == T.FLOWER or hf(tx, ty, 7) > 0.93:
				var fx2 := x + 6.0 + hf(tx, ty, 11) * 12.0
				var fy2 := y + 6.0 + hf(tx, ty, 12) * 12.0
				var petal := [Color(0.94, 0.94, 0.9), Color(0.94, 0.75, 0.35), Color(0.87, 0.43, 0.51)][int(h * 3.0) % 3]
				for dd in 4:
					var a := dd * PI * 0.5
					_circle(Vector2(fx2 + cos(a) * 2.0, fy2 + sin(a) * 2.0), 1.4, petal, 6)
				_circle(Vector2(fx2, fy2), 1.2, Color(0.98, 0.84, 0.38), 6)
		T.FLOOR, T.PORTAL, T.CAVE, T.MIRROR:
			var d := drift(x, y, 13.0)
			var base := mix(p[0], p[1], d).lerp(p[2], absf(d - 0.5) * 0.4)
			draw_rect(Rect2(x, y, TILE + 0.5, TILE + 0.5), base)
			for k in 4:
				draw_rect(Rect2(x + hf(tx, ty, 40 + k) * TILE, y + hf(tx, ty, 50 + k) * TILE, 1.6, 1.0), p[2])
			if hf(tx, ty, 60) > 0.75:
				draw_line(Vector2(x + 3, y + TILE * 0.55), Vector2(x + TILE - 3, y + TILE * 0.55), p[2], 0.9, true)
			match t:
				T.PORTAL:
					_draw_portal(x, y)
				T.CAVE:
					_draw_cave(x, y)
				T.MIRROR:
					_draw_mirror_slab(x, y)
		T.BRIDGE:
			draw_rect(Rect2(x, y, TILE + 0.5, TILE + 0.5), Color8(146, 104, 62))
			for k in 4:
				draw_rect(Rect2(x + 3 + k * 6.0, y, 2.2, TILE), Color8(120, 84, 50))
			draw_rect(Rect2(x, y, TILE + 0.5, 2.5), Color8(96, 66, 40))
			draw_rect(Rect2(x, y + TILE - 2.5, TILE + 0.5, 2.5), Color8(96, 66, 40))
		T.WATER:
			draw_rect(Rect2(x, y, TILE + 0.5, TILE + 0.5), p[7])
			for k in 2:
				var phase2 := time * 1.7 + tx * 0.9 + ty * 1.7 + k * 2.1
				var wy := y + 6.0 + k * 10.0 + sin(phase2) * 2.0
				var wx := x + 4.0 + (sin(phase2 * 0.7) * 0.5 + 0.5) * 6.0
				draw_line(Vector2(wx, wy), Vector2(wx + 9, wy), p[8], 1.6, true)
			var tw := sin(time * 1.6 + hf(tx, ty, 8) * TAU)
			if tw > 0.86:
				var g := (tw - 0.86) / 0.14
				_circle(Vector2(x + 5 + hf(tx, ty, 9) * 14, y + 5 + hf(tx, ty, 10) * 14), 1.6 * g, Color(0.94, 0.98, 1), 6)
			var foam := mix(p[8], Color(0.92, 0.96, 0.99), 0.5)
			if not solid(tile(tx, ty - 1)):
				draw_rect(Rect2(x, y, TILE + 0.5, 2.0), foam)
			if not solid(tile(tx, ty + 1)):
				draw_rect(Rect2(x, y + TILE - 2.0, TILE + 0.5, 2.0), foam)
			if not solid(tile(tx - 1, ty)):
				draw_rect(Rect2(x, y, 2.0, TILE + 0.5), foam)
			if not solid(tile(tx + 1, ty)):
				draw_rect(Rect2(x + TILE - 2.0, y, 2.0, TILE + 0.5), foam)
		T.PIT:
			draw_rect(Rect2(x, y, TILE + 0.5, TILE + 0.5), p[13])
			if not solid(tile(tx, ty - 1)):
				draw_rect(Rect2(x, y, TILE + 0.5, 3.0), p[5])
			if hf(tx, ty, 3) > 0.6:
				draw_rect(Rect2(x + 8, y + 8, 2.0, 10.0), p[13].darkened(0.5))
		T.WALL:
			_draw_wall(tx, ty)
		T.TREE:
			draw_rect(Rect2(x, y, TILE + 0.5, TILE + 0.5), p[1])
			_shadow(Vector2(cxm, y + TILE * 0.78), 8.5, 3.2)
			draw_rect(Rect2(cxm - 2.5, y + TILE * 0.45, 5.0, TILE * 0.5), p[12])
			_circle(Vector2(cxm, y + TILE * 0.42), 11.0, p[9], 12)
			_circle(Vector2(cxm - 4, y + TILE * 0.42 - 3.0), 6.5, p[10], 10)
			_circle(Vector2(cxm + 4.5, y + TILE * 0.42 - 1.0), 5.5, p[10], 10)
			_circle(Vector2(cxm - 1 + (hf(tx, ty, 2) - 0.5) * 4, y + TILE * 0.42 - 6.0), 4.0, p[11], 8)
			_circle(Vector2(cxm + 3, y + TILE * 0.42 - 5.5 + (hf(tx, ty, 3) - 0.5) * 3), 2.4, p[11], 6)
		T.BUSH:
			draw_rect(Rect2(x, y, TILE + 0.5, TILE + 0.5), p[1])
			_shadow(Vector2(cxm, y + TILE * 0.72), 7.5, 2.6)
			var cym := y + TILE * 0.52
			_circle(Vector2(cxm, cym), 8.0, p[9], 10)
			_circle(Vector2(cxm - 2.5, cym - 2.0), 5.0, p[10], 8)
			_circle(Vector2(cxm + 2.5, cym - 1.5), 4.2, p[10], 8)
			_circle(Vector2(cxm - 1, cym - 3.5), 2.2, p[11], 6)
			if hf(tx, ty, 9) > 0.8:
				_circle(Vector2(cxm + 3, cym + 2.0), 1.4, Color(0.82, 0.24, 0.31), 6)
		T.STATUE:
			draw_rect(Rect2(x, y, TILE + 0.5, TILE + 0.5), p[1])
			_shadow(Vector2(cxm, y + TILE * 0.8), 8.0, 2.8)
			draw_rect(Rect2(cxm - 7, y + TILE - 7, 14, 5), p[5])
			draw_rect(Rect2(cxm - 6, y + TILE - 11, 12, 5), p[4])
			_poly(PackedVector2Array([
				Vector2(cxm - 5, y + TILE - 11), Vector2(cxm + 5, y + TILE - 11),
				Vector2(cxm + 3, y + 5), Vector2(cxm - 3, y + 5)]), p[3])
			_circle(Vector2(cxm, y + 5.5), 3.4, Color(0.78, 0.75, 0.69), 8)
			draw_rect(Rect2(cxm - 2, y + 5, 4, 1.4), p[5])
		T.BRAZIER:
			draw_rect(Rect2(x, y, TILE + 0.5, TILE + 0.5), p[1])
			_shadow(Vector2(cxm, y + TILE * 0.75), 6.0, 2.2)
			draw_rect(Rect2(cxm - 1.6, y + 10, 3.2, 8), Color8(96, 66, 40))
			_circle(Vector2(cxm, y + 10), 5.2, Color8(120, 84, 50), 8)
			_circle(Vector2(cxm, y + 9.4), 4.2, Color8(70, 50, 34), 8)
			var flick := 1.0 + sin(time * 9.0 + tx * 7 + ty * 13) * 0.3
			var fh := 8.0 * flick
			_poly(PackedVector2Array([
				Vector2(cxm - 3.2, y + 8), Vector2(cxm + 3.2, y + 8),
				Vector2(cxm + 1, y + 8 - fh), Vector2(cxm, y + 8 - fh - 2.5),
				Vector2(cxm - 1, y + 8 - fh)]), Color8(255, 168, 66))
			_poly(PackedVector2Array([
				Vector2(cxm - 1.6, y + 8), Vector2(cxm + 1.6, y + 8),
				Vector2(cxm, y + 8 - fh * 0.62)]), Color8(255, 232, 150))
		T.DOOR:
			_draw_door(x, y, false)
		T.SEALED:
			_draw_door(x, y, true)


func _draw_wall(tx: int, ty: int) -> void:
	var p := pal()
	var x := tx * TILE
	var y := ty * TILE
	var h := hf(tx, ty, 0)
	var open := func(t: int) -> bool:
		return t == T.FLOOR or t == T.GRASS or t == T.FLOWER or t == T.BRIDGE \
			or t == T.WATER or t == T.PIT or t == T.MIRROR or t == T.PORTAL or t == T.CAVE
	draw_rect(Rect2(x, y, TILE + 0.5, TILE + 0.5), p[4])
	for row in 3:
		var by := y + row * 8.0
		draw_rect(Rect2(x, by, TILE + 0.5, 0.9), p[6])
		var off := 0.0 if fmod(row + h, 2.0) == 0.0 else 12.0
		for k in 2:
			var bx := x + off + k * 24.0
			if bx > x + 1.0 and bx < x + TILE:
				draw_rect(Rect2(bx, by, 0.9, 8.0), p[6])
		if fmod(h + row, 3.0) == 0.0:
			draw_rect(Rect2(x + 3 + fmod(off, 12.0), by + 1.6, 8, 1.6), mix(p[3], p[4], 0.4))
	if not solid(tile(tx, ty - 1)):
		draw_rect(Rect2(x, y, TILE + 0.5, 4.5), p[3])
		draw_rect(Rect2(x, y + 4.5, TILE + 0.5, 1.4), p[5])
	if open.call(tile(tx, ty + 1)):
		draw_rect(Rect2(x, y + TILE, TILE + 0.5, 3.2), Color(0, 0, 0, 0.16))
	if not solid(tile(tx - 1, ty)):
		draw_rect(Rect2(x, y + 4.5, 1.8, TILE - 4.0), p[5])
	if not solid(tile(tx + 1, ty)):
		draw_rect(Rect2(x + TILE - 1.8, y + 4.5, 1.8, TILE - 4.0), p[5])


func _draw_portal(x: float, y: float) -> void:
	var p := pal()
	var c := Vector2(x + TILE * 0.5, y + TILE * 0.5)
	_circle(c, 10.5, p[4], 14)
	_circle(c, 8.5, p[13], 12)
	var cols := [Color(0.59, 0.47, 0.86), Color(0.38, 0.63, 0.86), Color(0.86, 0.78, 1)]
	for k in 3:
		var r := 7.5 - k * 2.2
		var spin := time * (1.4 + k * 0.5) + k * 2.1
		var pts := PackedVector2Array()
		for i in 11:
			var a0 := spin + i / 10.0 * TAU
			var wob := 1.0 + sin(a0 * 2.0 + time * 3.0) * 0.12
			pts.append(c + Vector2(cos(a0) * r * wob, sin(a0) * r * wob))
		draw_polyline(pts + PackedVector2Array([pts[0]]), cols[k], 1.8, true)
	_circle(c, 2.2 + absf(sin(time * 3.0)) * 0.8, Color(0.92, 0.88, 1), 8)


func _draw_cave(x: float, y: float) -> void:
	var p := pal()
	var cxm := x + TILE * 0.5
	var base := y + TILE - 2.0
	_circle(Vector2(cxm, base), 11.0, p[5], 12)
	draw_rect(Rect2(x, base, TILE + 0.5, 3.0), p[0])
	_circle(Vector2(cxm, base + 0.5), 8.6, Color(0.03, 0.02, 0.04), 12)
	_circle(Vector2(cxm, base + 2.5), 5.0, Color(0.01, 0.01, 0.02), 10)
	draw_line(Vector2(cxm - 8, base - 3), Vector2(cxm + 8, base - 3), p[3], 2.0, true)


func _draw_mirror_slab(x: float, y: float) -> void:
	var pad := 2.5
	var active: bool = inv.mirror
	var glow := 0.5 + 0.5 * sin(time * 2.2) if active else 0.0
	var frame := Color(0.58, 0.6, 0.66)
	var face := Color(0.77, 0.82, 0.88).lerp(Color(0.59, 0.75, 0.94), glow * 0.5)
	draw_rect(Rect2(x + pad, y + pad, TILE - pad * 2, TILE - pad * 2), frame)
	draw_rect(Rect2(x + pad + 1.6, y + pad + 1.6, TILE - pad * 2 - 3.2, TILE - pad * 2 - 3.2), face)
	_poly(PackedVector2Array([
		Vector2(x + pad + 2, y + TILE - pad - 4), Vector2(x + TILE - pad - 6, y + pad + 2),
		Vector2(x + TILE - pad - 2, y + pad + 2), Vector2(x + pad + 6, y + TILE - pad - 4)]),
		Color(0.92, 0.95, 0.98))
	var sp := sin(time * 1.3 + x * 0.13 + y * 0.17)
	if sp > 0.55:
		var sx := x + TILE * (0.3 + 0.4 * fposmod(sp, 1.0))
		var sy := y + TILE * 0.4
		draw_line(Vector2(sx - 2.5, sy), Vector2(sx + 2.5, sy), Color.WHITE, 1.0, true)
		draw_line(Vector2(sx, sy - 2.5), Vector2(sx, sy + 2.5), Color.WHITE, 1.0, true)


func _draw_door(x: float, y: float, sealed: bool) -> void:
	var pad := 1.5
	if sealed:
		draw_rect(Rect2(x + pad, y + pad, TILE - pad * 2, TILE - pad * 2), Color8(158, 120, 44))
		draw_rect(Rect2(x + pad + 2, y + pad + 2, TILE - pad * 2 - 4, TILE - pad * 2 - 4), Color8(196, 152, 60))
		var c := Vector2(x + TILE * 0.5, y + TILE * 0.5)
		_circle(c, 6.0, Color8(120, 88, 30), 10)
		_circle(c, 4.6, Color8(232, 196, 96), 10)
		_poly(PackedVector2Array([c + Vector2(0, -3.2), c + Vector2(3, 2.2), c + Vector2(-3, 2.2)]), Color8(120, 88, 30))
	else:
		draw_rect(Rect2(x + pad, y + pad, TILE - pad * 2, TILE - pad * 2), Color8(104, 70, 40))
		draw_rect(Rect2(x + pad + 1.4, y + pad + 1.4, TILE * 0.5 - pad - 2.2, TILE - pad * 2 - 2.8), Color8(134, 94, 56))
		draw_rect(Rect2(x + TILE * 0.5 + 0.8, y + pad + 1.4, TILE * 0.5 - pad - 2.2, TILE - pad * 2 - 2.8), Color8(134, 94, 56))
		for by in [y + 6.0, y + TILE - 8.0]:
			draw_rect(Rect2(x + pad, by, TILE - pad * 2, 2.2), Color8(72, 74, 84))
		draw_rect(Rect2(x + TILE * 0.5 - 1.6, y + TILE * 0.5 - 2.4, 3.2, 5.0), Color8(72, 74, 84))
		draw_rect(Rect2(x + TILE * 0.5 - 0.7, y + TILE * 0.5 - 1.2, 1.4, 2.6), Color8(24, 24, 30))


# ---------------------------------------------------------------- prince
const SKIN := Color(0.886, 0.698, 0.518)
const TUNIC := Color(0.345, 0.549, 0.361)
const TUNIC_DARK := Color(0.251, 0.424, 0.282)
const HEADBAND := Color(0.776, 0.235, 0.212)
const BOOT := Color(0.329, 0.227, 0.157)
const STEEL := Color(0.831, 0.855, 0.894)
const SILVER := Color(0.941, 0.965, 1.0)


func _draw_prince() -> void:
	var bob := sin(pl.anim * 2.0) * 0.8 if pl.moving else sin(time * 2.2) * 0.4
	var step := sin(pl.anim) if pl.moving else 0.0
	var blink: bool = pl.invuln > 0.0 and int(pl.invuln * 14.0) % 2 == 0
	var p: Vector2 = pl.p
	_shadow(p + Vector2(0, 7), 7.0, 2.6)
	if blink:
		return
	var fv := _dir_vec(pl.facing)
	var f1 := p + Vector2(-fv.y * 3.0 + fv.x * step * 3.0, 5.0 + absf(fv.y) * step * 2.0)
	var f2 := p + Vector2(fv.y * 3.0 - fv.x * step * 3.0, 5.0 - absf(fv.y) * step * 2.0)
	_circle(f1, 2.2, BOOT, 6)
	_circle(f2, 2.2, BOOT, 6)
	var body_c := p + Vector2(0, bob * 0.5)
	_circle(body_c + Vector2(0, 1.5), 6.2, TUNIC_DARK, 10)
	_circle(body_c, 6.0, TUNIC, 10)
	draw_rect(Rect2(body_c.x - 5.6, body_c.y + 1.2, 11.2, 2.0), Color(0.47, 0.33, 0.19))
	_circle(body_c + Vector2(0, 2.2), 1.4, Color(0.886, 0.729, 0.353), 6)
	var arm := Vector2(-fv.y, fv.x)
	_circle(body_c + arm * -5.4 + Vector2(0, -1), 2.4, TUNIC_DARK, 6)
	_circle(body_c + arm * 5.4 + Vector2(0, -1), 2.4, TUNIC_DARK, 6)
	var head := p + Vector2(0, -6 + bob)
	_circle(head, 4.6, Color(0.235, 0.173, 0.125), 10)
	_circle(head - fv * 0.6, 4.0, SKIN, 10)
	draw_rect(Rect2(head.x - 4.4, head.y - 1.6, 8.8, 2.4), HEADBAND)
	if pl.facing != Dir.UP:
		var ex := head.x + fv.x * 1.6
		var ey := head.y + 1.2 + fv.y * 1.0
		var dx := 1.8 if fv.x == 0.0 else 0.0
		draw_rect(Rect2(ex - dx - 0.7, ey, 1.4, 1.6), Color(0.16, 0.12, 0.1))
		draw_rect(Rect2(ex + dx - 0.7, ey, 1.4, 1.6), Color(0.16, 0.12, 0.1))
	else:
		_circle(head + Vector2(0, -1.5), 2.6, Color(0.29, 0.21, 0.16), 8)
	var blade := SILVER if inv.sword else STEEL
	var atk: float = pl.attack_t / ATTACK_TIME
	if atk > 0.0:
		var sweep := (atk - 0.5) * 2.0
		var dir := fv.rotated(sweep * 1.15)
		draw_line(body_c + dir * 3.0 + Vector2(0, -2), body_c + dir * 15.0 + Vector2(0, -2), blade, 2.6, true)
		draw_line(body_c + dir * 2.0, body_c + dir * 4.4, Color(0.59, 0.43, 0.24), 3.4, true)
		var td := fv.rotated(sweep * 1.15 + 0.55)
		draw_line(body_c + td * 13.0 + Vector2(0, -2), body_c + dir * 13.0 + Vector2(0, -2), Color(1, 1, 1, atk * 0.5), 3.0, true)
	else:
		var rest := Vector2(fv.x * 0.4 - fv.y * 0.9, fv.y * 0.4 + fv.x * 0.9)
		draw_line(body_c - rest * 2.0 + Vector2(0, -4), body_c + rest * 9.0 + Vector2(0, -10), blade, 2.2, true)
	var sh_at := body_c + Vector2(fv.y, -fv.x) * 6.5
	_circle(sh_at, 3.4, Color(0.47, 0.33, 0.19), 8)
	_circle(sh_at, 2.2, Color(0.745, 0.588, 0.353), 8)


# ---------------------------------------------------------------- ennemis
func _draw_foe(foe: Dictionary) -> void:
	var p: Vector2 = foe.p
	var flash: bool = foe.hurt_t > 0.0
	var face: Vector2 = foe.dir.normalized() if foe.dir.length() > 0.1 else Vector2(0, 1)
	var W_ := func(c: Color) -> Color:
		return c.lerp(Color(1, 1, 1), 0.65) if flash else c
	match foe.kind:
		F.CRAB:
			_shadow(p + Vector2(0, 5), 6.5, 2.2)
			var wig := sin(foe.anim * 2.0)
			for s in [-1.0, 1.0]:
				for k in 2:
					var lx := p.x + s * (5.0 + k * 2.5)
					var ly := p.y + 2.0 + k * 3.0 + wig * s * 1.2
					draw_line(p + Vector2(s * 3, 0), Vector2(lx, ly + 2), W_.call(Color(0.55, 0.24, 0.16)), 1.6, true)
			var shell: Color = W_.call(Color(0.8, 0.33, 0.23))
			_circle(p, 6.4, shell, 10)
			_circle(p + Vector2(-2, -2), 3.4, W_.call(Color(0.91, 0.5, 0.35)), 8)
			var sn := p + face * 5.5
			_circle(sn, 2.6, W_.call(Color(0.91, 0.5, 0.35)), 8)
			_circle(sn + face * 1.2, 1.2, Color(0.16, 0.09, 0.08), 6)
			for s in [-1.0, 1.0]:
				var e := p + face * 2.0 + face.orthogonal() * s * 2.6 + Vector2(0, -4.5)
				draw_line(p + Vector2(0, -3), e, shell, 1.2, true)
				_circle(e, 1.1, Color(0.98, 0.94, 0.86), 5)
		F.GRUNT, F.OGRE:
			var big: bool = foe.kind == F.OGRE
			var sc := 1.5 if big else 1.0
			_shadow(p + Vector2(0, 6 * sc), 7.0 * sc, 2.6 * sc)
			var hide: Color = W_.call(Color(0.478, 0.329, 0.51) if big else Color(0.408, 0.463, 0.29))
			var hide_hi: Color = W_.call(Color(0.62, 0.455, 0.65) if big else Color(0.525, 0.58, 0.384))
			var bc := p + Vector2(0, -1.0 * sc)
			_circle(p + Vector2(-3.5 * sc, 4.5), 2.2 * sc, W_.call(Color(0.275, 0.204, 0.157)), 6)
			_circle(p + Vector2(3.5 * sc, 4.5), 2.2 * sc, W_.call(Color(0.275, 0.204, 0.157)), 6)
			_circle(bc, 6.6 * sc, hide, 10)
			_circle(bc + Vector2(-2 * sc, -2 * sc), 3.4 * sc, hide_hi, 8)
			var sn2 := bc + face * 5.0 * sc
			_circle(sn2, 3.2 * sc, hide_hi, 8)
			_circle(sn2 + face * 1.6 * sc, 1.4 * sc, Color(0.196, 0.133, 0.118), 6)
			for s in [-1.0, 1.0]:
				_circle(sn2 + face * 2.6 * sc + face.orthogonal() * s * 2.0 * sc, 0.9 * sc, Color(0.94, 0.925, 0.86), 5)
			var eb := bc + face * 2.4 * sc + Vector2(0, -3 * sc)
			for s in [-1.0, 1.0]:
				_circle(eb + face.orthogonal() * s * 2.2 * sc, 1.1 * sc, Color(0.86, 0.2, 0.16), 5)
			for s in [-1.0, 1.0]:
				var h := bc + face.orthogonal() * s * 5.4 * sc + Vector2(0, -4 * sc)
				draw_line(h, h + Vector2(0, -3 * sc) + face.orthogonal() * s * 1.5 * sc, Color(0.886, 0.863, 0.784), 1.6 * sc, true)
			var club := bc + face * 7.5 * sc + Vector2(0, 2)
			draw_line(bc + face * 3.0 * sc, club, Color(0.376, 0.267, 0.173), 2.4 * sc, true)
			_circle(club, 2.6 * sc, Color(0.455, 0.329, 0.212), 7)
			if big:
				draw_line(bc + Vector2(-6 * sc, 3), bc + Vector2(-9 * sc, 6), Color(0.59, 0.59, 0.63), 1.4, true)
				draw_line(bc + Vector2(6 * sc, 3), bc + Vector2(9 * sc, 6), Color(0.59, 0.59, 0.63), 1.4, true)
		F.BAT:
			var flap := sin(foe.anim)
			var hover := sin(foe.anim * 0.7) * 2.0
			var bp := p + Vector2(0, -6 + hover)
			_shadow(p + Vector2(0, 6), 4.5, 1.8, 0.7)
			for s in [-1.0, 1.0]:
				var root := bp + Vector2(s * 2.5, 0)
				var tip := root + Vector2(s * 7.0, -3.0 - flap * 4.0)
				var mid := root + Vector2(s * 4.5, 1.5 - flap * 2.0)
				_poly(PackedVector2Array([root, tip, mid]), W_.call(Color(0.275, 0.227, 0.345) if s < 0 else Color(0.408, 0.345, 0.502)))
			_circle(bp, 3.4, W_.call(Color(0.22, 0.18, 0.28)), 8)
			_circle(bp + Vector2(0, -2.6), 1.4, W_.call(Color(0.275, 0.227, 0.345)), 5)
			for s in [-1.0, 1.0]:
				_circle(bp + face * 1.6 + face.orthogonal() * s * 1.4, 0.9, W_.call(Color(1, 0.35, 0.27)), 5)
		F.DJINN:
			var rise := sin(time * 2.0 + p.x * 0.05) * 1.5
			var dp := p + Vector2(0, -4 + rise)
			var rr := 7.0 + fposmod(time * 2.4, 1.0) * 5.0
			var ra := 0.5 - fposmod(time * 2.4, 1.0) * 0.4
			var rpts := PackedVector2Array()
			for i in 14:
				var a := TAU * i / 14
				rpts.append(dp + Vector2(cos(a), sin(a)) * rr)
			draw_polyline(rpts + PackedVector2Array([rpts[0]]), Color(0.86, 0.94, 0.98, ra), 1.2, true)
			var fin: Color = W_.call(Color(0.227, 0.588, 0.627))
			var fin_hi: Color = W_.call(Color(0.376, 0.769, 0.784))
			for k in range(-2, 3):
				var a2 := k * 0.5
				var dv := Vector2(sin(a2), -cos(a2) * 0.6 - 0.55).normalized()
				draw_line(dp, dp + dv * 8.5, fin if k % 2 == 0 else fin_hi, 2.0, true)
			_circle(dp, 5.0, fin, 9)
			_circle(dp + Vector2(-1.5, -1.5), 2.6, fin_hi, 7)
			_circle(dp + face * 4.0 + Vector2(0, 1), 1.8, Color(0.08, 0.16, 0.18), 6)
			for s in [-1.0, 1.0]:
				_circle(dp + face * 1.5 + face.orthogonal() * s * 2.2 + Vector2(0, -2), 1.0, Color(0.98, 0.86, 0.35), 5)
		F.KNIGHT, F.BOSS:
			var boss: bool = foe.kind == F.BOSS
			var sc2 := 1.35 if boss else 1.0
			_shadow(p + Vector2(0, 6.5 * sc2), 7.2 * sc2, 2.6 * sc2)
			var armour: Color = W_.call(Color(0.376, 0.329, 0.431) if boss else Color(0.329, 0.345, 0.408))
			var armour_hi: Color = W_.call(Color(0.549, 0.494, 0.627) if boss else Color(0.494, 0.518, 0.596))
			var trim: Color = W_.call(Color(0.831, 0.659, 0.275) if boss else Color(0.47, 0.502, 0.557))
			for s in [-1.0, 1.0]:
				_circle(p + Vector2(s * 3.6 * sc2, 5.0), 2.3 * sc2, armour, 6)
			var bc2 := p + Vector2(0, -1)
			_circle(bc2, 6.4 * sc2, armour, 10)
			_circle(bc2 + Vector2(-1.8 * sc2, -2 * sc2), 3.0 * sc2, armour_hi, 8)
			var hd := bc2 + Vector2(0, -5.5 * sc2)
			_circle(hd, 4.4 * sc2, armour_hi, 9)
			draw_rect(Rect2(hd.x - 3.6 * sc2, hd.y - 0.9 * sc2, 7.2 * sc2, 1.8 * sc2), Color(0.06, 0.055, 0.08))
			for s in [-1.0, 1.0]:
				_circle(hd + face * 1.2 * sc2 + face.orthogonal() * s * 1.9 * sc2, 0.8 * sc2,
					W_.call(Color(1, 0.67, 0.24) if boss else Color(1, 0.27, 0.24)), 5)
			if boss:
				draw_line(hd + Vector2(0, -4.2 * sc2), hd + Vector2(-face.x * 3.0, -8.5 * sc2), W_.call(Color(0.78, 0.2, 0.24)), 2.2, true)
			var sh := bc2 + face * 6.8 * sc2 + Vector2(0, 0.5)
			draw_rect(Rect2(sh.x - 3.0 * sc2, sh.y - 5.0 * sc2, 6.0 * sc2, 10.0 * sc2), armour)
			draw_rect(Rect2(sh.x - 2.2 * sc2, sh.y - 4.2 * sc2, 4.4 * sc2, 8.4 * sc2), armour_hi)
			draw_rect(Rect2(sh.x - 0.8 * sc2, sh.y - 2.0 * sc2, 1.6 * sc2, 4.0 * sc2), trim)
		F.GOLEM:
			var sc3 := 1.45
			_shadow(p + Vector2(0, 6.5 * sc3), 8.0 * sc3, 2.8 * sc3)
			var core: Color = W_.call(Color(0.47, 0.745, 0.9))
			var core_hi: Color = W_.call(Color(0.706, 0.9, 1))
			var core_dark: Color = W_.call(Color(0.275, 0.47, 0.667))
			for s in [-1.0, 1.0]:
				_circle(p + Vector2(s * 4.5 * sc3, 5.0), 2.6 * sc3, core_dark, 6)
			var bc3 := p + Vector2(0, -1.5 * sc3)
			_poly(PackedVector2Array([bc3 + Vector2(-7 * sc3, -4 * sc3), bc3 + Vector2(0, -8 * sc3),
				bc3 + Vector2(7 * sc3, -4 * sc3), bc3 + Vector2(6 * sc3, 5 * sc3),
				bc3 + Vector2(0, 7 * sc3), bc3 + Vector2(-6 * sc3, 5 * sc3)]), core)
			_poly(PackedVector2Array([bc3 + Vector2(-7 * sc3, -4 * sc3), bc3 + Vector2(0, -8 * sc3), bc3]), core_hi)
			_poly(PackedVector2Array([bc3, bc3 + Vector2(6 * sc3, 5 * sc3), bc3 + Vector2(0, 7 * sc3)]), core_dark)
			var hd3 := bc3 + Vector2(0, -9 * sc3 + sin(foe.anim * 1.4) * 0.5)
			_poly(PackedVector2Array([hd3 + Vector2(-3.4 * sc3, 1), hd3 + Vector2(0, -4.6 * sc3),
				hd3 + Vector2(3.4 * sc3, 1), hd3 + Vector2(0, 2.6 * sc3)]), core)
			var pulse := 0.65 + 0.35 * sin(time * 4.0)
			for s in [-1.0, 1.0]:
				_circle(hd3 + face + face.orthogonal() * s * 1.7 * sc3, 0.9 * sc3,
					Color(1, 0.98, 0.78).lerp(Color(0.63, 0.94, 1), pulse), 5)
			for s in [-1.0, 1.0]:
				var sh3 := bc3 + Vector2(s * 7.5 * sc3, -3 * sc3)
				draw_line(sh3, sh3 + Vector2(s * 1.6, -5), core_hi, 2.0, true)
				draw_line(sh3 + Vector2(s * 2, 1), sh3 + Vector2(s * 3.6, -2.6), core, 1.6, true)
		F.PHARAOH:
			var sc4 := 1.4
			_shadow(p + Vector2(0, 6.5 * sc4), 7.4 * sc4, 2.6 * sc4, 0.9)
			var wrap: Color = W_.call(Color(0.839, 0.792, 0.675))
			var wrap_sh: Color = W_.call(Color(0.682, 0.635, 0.525))
			var gold: Color = W_.call(Color(0.886, 0.729, 0.353))
			var stp := sin(foe.anim)
			for s in [-1.0, 1.0]:
				_circle(p + Vector2(s * 3.4 * sc4, 5.0 + s * stp * 1.4), 2.2 * sc4, wrap_sh, 6)
			var bc4 := p + Vector2(0, -1.0 * sc4)
			_poly(PackedVector2Array([bc4 + Vector2(-5.6 * sc4, -4 * sc4), bc4 + Vector2(5.6 * sc4, -4 * sc4),
				bc4 + Vector2(4 * sc4, 6 * sc4), bc4 + Vector2(-4 * sc4, 6 * sc4)]), wrap)
			for k in 3:
				var ly := bc4.y - 2.0 * sc4 + k * 3.4
				draw_line(Vector2(bc4.x - 4.6 * sc4, ly),
					Vector2(bc4.x + (4.6 if k % 2 == 0 else 3.4) * sc4, ly + 1.6), wrap_sh, 1.2, true)
			draw_rect(Rect2(bc4.x - 4.6 * sc4, bc4.y - 4.6 * sc4, 9.2 * sc4, 2.0), gold)
			var hd4 := bc4 + Vector2(face.x * 0.6, -8.0 * sc4)
			_circle(hd4, 4.2 * sc4, wrap, 9)
			_circle(hd4 + Vector2(0, -2.6 * sc4), 3.4 * sc4, gold, 8)
			for s in [-1.0, 1.0]:
				draw_line(hd4 + Vector2(s * 4.0 * sc4, -1), hd4 + Vector2(s * 5.2 * sc4, 3.4 * sc4), gold, 1.8, true)
			for s in [-1.0, 1.0]:
				_circle(hd4 + face * 1.2 * sc4 + face.orthogonal() * s * 1.7 * sc4, 0.95 * sc4, W_.call(Color(0.549, 1, 0.667)), 5)
			draw_line(bc4 + Vector2(-4 * sc4, 4 * sc4), bc4 + Vector2(3.2 * sc4, -1.6 * sc4), gold, 1.5, true)
			draw_line(bc4 + Vector2(4 * sc4, 4 * sc4), bc4 + Vector2(-3.2 * sc4, -1.6 * sc4), gold, 1.5, true)
		F.GANAR:
			var fl := sin(time * 2.0) * 1.6
			var gp := p + Vector2(0, -5 + fl)
			_shadow(p + Vector2(0, 7), 7.5, 2.6, 0.9)
			var sway := sin(time * 3.1) * 2.0
			_poly(PackedVector2Array([gp + Vector2(-5, -4), gp + Vector2(5, -4),
				gp + Vector2(8 + sway, 8), gp + Vector2(-8 - sway, 8)]), W_.call(Color(0.227, 0.102, 0.259)))
			_poly(PackedVector2Array([gp + Vector2(-5.5, -4), gp + Vector2(5.5, -4),
				gp + Vector2(3.5, 8.5), gp + Vector2(-3.5, 8.5)]), W_.call(Color(0.157, 0.078, 0.188)))
			draw_line(gp + Vector2(-4.6, -2), gp + Vector2(-3, 7.5), W_.call(Color(0.831, 0.659, 0.275)), 1.2, true)
			draw_line(gp + Vector2(4.6, -2), gp + Vector2(3, 7.5), W_.call(Color(0.831, 0.659, 0.275)), 1.2, true)
			var hd5 := gp + Vector2(0, -7.5)
			_circle(hd5, 4.2, W_.call(Color(0.376, 0.549, 0.329)), 9)
			draw_line(hd5 + Vector2(-3.4, -2.4), hd5 + Vector2(-6.5, -10), W_.call(Color(0.118, 0.07, 0.141)), 2.0, true)
			draw_line(hd5 + Vector2(3.4, -2.4), hd5 + Vector2(6.5, -10), W_.call(Color(0.118, 0.07, 0.141)), 2.0, true)
			_circle(hd5 + Vector2(-6.5, -10), 1.4, W_.call(Color(0.831, 0.659, 0.275)), 5)
			_circle(hd5 + Vector2(6.5, -10), 1.4, W_.call(Color(0.831, 0.659, 0.275)), 5)
			var pulse5 := 0.7 + 0.3 * sin(time * 5.0)
			var eg := Color(1, 0.78, 0.24).lerp(Color(1, 0.94, 0.55), pulse5)
			for s in [-1.0, 1.0]:
				_circle(hd5 + face * 1.4 + face.orthogonal() * s * 1.8, 1.1, eg, 5)
			var st := gp + face.orthogonal() * 7.0
			draw_line(st + Vector2(0, -8), st + Vector2(0, 8), Color(0.275, 0.173, 0.118), 1.8, true)
			var orb := st + Vector2(0, -10)
			var orb_r := 2.6 + absf(sin(time * 4.0)) * 0.7
			_circle(orb, orb_r, Color(0.59, 0.24, 0.86), 8)
			_circle(orb, orb_r * 0.5, Color(0.9, 0.7, 1), 6)


# ---------------------------------------------------------------- PNJ
func _draw_npc(npc: Dictionary) -> void:
	var p: Vector2 = npc.p
	var bob := sin(time * 2.4) * 0.6
	_shadow(p + Vector2(0, 7), 6.5, 2.4)
	match npc.ch:
		"V":
			var robe := Color(0.478, 0.361, 0.243)
			var robe_hi := Color(0.58, 0.447, 0.314)
			_poly(PackedVector2Array([p + Vector2(-5.5, -4 + bob), p + Vector2(5.5, -4 + bob),
				p + Vector2(4, 7), p + Vector2(-4, 7)]), robe)
			_poly(PackedVector2Array([p + Vector2(-5.5, -4 + bob), p + Vector2(-1, -4 + bob),
				p + Vector2(-1, 6), p + Vector2(-4, 6.5)]), robe_hi)
			var hd := p + Vector2(0, -6.5 + bob)
			_circle(hd, 3.6, SKIN, 8)
			_poly(PackedVector2Array([hd + Vector2(-2.8, 1), hd + Vector2(2.8, 1), hd + Vector2(0, 8)]),
				Color(0.91, 0.894, 0.855))
			_circle(hd + Vector2(0, -2.2), 3.2, robe_hi, 8)
			draw_line(p + Vector2(6.5, -9), p + Vector2(6.5, 7), Color(0.376, 0.267, 0.173), 1.6, true)
			_circle(p + Vector2(6.5, -9.5), 1.8, Color(0.886, 0.729, 0.353), 6)
		"F":
			var hover := sin(time * 2.6) * 3.0
			var fp := p + Vector2(0, -8 + hover)
			var flap := sin(time * 14.0)
			for s in [-1.0, 1.0]:
				var root := fp + Vector2(s * 2.0, 0)
				_poly(PackedVector2Array([root, root + Vector2(s * 6.5, -2.5 - flap * 2.5),
					root + Vector2(s * 4.0, 2.0 + flap * 1.5)]), Color(0.784, 0.941, 0.98))
			_circle(fp + Vector2(0, 2.4), 2.0, Color(0.47, 0.784, 0.824), 7)
			_circle(fp, 2.8, SKIN, 8)
			_circle(fp + Vector2(0, -2.6), 2.4, Color(0.863, 0.627, 0.353), 7)
			for k in 4:
				var a := time * 1.8 + k * 1.57
				_circle(fp + Vector2(cos(a) * 8.0, sin(a) * 5.0), 0.9 + absf(sin(a) * 0.5), Color(0.745, 1, 0.9), 5)
		"Y":
			var freed := ganar_dead
			var dress := Color(0.933, 0.839, 0.886) if freed else Color(0.808, 0.714, 0.816)
			_poly(PackedVector2Array([p + Vector2(-4.5, -4 + bob), p + Vector2(4.5, -4 + bob),
				p + Vector2(6.5, 7), p + Vector2(-6.5, 7)]), dress)
			draw_rect(Rect2(p.x - 4.6, p.y + 0.5, 9.2, 1.6), Color(0.776, 0.58, 0.659))
			var hd2 := p + Vector2(0, -6.5 + bob)
			_circle(hd2, 3.8, SKIN, 8)
			_circle(hd2 + Vector2(0, -1.4), 3.9, Color(0.227, 0.157, 0.133), 8)
			_circle(hd2 + Vector2(0, 1.2), 3.2, SKIN, 8)
			draw_rect(Rect2(hd2.x - 3.4, hd2.y - 3.4, 6.8, 1.6), Color(0.886, 0.729, 0.353))
			if freed:
				for k in 3:
					var a := time * 2.0 + k * 2.1
					_circle(p + Vector2(cos(a) * 9.0, sin(a) * 5.0 - 6.0), 1.0, Color(1, 0.94, 0.71), 5)


# ---------------------------------------------------------------- objets & effets
func _draw_pickups() -> void:
	for pk in worlds[cur].pickups:
		if pk.taken:
			continue
		var bob := sin(time * 3.0 + pk.bob) * 1.5
		var p: Vector2 = pk.p + Vector2(0, bob)
		_shadow(pk.p + Vector2(0, 6), 4.0, 1.5, 0.8)
		match pk.kind:
			PK.HEART:
				_heart_shape(p, Color(0.863, 0.204, 0.243), 1.0)
			PK.HEART_CONTAINER:
				var pulse := 1.0 + 0.12 * sin(time * 5.0)
				_heart_shape(p, Color(0.863, 0.204, 0.243), pulse)
			PK.GEM:
				_poly(PackedVector2Array([p + Vector2(0, -4.2), p + Vector2(3.6, -1),
					p + Vector2(0, 4.2), p + Vector2(-3.6, -1)]), Color(0.275, 0.745, 0.784))
				_poly(PackedVector2Array([p + Vector2(0, -4.2), p + Vector2(3.6, -1), p + Vector2(0, -0.5)]),
					Color(0.588, 0.925, 0.941))
			PK.KEY:
				var c := Color(0.886, 0.729, 0.353)
				_circle(p + Vector2(-2.5, 0), 2.6, c, 8)
				_circle(p + Vector2(-2.5, 0), 1.1, Color(0.16, 0.125, 0.094), 5)
				draw_line(p + Vector2(-0.4, 0), p + Vector2(5.5, 0), c, 1.6, true)
				draw_rect(Rect2(p.x + 3.4, p.y + 0.8, 1.4, 2.4), c)
				draw_rect(Rect2(p.x + 5.2, p.y + 0.8, 1.4, 2.4), c)
			PK.MIRROR:
				_circle(p + Vector2(0, -1), 4.2, Color(0.58, 0.6, 0.66), 10)
				_circle(p + Vector2(0, -1), 3.2, Color(0.77, 0.82, 0.88), 9)
				draw_line(p + Vector2(0, 2.6), p + Vector2(0, 6), Color(0.58, 0.6, 0.66), 1.8, true)
				if sin(time * 2.4) > 0.0:
					draw_line(p + Vector2(-1.5, -2.5), p + Vector2(0.5, -0.5), Color.WHITE, 1.0, true)
			PK.SWORD:
				draw_line(p + Vector2(-5, 4), p + Vector2(4, -5), Color(0.94, 0.965, 1), 2.4, true)
				draw_line(p + Vector2(-4, 5.6), p + Vector2(-1.6, 3.2), Color(0.706, 0.588, 0.353), 1.6, true)
				draw_line(p + Vector2(-6.4, 2.6), p + Vector2(-3.6, 5.4), Color(0.588, 0.47, 0.275), 1.4, true)


func _heart_shape(p: Vector2, c: Color, sc: float) -> void:
	_circle(p + Vector2(-1.8 * sc, -1.0 * sc), 2.4 * sc, c, 7)
	_circle(p + Vector2(1.8 * sc, -1.0 * sc), 2.4 * sc, c, 7)
	_poly(PackedVector2Array([p + Vector2(-4.0 * sc, -0.2 * sc), p + Vector2(4.0 * sc, -0.2 * sc),
		p + Vector2(0, 4.6 * sc)]), c)
	_circle(p + Vector2(-1.2 * sc, -1.6 * sc), 0.9 * sc, Color(0.957, 0.47, 0.47), 5)


func _draw_shots() -> void:
	for s in shots:
		if s.fire:
			var r := 3.2 + absf(sin(time * 9.0)) * 0.8
			_circle(s.p, r, Color(1, 0.549, 0.196), 8)
			_circle(s.p, r * 0.55, Color(1, 0.863, 0.51), 6)
		else:
			var a := time * 12.0
			_circle(s.p, 2.4, Color(0.549, 0.51, 0.47), 7)
			draw_line(s.p + Vector2(cos(a), sin(a)) * 2.0, s.p - Vector2(cos(a), sin(a)) * 2.0,
				Color(0.376, 0.345, 0.314), 0.9, true)


func _draw_particles() -> void:
	for pt in parts:
		var a: float = clampf(pt.life / pt.max, 0.0, 1.0)
		_circle(pt.p, pt.size * a, Color(pt.col.r, pt.col.g, pt.col.b, a), 6)


# ---------------------------------------------------------------- lumières (additives)
func _draw_lights() -> void:
	var off := cam_draw()
	var z := _zoom()
	var vw := get_viewport_rect().size.x / z
	var vh := get_viewport_rect().size.y / z
	var tx0 := int(floorf(off.x / TILE)) - 2
	var ty0 := int(floorf(off.y / TILE)) - 2
	var tx1 := int(ceilf((off.x + vw) / TILE)) + 2
	var ty1 := int(ceilf((off.y + vh) / TILE)) + 2
	for ty in range(ty0, ty1 + 1):
		for tx in range(tx0, tx1 + 1):
			var t := tile(tx, ty)
			var c := Vector2((tx + 0.5) * TILE, (ty + 0.35) * TILE)
			if t == T.BRAZIER:
				var flick := 0.85 + 0.15 * sin(time * 8.0 + tx * 31 + ty * 17)
				_light_blob(light_layer, c, 62.0, pcol(15), 0.85 * flick)
			elif t == T.PORTAL:
				_light_blob(light_layer, Vector2((tx + 0.5) * TILE, (ty + 0.5) * TILE), 46.0,
					Color(0.59, 0.47, 0.9), 0.5 + 0.2 * sin(time * 2.0))
	for s in shots:
		if s.fire:
			_light_blob(light_layer, s.p, 30.0, Color(1, 0.59, 0.24), 0.8)
	for f in foes:
		if f.alive and f.kind == F.GANAR:
			_light_blob(light_layer, f.p + Vector2(7, -15), 40.0, Color(0.67, 0.35, 1), 0.8)
	var npc = world().npc
	if npc != null:
		if npc.ch == "Y" and ganar_dead:
			_light_blob(light_layer, npc.p, 50.0, Color(1, 0.86, 0.67), 0.5)
		elif npc.ch == "F":
			_light_blob(light_layer, (npc.p as Vector2) + Vector2(0, -8), 44.0,
				Color(0.59, 0.94, 0.82), 0.55 + 0.2 * sin(time * 3.0))


func _light_blob(node: CanvasItem, c: Vector2, r: float, col: Color, strength: float) -> void:
	# Quatre disques concentriques : faux dégradé radial, vrai halo additif.
	for k in 4:
		var f := float(k + 1) / 4.0
		node.draw_circle(c, r * f, Color(col.r, col.g, col.b, strength * (1.0 - f) * 0.6))
