# Le point d'entrée : construit l'arbre, tient les écrans, cadre la caméra.
#
# L'arbre est monté en code plutôt que dans une scène : un .tscn écrit à la main
# est fragile, et ici la structure tient en trente lignes qui se lisent.
#
#   Main
#   ├── World          (Node2D)   décor, personnages, matière, lumières
#   ├── Camera2D
#   ├── CanvasLayer 1  émissif — suit la caméra, jamais assombri par l'ambiance
#   ├── CanvasLayer 2  étalonnage (vignette, grain, éclair, fondu)
#   └── CanvasLayer 3  interface

extends Node

const CAM_ROOM := 1.0
const CAM_FOLLOW := 2.0

var world: World
var cam: Camera2D
var hud: HUD
var sfx: Sfx
var grade: ColorRect
var emissive: Node2D

var inp := InputState.new()
var screen := "title"
var intro_t := 0.0
var menu_index := 0
var fade := 1.0
var _last_hp := 3
var _last_state := -1
var _step_cd := 0.0
var _selftest := false
var _shot_path := ""
var _shot_level := 0
var _shot_frames := 90
var _shot_at := Vector2i(-1, -1)
var _shot_follow := false
var _shot_left := -1

func _ready() -> void:
	_register_input()
	get_window().title = "Prince of Persia"

	world = World.new()
	world.name = "World"
	add_child(world)

	cam = Camera2D.new()
	cam.enabled = true
	add_child(cam)

	var em_layer := CanvasLayer.new()
	em_layer.layer = 1
	em_layer.follow_viewport_enabled = true
	add_child(em_layer)
	emissive = Node2D.new()
	emissive.set_script(preload("res://scripts/nodes/emissive.gd"))
	emissive.set("world", world)
	em_layer.add_child(emissive)
	world.attach_emissive(emissive)

	var gl := CanvasLayer.new()
	gl.layer = 2
	add_child(gl)
	grade = ColorRect.new()
	grade.set_anchors_preset(Control.PRESET_FULL_RECT)
	grade.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var mat := ShaderMaterial.new()
	mat.shader = preload("res://shaders/grade.gdshader")
	grade.material = mat
	gl.add_child(grade)

	var hl := CanvasLayer.new()
	hl.layer = 3
	add_child(hl)
	hud = HUD.new()
	hud.world = world
	hl.add_child(hud)

	sfx = Sfx.new()
	add_child(sfx)

	world.said.connect(func(_t, _w): pass)
	_set_screen("title")

	# Mode d'autotest : lance la campagne et la joue au hasard, sans affichage.
	# C'est ce qui permet de vérifier en intégration que les six niveaux se
	# chargent et que la simulation tourne sans lever d'erreur.
	var args := OS.get_cmdline_user_args()
	for i in args.size():
		var a: String = args[i]
		if a == "--selftest":
			_selftest = true
		elif a == "--shot" and i + 1 < args.size():
			_shot_path = args[i + 1]
		elif a == "--level" and i + 1 < args.size():
			_shot_level = int(args[i + 1]) - 1
		elif a == "--frames" and i + 1 < args.size():
			_shot_frames = int(args[i + 1])
		elif a == "--at" and i + 1 < args.size():
			var xy: PackedStringArray = args[i + 1].split(",")
			if xy.size() == 2:
				_shot_at = Vector2i(int(xy[0]), int(xy[1]))
		elif a == "--follow":
			_shot_follow = true
	if _selftest:
		_run_selftest()
	elif _shot_path != "":
		_begin_shot()

func _register_input() -> void:
	var table := {
		"pop_left": [KEY_LEFT, KEY_A, KEY_Q],
		"pop_right": [KEY_RIGHT, KEY_D],
		"pop_up": [KEY_UP, KEY_W, KEY_Z],
		"pop_down": [KEY_DOWN, KEY_S],
		"pop_careful": [KEY_SHIFT],
		"pop_attack": [KEY_SPACE, KEY_X],
		"pop_parry": [KEY_ALT, KEY_E],
		"pop_throw": [KEY_T],
		"pop_cast": [KEY_F],
		"pop_sheathe": [KEY_C],
		"pop_pause": [KEY_ESCAPE, KEY_P],
		"pop_restart": [KEY_R],
		"pop_zoom": [KEY_V],
		"pop_fullscreen": [KEY_F11],
		"pop_menu_up": [KEY_UP],
		"pop_menu_down": [KEY_DOWN],
		"pop_confirm": [KEY_ENTER, KEY_SPACE, KEY_KP_ENTER],
	}
	for action in table:
		if not InputMap.has_action(action):
			InputMap.add_action(action)
		for key in table[action]:
			var ev := InputEventKey.new()
			ev.physical_keycode = key
			InputMap.action_add_event(action, ev)

func _set_screen(s: String) -> void:
	screen = s
	hud.screen = s
	if s == "intro":
		intro_t = 1.8

func _start_run() -> void:
	var err := world.load_level(0, {"hp_max": 3, "sword": false, "scimitar": false,
		"buckler": false, "wand": false, "daggers": 0}, 1234567)
	if err != "":
		push_error(err)
		return
	_last_hp = world.player.hp
	_set_screen("intro")

func _next_level() -> void:
	var i := world.idx + 1
	if i >= Levels.CAMPAIGN.size():
		_set_screen("won")
		return
	world.load_level(i, world.carry, world.rng.randi())
	_set_screen("intro")

func _process(dt: float) -> void:
	_update_camera()
	_update_grade(dt)

	if Input.is_action_just_pressed("pop_fullscreen"):
		var m := DisplayServer.window_get_mode()
		DisplayServer.window_set_mode(DisplayServer.WINDOW_MODE_WINDOWED
			if m == DisplayServer.WINDOW_MODE_FULLSCREEN
			else DisplayServer.WINDOW_MODE_FULLSCREEN)

	if _shot_left >= 0:
		inp.clear()
		world.step_frame(1.0 / 60.0, inp)
		_shot_left -= 1
		if _shot_left <= 0:
			await RenderingServer.frame_post_draw
			_finish_shot()
		return

	match screen:
		"title": _tick_title()
		"intro": _tick_intro(dt)
		"play": _tick_play(dt)
		"pause": _tick_pause()
		"dead": _tick_dead()
		"won": _tick_won()

func _tick_title() -> void:
	fade = maxf(fade - 0.03, 0.0)
	if Input.is_action_just_pressed("pop_menu_down"):
		menu_index = (menu_index + 1) % HUD.MENU_ITEMS.size()
	if Input.is_action_just_pressed("pop_menu_up"):
		menu_index = (menu_index + HUD.MENU_ITEMS.size() - 1) % HUD.MENU_ITEMS.size()
	hud.menu_index = menu_index
	if Input.is_action_just_pressed("pop_confirm"):
		match menu_index:
			0: _start_run()
			1: _set_screen("pause")
			2: get_tree().quit()
	if Input.is_action_just_pressed("pop_pause"):
		get_tree().quit()

func _tick_intro(dt: float) -> void:
	intro_t -= dt
	fade = clampf(intro_t / 1.8, 0.0, 1.0) * 0.0
	if intro_t <= 0.0 or Input.is_action_just_pressed("pop_confirm"):
		_set_screen("play")
		fade = 1.0

func _tick_play(dt: float) -> void:
	fade = maxf(fade - dt * 2.2, 0.0)
	if Input.is_action_just_pressed("pop_pause"):
		_set_screen("pause")
		return
	if Input.is_action_just_pressed("pop_restart"):
		world.restart()
		return
	if Input.is_action_just_pressed("pop_zoom"):
		world.zoom = CAM_FOLLOW if world.zoom <= 1.001 else CAM_ROOM
		world.centre_camera()

	inp.poll()
	world.step_frame(dt, inp)
	_audio_cues(dt)

	if world.phase == World.Phase.LEVEL_DONE:
		_next_level()
	elif world.phase == World.Phase.DEAD or world.phase == World.Phase.TIME_UP:
		_set_screen("dead")

func _tick_pause() -> void:
	if Input.is_action_just_pressed("pop_pause") or Input.is_action_just_pressed("pop_confirm"):
		_set_screen("play" if world.lv != null else "title")

func _tick_dead() -> void:
	if Input.is_action_just_pressed("pop_confirm") or Input.is_action_just_pressed("pop_restart"):
		world.restart()
		_set_screen("play")
		fade = 1.0

func _tick_won() -> void:
	if Input.is_action_just_pressed("pop_confirm"):
		_set_screen("title")
		fade = 1.0

# ---------------------------------------------------------------- caméra

func _update_camera() -> void:
	var vp := get_viewport().get_visible_rect().size
	if world.lv == null:
		return
	# Au cadrage d'origine, la salle occupe toute la largeur ; l'espace qui reste
	# en hauteur laisse voir la corniche d'en haut et la dalle d'en bas, ce qui
	# donne du contexte sans trahir le cadrage à la salle.
	var room_w := Geom.ROOM_W / world.zoom
	var z := vp.x / room_w
	cam.zoom = Vector2(z, z)
	world.set_view_size(vp.x / z, vp.y / z)
	cam.position = world.camera_centre()

func _update_grade(dt: float) -> void:
	var m: ShaderMaterial = grade.material
	var tint := Color(0.03, 0.03, 0.06)
	if world.lv != null:
		tint = world.lv.theme["vignette"]
	m.set_shader_parameter("vignette_tint", Vector3(tint.r, tint.g, tint.b))
	m.set_shader_parameter("time_s", Time.get_ticks_msec() / 1000.0)
	var f := 0.0
	var fl := 0.0
	if world.lv != null:
		fl = clampf(world.flash_t, 0.0, 1.0)
		m.set_shader_parameter("flash_colour",
			Vector3(world.flash_col.r, world.flash_col.g, world.flash_col.b))
		match world.phase:
			World.Phase.DYING: f = 1.0 - clampf(world.phase_t / 2.1, 0.0, 1.0)
			World.Phase.LEAVING: f = 1.0 - clampf(world.phase_t / 1.4, 0.0, 1.0)
			World.Phase.DEAD, World.Phase.LEVEL_DONE, World.Phase.TIME_UP: f = 1.0
	if screen == "title" or screen == "intro" or screen == "won":
		f = 1.0
	m.set_shader_parameter("flash", fl)
	m.set_shader_parameter("fade", maxf(f * 0.92, fade))

# ---------------------------------------------------------------- audio

func _audio_cues(dt: float) -> void:
	var pl := world.player
	if pl.hp < _last_hp:
		sfx.play("hit", -4.0)
	_last_hp = pl.hp

	if pl.st != _last_state:
		match pl.st:
			Prince.S.JUMP_UP, Prince.S.JUMP_RUN: sfx.play("jump", -12.0)
			Prince.S.LAND: sfx.play("land", -9.0)
			Prince.S.DRINK: sfx.play("potion", -8.0)
			Prince.S.DEAD: sfx.play("death", -4.0)
			Prince.S.STRIKE: sfx.play("clash", -12.0, 1.2)
		_last_state = pl.st

	# Les pas suivent le cycle de course plutôt qu'une horloge : ils tombent donc
	# toujours quand le pied touche le sol.
	if pl.st == Prince.S.RUN:
		_step_cd -= dt
		if _step_cd <= 0.0:
			_step_cd = Geom.STRIDE_PX / maxf(pl.speed(), 1.0) * 0.5
			sfx.play("step", -18.0, world.rng.randf_range(0.9, 1.1))
	else:
		_step_cd = 0.0


# ---------------------------------------------------------------- autotest

## Charge les six niveaux, vérifie leur cohérence, puis joue quelques secondes de
## simulation sur chacun avec des commandes tirées au sort. Sert de test
## d'intégration : `godot --headless -- --selftest`.
func _run_selftest() -> void:
	var rng := RandomNumberGenerator.new()
	rng.seed = 20260730
	var fail := 0
	for i in Levels.CAMPAIGN.size():
		var err := world.load_level(i, {"hp_max": 3, "sword": true, "scimitar": false,
			"buckler": false, "wand": true, "daggers": 6}, 4242 + i)
		if err != "":
			print("ÉCHEC  niveau %d : %s" % [i + 1, err])
			fail += 1
			continue
		var lvl := world.lv
		var probe := InputState.new()
		for step in 900:
			probe.clear()
			probe.left = rng.randf() < 0.22
			probe.right = rng.randf() < 0.38
			probe.up = rng.randf() < 0.12
			probe.up_edge = probe.up and rng.randf() < 0.5
			probe.down = rng.randf() < 0.08
			probe.attack = rng.randf() < 0.05
			probe.careful = rng.randf() < 0.1
			world.step_frame(1.0 / 60.0, probe)
			if world.phase == World.Phase.DEAD or world.phase == World.Phase.TIME_UP:
				world.restart()
		print("OK     niveau %d : %-26s %2d x %2d salles, %3d salles jouables, %2d gardes, %2d objets"
			% [i + 1, lvl.name, lvl.rw, lvl.rh, lvl.playable_rooms(),
				lvl.mobs.size(), lvl.items.size()])
	print("autotest : %d échec(s)" % fail)
	get_tree().quit(1 if fail > 0 else 0)


# ---------------------------------------------------------------- captures

## Mode capture : charge un niveau, place éventuellement le prince, simule
## quelques images puis écrit un PNG. Sert à relire l'art sans lancer de partie :
##   godot --rendering-driver opengl3 -- --shot art.png --level 3 --at 24,7
func _begin_shot() -> void:
	var err := world.load_level(_shot_level, {"hp_max": 3, "sword": true,
		"scimitar": false, "buckler": false, "wand": true, "daggers": 6}, 909090)
	if err != "":
		push_error(err)
		get_tree().quit(1)
		return
	if _shot_at.x >= 0:
		world.player.p = Vector2(Geom.cx(_shot_at.x), Geom.surf(_shot_at.y))
		world.player.fall_from = world.player.p.y
	if _shot_follow:
		world.zoom = CAM_FOLLOW
	world.centre_camera()
	_set_screen("play")
	fade = 0.0
	_shot_left = _shot_frames

func _finish_shot() -> void:
	var img := get_viewport().get_texture().get_image()
	var dir := _shot_path.get_base_dir()
	if dir != "" and not DirAccess.dir_exists_absolute(dir):
		DirAccess.make_dir_recursive_absolute(dir)
	img.save_png(_shot_path)
	print("écrit %s (%dx%d)" % [_shot_path, img.get_width(), img.get_height()])
	get_tree().quit(0)
