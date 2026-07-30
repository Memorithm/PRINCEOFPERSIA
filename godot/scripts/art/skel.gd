# Rendu des personnages articulés.
#
# Plutôt que de stocker des planches de sprites, chaque personnage du jeu est un
# petit squelette — hanche, torse, tête, deux bras, deux jambes — qui est
# *dessiné* à chaque image. Chaque os porte une silhouette écrite (Shape.Profile)
# pour qu'un bras enfle au deltoïde et se pince au coude, chacun est ombré en cel
# avec une ombre propre à bord franc et un fin liseré éclairé, et les têtes, les
# mains, les cheveux et l'étoffe sont des polygones écrits à la main plutôt que
# des ellipses. Là où deux formes de même couleur se recouvrent — le bras proche
# contre la poitrine, l'écharpe contre le ventre — un contour dessiné les sépare.
#
# Les animations sont des angles d'articulation clés, ce qui rend praticable
# d'avoir la course, le saut, l'escalade, la suspension, trois gardes d'escrime
# et cinq morphologies qui appartiennent au même monde.
#
# Convention d'angle, partout : **degrés mesurés depuis la verticale basse,
# positifs en tournant vers la direction du regard.** Un membre à 0 pend droit.
# Genoux et coudes sont donnés en flexion, positive dans le sens naturel.
#
# Tout est dessiné en unités monde (pixels d'art) : la caméra s'occupe de
# l'échelle, donc aucune routine de dessin ne sait quelle est la résolution.

class_name Skel

enum Blade { NONE, SWORD, SCIMITAR, DAGGER, WAND }

# ---------------------------------------------------------------- pose

class Pose:
	var hip := 13.0        ## Hauteur de la hanche au-dessus du sol.
	var lean := 0.0        ## Décalage de la hanche vers l'avant.
	var torso := 0.0       ## Inclinaison du torse, degrés vers l'avant.
	var head := 0.0        ## Angle de la tête par rapport au torse.
	var arm := [[0.0, 8.0], [0.0, 8.0]]  ## [proche, lointain] x [épaule, coude]
	var leg := [[0.0, 2.0], [0.0, 2.0]]  ## [proche, lointain] x [hanche, genou]
	var sword := 0.0       ## Angle de la lame par rapport à l'avant-bras proche.
	var tail := 1.0        ## Traîne de l'écharpe.
	var squash := 1.0      ## Écrasement, 1.0 = neutre (réceptions).

	func copy() -> Pose:
		var p := Pose.new()
		p.hip = hip; p.lean = lean; p.torso = torso; p.head = head
		p.sword = sword; p.tail = tail; p.squash = squash
		p.arm = [[arm[0][0], arm[0][1]], [arm[1][0], arm[1][1]]]
		p.leg = [[leg[0][0], leg[0][1]], [leg[1][0], leg[1][1]]]
		return p

	## Échange membres proches et lointains — transforme une demi-foulée en
	## l'autre.
	func mirrored() -> Pose:
		var p := copy()
		var a: Array = p.arm[0]
		p.arm[0] = p.arm[1]
		p.arm[1] = a
		var l: Array = p.leg[0]
		p.leg[0] = p.leg[1]
		p.leg[1] = l
		return p

	func with_sword(a: float) -> Pose:
		var p := copy()
		p.sword = a
		return p

	func with_squash(s: float) -> Pose:
		var p := copy()
		p.squash = s
		return p

	func with_tail(t: float) -> Pose:
		var p := copy()
		p.tail = t
		return p

	func blend(o: Pose, t: float) -> Pose:
		var p := Pose.new()
		p.hip = lerpf(hip, o.hip, t)
		p.lean = lerpf(lean, o.lean, t)
		p.torso = lerpf(torso, o.torso, t)
		p.head = lerpf(head, o.head, t)
		p.sword = lerpf(sword, o.sword, t)
		p.tail = lerpf(tail, o.tail, t)
		p.squash = lerpf(squash, o.squash, t)
		for i in 2:
			for j in 2:
				p.arm[i][j] = lerpf(arm[i][j], o.arm[i][j], t)
				p.leg[i][j] = lerpf(leg[i][j], o.leg[i][j], t)
		return p

static func rest() -> Pose:
	return Pose.new()

## Constructeur commode : ps(hip, lean, torse, tête, bras_proche, bras_loin,
## jambe_proche, jambe_loin) — chaque membre en (épaule/hanche, coude/genou).
static func ps(hip: float, lean: float, torso: float, head: float,
		an: Array, af: Array, ln: Array, lf: Array) -> Pose:
	var p := Pose.new()
	p.hip = hip
	p.lean = lean
	p.torso = torso
	p.head = head
	p.arm = [[an[0], an[1]], [af[0], af[1]]]
	p.leg = [[ln[0], ln[1]], [lf[0], lf[1]]]
	return p

# ---------------------------------------------------------------- proportions

class Prop:
	var thigh := 7.0
	var shin := 6.8
	var foot := 3.2
	var torso := 7.4
	var neck := 1.15
	var head_r := 2.10
	var upper := 5.3
	var fore := 4.8
	var hand := 1.5
	var chest := 2.60   ## Demi-profondeur du buste, vu de côté.
	var waist := 2.05
	var scale := 1.0
	var girth := 1.0

	func scaled(s: float, g: float) -> Prop:
		var p := Prop.new()
		p.thigh = thigh; p.shin = shin; p.foot = foot; p.torso = torso
		p.neck = neck; p.head_r = head_r; p.upper = upper; p.fore = fore
		p.hand = hand; p.chest = chest; p.waist = waist
		p.scale = s
		p.girth = g
		return p

	## Portée des mains au-dessus des pieds, bras tendus. C'est ce avec quoi
	## Geom.HANG_DROP doit s'accorder, sinon les mains flottent au-dessus de la
	## corniche qu'elles sont censées agripper.
	func reach_up() -> float:
		return (thigh + shin + torso + upper + fore) * scale

## La stature du prince : environ 27 pixels d'art debout — un peu moins que les
## 31 pixels de dégagement que laisse une tuile, et proche du rapport
## prince/plancher de l'original.
static func prince_prop() -> Prop:
	return Prop.new()

# ---------------------------------------------------------------- costume

class Style:
	var skin := Color8(226, 172, 124)
	var skin_dk := Color8(150, 96, 62)
	var cloth := Color8(238, 236, 226)
	var cloth_dk := Color8(150, 152, 166)
	var sash := Color8(190, 48, 48)
	var sash_dk := Color8(104, 20, 26)
	var hair := Color8(70, 44, 32)
	var boot := Color8(126, 78, 44)
	var metal := Color8(198, 208, 222)
	var outline := Color8(20, 16, 28)
	var head_wrap: Variant = null  ## Turban / casque ; null = cheveux nus.
	var robe := 0.0                ## 0 aucune, 0.5 tunique courte, 1 robe longue.
	var bones := false             ## Dessiner en squelette.
	var shield: Variant = null
	var plume: Variant = null
	var belt := false
	var bare_chest := true         ## Torse nu, modelé en peau.
	var vest: Variant = null       ## Gilet ouvert : couvre le dos, laisse le buste.
	var trouser := Color8(126, 158, 164)
	var baggy := 0.85              ## 0 ajusté, 1 resserré à la cheville.
	var band: Variant = Color8(150, 40, 44)  ## Bandeau noué sur les cheveux.
	var scarf: Variant = null      ## Long ruban traînant derrière les épaules.

	func copy() -> Style:
		var s := Style.new()
		for p in ["skin", "skin_dk", "cloth", "cloth_dk", "sash", "sash_dk",
				"hair", "boot", "metal", "outline", "head_wrap", "robe", "bones",
				"shield", "plume", "belt", "bare_chest", "vest", "trouser",
				"baggy", "band", "scarf"]:
			s.set(p, get(p))
		return s

static func prince_style() -> Style:
	return Style.new()

## Traitement des membres du côté opposé : plus sombre, plus froid, un peu
## désaturé. Doit tomber *sous* l'ombre propre du torse, sinon la profondeur se
## lit à l'envers et le bras lointain semble flotter devant la poitrine — la
## façon la plus simple de rendre une figure fausse.
static func recede(c: Color) -> Color:
	return Geom.desat(Geom.shade(c, 0.48), 0.26).lerp(Color8(26, 24, 42), 0.18)

# ---------------------------------------------------------------- squelette résolu

class Figure:
	var hip := Vector2.ZERO
	var knee := [Vector2.ZERO, Vector2.ZERO]
	var ankle := [Vector2.ZERO, Vector2.ZERO]
	var toe := [Vector2.ZERO, Vector2.ZERO]
	var shoulder := Vector2.ZERO
	var elbow := [Vector2.ZERO, Vector2.ZERO]
	var hand := [Vector2.ZERO, Vector2.ZERO]
	var neck := Vector2.ZERO
	var head := Vector2.ZERO
	var head_r := 2.1
	var up := Vector2.UP       ## Unitaire hanche → épaule.
	var fwd := Vector2.RIGHT   ## Unitaire perpendiculaire au torse, vers l'avant.
	var facing := 1.0
	var hand_dir := Vector2.RIGHT  ## Direction de l'avant-bras proche (arme).
	var prop: Prop
	var squash := 1.0
	var girth := 1.0   ## prop.girth * prop.scale, en pixels d'art.
	var unit := 1.0    ## prop.scale.
	var chest := 2.6   ## Demi-profondeur du buste, en pixels d'art.
	var tl := 7.4      ## Distance hanche → épaule.
	var heavy := false

static func _dir_down(deg: float) -> Vector2:
	var r := deg_to_rad(deg)
	return Vector2(sin(r), cos(r))

static func _dir_up(deg: float) -> Vector2:
	var r := deg_to_rad(deg)
	return Vector2(sin(r), -cos(r))

## Cinématique directe. `feet` est le point monde à la base du personnage.
##
## `facing` porte à la fois le sens et l'amplitude : le signe reflète la figure,
## l'amplitude la comprime horizontalement, si bien qu'un demi-tour passe par une
## pose de profil au lieu de claquer d'un côté à l'autre.
static func solve(pose: Pose, prop: Prop, feet: Vector2, facing: float) -> Figure:
	var s := prop.scale
	var sq := pose.squash
	var turn := clampf(absf(facing), 0.16, 1.0)
	# L'écrasement vertical pivote sur les pieds ; l'étirement horizontal
	# conserve le volume.
	var vs := s * sq
	var hs := s * maxf(2.0 - sq, 0.6) * turn

	var f := Figure.new()
	f.prop = prop
	f.squash = sq
	f.girth = prop.girth * s
	f.unit = s
	f.chest = prop.chest * prop.girth * s
	f.head_r = prop.head_r * s
	f.heavy = prop.girth > 1.20
	f.facing = -1.0 if facing < 0.0 else 1.0

	f.hip = Vector2(feet.x + pose.lean * hs, feet.y - pose.hip * vs)

	for i in 2:
		var h: float = pose.leg[i][0]
		var k: float = pose.leg[i][1]
		var d1 := _dir_down(h)
		f.knee[i] = f.hip + Vector2(d1.x * prop.thigh * hs, d1.y * prop.thigh * vs)
		var d2 := _dir_down(h - k)
		f.ankle[i] = f.knee[i] + Vector2(d2.x * prop.shin * hs, d2.y * prop.shin * vs)
		# Le pied reste à peu près perpendiculaire au tibia.
		var d3 := _dir_down(h - k + 96.0)
		f.toe[i] = f.ankle[i] + Vector2(d3.x * prop.foot * hs, d3.y * prop.foot * vs)

	var du := _dir_up(pose.torso)
	f.shoulder = f.hip + Vector2(du.x * prop.torso * hs, du.y * prop.torso * vs)
	var dn := _dir_up(pose.torso + pose.head)
	f.neck = f.shoulder + Vector2(dn.x * prop.neck * hs, dn.y * prop.neck * vs)
	f.head = f.neck + Vector2(dn.x * prop.head_r * hs, dn.y * prop.head_r * vs)

	for i in 2:
		var a: float = pose.arm[i][0] + pose.torso
		var e: float = pose.arm[i][1]
		var d1 := _dir_down(a)
		f.elbow[i] = f.shoulder + Vector2(d1.x * prop.upper * hs, d1.y * prop.upper * vs)
		var d2 := _dir_down(a + e)
		f.hand[i] = f.elbow[i] + Vector2(d2.x * prop.fore * hs, d2.y * prop.fore * vs)
		if i == 0:
			f.hand_dir = d2

	f.up = (f.shoulder - f.hip).normalized()
	# orthogonal() de (0,-1) donne (-1,0) ; on veut +x quand le corps est droit.
	f.fwd = -f.up.orthogonal()
	f.tl = maxf(f.shoulder.distance_to(f.hip), 0.5)

	if facing < 0.0:
		var px := feet.x
		f.hip = _flip(f.hip, px)
		f.shoulder = _flip(f.shoulder, px)
		f.neck = _flip(f.neck, px)
		f.head = _flip(f.head, px)
		for i in 2:
			f.knee[i] = _flip(f.knee[i], px)
			f.ankle[i] = _flip(f.ankle[i], px)
			f.toe[i] = _flip(f.toe[i], px)
			f.elbow[i] = _flip(f.elbow[i], px)
			f.hand[i] = _flip(f.hand[i], px)
		f.hand_dir = Vector2(-f.hand_dir.x, f.hand_dir.y)
		f.up = Vector2(-f.up.x, f.up.y)
		f.fwd = Vector2(-f.fwd.x, f.fwd.y)
	return f

static func _flip(p: Vector2, pivot: float) -> Vector2:
	return Vector2(2.0 * pivot - p.x, p.y)

## Boîte englobante monde d'une figure, marge comprise.
static func bbox(f: Figure, pad: float) -> Rect2:
	var r := Rect2(f.hip, Vector2.ZERO)
	for p in [f.shoulder, f.head, f.knee[0], f.knee[1], f.ankle[0], f.ankle[1],
			f.toe[0], f.toe[1], f.elbow[0], f.elbow[1], f.hand[0], f.hand[1]]:
		r = r.expand(p)
	return r.grow(pad)

# ---------------------------------------------------------------- dessin

## Peint une figure. L'appelant a déjà posé la transformation monde.
static func draw_figure(ci: CanvasItem, f: Figure, st: Style, pose: Pose, blade: int) -> void:
	# Lumière clé venue d'en haut et légèrement de face, pour que le visage et la
	# poitrine l'accrochent quel que soit le sens du regard.
	var light := Vector2(f.facing * 0.40, -0.92).normalized()

	draw_silhouette(ci, f, st, pose, blade)

	if st.bones:
		_draw_bones(ci, f, st, pose, blade, light)
		return

	# --- côté opposé ----------------------------------------------------
	_draw_leg(ci, f, st, 1, light, true)
	_draw_arm(ci, f, st, 1, light, true)
	if st.shield != null:
		_draw_shield(ci, f, st, st.shield, light)
	_draw_scarf(ci, f, st, pose)
	_draw_sash_tail(ci, f, st, pose)

	# --- jambe proche, torse, puis le vêtement sur les cuisses ----------
	_draw_leg(ci, f, st, 0, light, false)
	_draw_torso(ci, f, st, light)
	if st.robe > 0.01:
		_draw_tunic(ci, f, st, pose, light)
	_draw_sash(ci, f, st, light)

	# --- tête ------------------------------------------------------------
	_draw_head(ci, f, st, light)

	# --- bras proche et arme --------------------------------------------
	_draw_arm(ci, f, st, 0, light, false)
	_draw_blade(ci, f, st, pose, blade)


## La silhouette entière, dilatée et peinte en une seule valeur sombre avant que
## la figure ne se pose dessus.
##
## Le contour est *dessiné*, pas déduit : on reprend les mêmes formes que le rendu
## et on les gonfle. Un détourage par dilatation d'image serait plus fidèle mais
## demanderait un rendu hors écran par personnage ; à cette échelle, la différence
## ne se voit pas, et le trait reste net à tous les grossissements.
static func draw_silhouette(ci: CanvasItem, f: Figure, st: Style, pose: Pose,
		blade: int) -> void:
	var col := st.outline
	var g := f.girth
	var pad := 0.42 * maxf(f.unit, 0.6)

	# Jambes et bottes.
	var loose := st.baggy > 0.15 and not st.bones
	for i in 2:
		var hip := f.hip
		var knee: Vector2 = f.knee[i]
		var ankle: Vector2 = f.ankle[i]
		var toe: Vector2 = f.toe[i]
		var pt := Shape.BONE if st.bones else (Shape.THIGH_BAGGY if loose else Shape.THIGH)
		var pc := Shape.BONE if st.bones else (Shape.CALF_BAGGY if loose else Shape.CALF)
		Shape.poly(ci, Shape.Limb.new(hip, knee, Shape.front_of(hip, knee, f.facing),
			pt, g).with_steps(7).outline(pad), col)
		Shape.poly(ci, Shape.Limb.new(knee, ankle, Shape.front_of(knee, ankle, f.facing),
			pc, g).with_steps(7).outline(pad), col)
		if not st.bones:
			var sole := (toe - ankle).normalized()
			var up := sole.orthogonal()
			if up.y > 0.0:
				up = -up
			var heel := ankle - sole * (1.15 * g)
			Shape.poly(ci, Shape.grow(PackedVector2Array([
				heel + up * (1.95 * g), ankle + up * (2.20 * g), toe + up * (0.95 * g),
				toe + sole * (0.30 * g) - up * (0.26 * g), heel - up * (0.30 * g),
			]), pad), col)

	# Torse.
	var prof := Shape.TORSO
	if st.robe > 0.7:
		prof = Shape.TORSO_ROBE
	elif f.heavy:
		prof = Shape.TORSO_HEAVY
	Shape.poly(ci, Shape.Limb.new(f.shoulder, f.hip,
		Shape.front_of(f.shoulder, f.hip, f.facing), prof, f.chest)
		.with_steps(9).outline(pad), col)

	# Bras, décalés en profondeur comme au rendu.
	for i in 2:
		var off := f.fwd * ((-1.40 if i == 1 else 0.55) * g)
		var sh := f.shoulder + off
		var el: Vector2 = f.elbow[i] + off
		var hd: Vector2 = f.hand[i] + off
		var uf := Shape.front_of(sh, el, f.facing)
		var ff := Shape.front_of(el, hd, f.facing)
		Shape.poly(ci, Shape.Limb.new(sh, el, uf, Shape.UPPER_ARM, g)
			.with_steps(6).outline(pad), col)
		Shape.poly(ci, Shape.Limb.new(el, hd, ff, Shape.FOREARM, g)
			.with_steps(6).outline(pad), col)
		var along := (hd - el).normalized()
		Shape.poly(ci, Shape.grow(Shape.frame_xy(hd, along * g, ff * g, [
			-0.30, 0.74, 0.55, 0.86, 1.15, 0.60, 1.42, 0.10,
			1.30, -0.40, 0.55, -0.78, -0.30, -0.70,
		]), pad), col)

	# Cou, tête, chevelure ou coiffe.
	var hc := f.head
	var hr := f.head_r
	var ex := Vector2(f.facing * hr, 0.0)
	var ey := Vector2(0.0, hr)
	var nb := f.shoulder - f.up * (1.6 * g)
	var nt := hc - f.up * (0.35 * hr)
	Shape.poly(ci, Shape.Limb.new(nt, nb, Shape.front_of(nt, nb, f.facing),
		Shape.NECK, g).with_steps(4).outline(pad), col)
	Shape.poly(ci, Shape.grow(Shape.frame(hc, ex, ey,
		Shape.SKULL if st.bones else Shape.HEAD), pad), col)
	if st.bones:
		pass
	elif st.head_wrap == null:
		Shape.poly(ci, Shape.grow(Shape.frame(hc, ex, ey, Shape.HAIR), pad), col)
	else:
		Shape.poly(ci, Shape.grow(Shape.frame_xy(hc, ex, ey, [
			0.94, -0.30, 0.86, -0.70, 0.48, -1.16, -0.10, -1.36,
			-0.70, -1.20, -1.06, -0.78, -1.18, -0.28, -1.08, 0.00,
		]), pad), col)

	# Écharpe de taille : elle déborde la hanche et fait partie de la silhouette.
	if not st.bones:
		var waist := f.hip.lerp(f.shoulder, 0.20)
		var front := Shape.front_of(f.shoulder, f.hip, f.facing)
		var down := (f.hip - f.shoulder).normalized()
		var w := f.chest * 0.95
		Shape.poly(ci, Shape.grow(Shape.frame_xy(waist, front * w, down * g, [
			0.10, -1.35, 1.08, -1.00, 1.16, 0.80,
			0.10, 1.15, -1.06, 0.85, -1.10, -1.05,
		]), pad), col)

# ---------------------------------------------------------------- jambes

static func _draw_leg(ci: CanvasItem, f: Figure, st: Style, i: int, light: Vector2,
		far: bool) -> void:
	var g := f.girth
	var trouser := recede(st.trouser) if far else st.trouser
	var boot := recede(st.boot) if far else st.boot

	var hip := f.hip
	var knee: Vector2 = f.knee[i]
	var ankle: Vector2 = f.ankle[i]
	var toe: Vector2 = f.toe[i]
	var loose := st.baggy > 0.15
	var pt := Shape.THIGH_BAGGY if loose else Shape.THIGH
	var pc := Shape.CALF_BAGGY if loose else Shape.CALF
	var thigh := Shape.Limb.new(hip, knee, Shape.front_of(hip, knee, f.facing), pt, g).with_steps(9)
	var calf := Shape.Limb.new(knee, ankle, Shape.front_of(knee, ankle, f.facing), pc, g).with_steps(9)

	# La jambe proche reçoit un trait dessiné pour se détacher de la lointaine là
	# où les deux se recouvrent, la teinte de recul ne suffisant pas seule.
	if not far:
		var e := 0.34 * g
		var dk := Geom.shade(st.trouser, 0.26)
		thigh.edge(ci, e, dk)
		calf.edge(ci, e, dk)
	thigh.draw(ci, trouser, light)
	calf.draw(ci, trouser, light)

	if loose:
		# Deux plis le long de l'étoffe, et le fronçage dans la botte. Sans eux,
		# un pantalon bouffant est un sac.
		for spec in [[0.18, 0.72, 0.34], [0.42, 0.92, 0.22]]:
			var t0: float = spec[0]
			var t1: float = spec[1]
			Shape.contour(ci,
				thigh.dark_edge(t0, light).lerp(thigh.lit_edge(t0, light), 0.34),
				thigh.dark_edge(t1, light).lerp(thigh.lit_edge(t1, light), 0.42),
				0.20 * g, Geom.shade(st.trouser, 0.52), spec[2])
		var cf := Shape.front_of(knee, ankle, f.facing)
		var cuff := ankle.lerp(knee, 0.12)
		Shape.contour(ci, cuff + cf * (1.05 * g), cuff - cf * (1.05 * g),
			0.34 * g, Geom.shade(trouser, 0.72))
	else:
		# Un pli derrière le genou est ce qui transforme deux segments en une
		# articulation.
		var kf := Shape.front_of(hip, knee, f.facing)
		Shape.contour(ci, knee - kf * (1.20 * g), knee - kf * (0.30 * g),
			0.20 * g, Geom.shade(trouser, 0.46), 0.8)

	# --- botte : talon, semelle et bout en une seule forme ---------------
	var sole := (toe - ankle).normalized()
	# On choisit la perpendiculaire qui pointe vers le haut de l'écran, quel que
	# soit le sens du pied — sinon un personnage tourné à gauche porte ses bottes
	# à l'envers.
	var up := sole.orthogonal()
	if up.y > 0.0:
		up = -up
	var heel := ankle - sole * (1.15 * g)
	var pts := PackedVector2Array([
		heel + up * (1.95 * g),
		ankle + up * (2.20 * g),
		toe + up * (0.95 * g),
		toe + sole * (0.30 * g) - up * (0.26 * g),
		heel - up * (0.30 * g),
		heel - sole * (0.25 * g) + up * (0.70 * g),
	])
	Shape.cel_poly(ci, pts, boot, light, 0.40)
	Shape.contour(ci, heel - up * (0.16 * g), toe - up * (0.18 * g),
		0.34 * g, Geom.shade(boot, 0.40))
	var cf2 := sole * (0.85 * g)
	Shape.contour(ci, ankle + up * (2.00 * g) - cf2, ankle + up * (1.80 * g) + cf2,
		0.52 * g, Geom.shade(boot, 1.28))

# ---------------------------------------------------------------- bras

static func _draw_arm(ci: CanvasItem, f: Figure, st: Style, i: int, light: Vector2,
		far: bool) -> void:
	var g := f.girth
	var skin := recede(st.skin) if far else st.skin
	var sleeved: bool = st.vest != null or not st.bare_chest
	var sleeve_col: Color = st.vest if st.vest != null else st.cloth
	if far:
		sleeve_col = recede(sleeve_col)

	# Les deux épaules ne sont pas au même endroit : l'une est plus près du
	# spectateur, et le corps entre elles est épais. Décaler chaque bras selon la
	# profondeur est ce qui dégage le bras lointain du dos et pose le bras proche
	# devant la poitrine — sans cela une figure de profil est une planche à un
	# seul bras.
	var off := f.fwd * ((-1.40 if far else 0.55) * g)
	var sh := f.shoulder + off
	var el: Vector2 = f.elbow[i] + off
	var hd: Vector2 = f.hand[i] + off
	var uf := Shape.front_of(sh, el, f.facing)
	var ff := Shape.front_of(el, hd, f.facing)
	var upper := Shape.Limb.new(sh, el, uf, Shape.UPPER_ARM, g).with_steps(7)
	var fore := Shape.Limb.new(el, hd, ff, Shape.FOREARM, g).with_steps(7)

	# Le bras proche croise la poitrine, et sur un torse nu les deux sont de la
	# même couleur : le trait dessiné est la seule chose qui les sépare.
	if not far:
		var e := 0.30 * g
		var dk := Geom.shade(st.skin_dk, 0.42) if st.bare_chest else Geom.shade(st.cloth_dk, 0.36)
		upper.edge(ci, e, dk)
		fore.edge(ci, e, dk)

	upper.draw(ci, skin, light)
	fore.draw(ci, skin, light)
	Shape.contour(ci, el + uf * (0.75 * g), el + ff * (0.75 * g),
		0.16 * g, Geom.shade(st.skin_dk, 0.80), 0.25 if far else 0.5)

	if sleeved:
		var end := sh.lerp(el, 0.46)
		Shape.Limb.new(sh, end, uf, Shape.SLEEVE, g).with_steps(4).draw(ci, sleeve_col, light)
		Shape.contour(ci, end + uf * (1.45 * g), end - uf * (1.45 * g),
			0.24 * g, Geom.shade(sleeve_col, 0.66))
		Shape.contour(ci, end + uf * (1.00 * g), end - uf * (1.00 * g),
			0.22 * g, Geom.shade(skin, 0.60), 0.45)

	# --- main : une moufle avec un pouce, pas une boule -------------------
	var along := (hd - el).normalized()
	var hand := Shape.frame_xy(hd, along * g, ff * g, [
		-0.30, 0.74, 0.55, 0.86, 1.15, 0.60, 1.42, 0.10,
		1.30, -0.40, 0.55, -0.78, -0.30, -0.70,
	])
	Shape.cel_poly(ci, hand, skin, light, 0.42)
	Shape.contour(ci, hd + along * (0.72 * g) + ff * (0.68 * g),
		hd + along * (1.10 * g) - ff * (0.34 * g),
		0.13 * g, Geom.shade(st.skin_dk, 0.78), 0.3 if far else 0.6)

static func _draw_shield(ci: CanvasItem, f: Figure, st: Style, face: Color,
		light: Vector2) -> void:
	var g := f.girth
	var c: Vector2 = f.hand[1] - f.fwd * (1.4 * g)
	var ex := f.fwd * g
	var ey := -f.up * g
	# Vu presque de profil : un ovale étroit, une jante et un umbo.
	var disc := Shape.frame_xy(c, ex, ey, [
		0.0, -4.6, 2.4, -3.4, 3.1, 0.0, 2.4, 3.4,
		0.0, 4.6, -2.4, 3.4, -3.1, 0.0, -2.4, -3.4,
	])
	Shape.cel_poly(ci, disc, recede(face), light, 0.44)
	Shape.contour(ci, c + ey * -4.2, c + ey * 4.2, 0.34 * g,
		Geom.shade(recede(face), 1.35), 0.8)
	Shape.disc(ci, c, 1.5 * g, recede(st.metal))
	Shape.disc(ci, c - ey * 0.4, 0.7 * g, Geom.shade(recede(st.metal), 1.3))

# ---------------------------------------------------------------- torse

static func _draw_torso(ci: CanvasItem, f: Figure, st: Style, light: Vector2) -> void:
	var g := f.girth
	var sh := f.shoulder
	var hip := f.hip
	var front := Shape.front_of(sh, hip, f.facing)
	var prof := Shape.TORSO
	if st.robe > 0.7:
		prof = Shape.TORSO_ROBE
	elif f.heavy:
		prof = Shape.TORSO_HEAVY
	var body: Color = st.skin if st.bare_chest else st.cloth
	Shape.Limb.new(sh, hip, front, prof, f.chest).with_steps(10).with_shade(0.46).draw(ci, body, light)

	# Repère local pour les points de repère : +x sort par la poitrine, +y descend
	# la colonne, tous deux normalisés pour que les chiffres se lisent comme des
	# fractions du corps.
	var ex := front * f.chest
	var ey := (hip - sh).normalized() * f.tl
	var at := func(x: float, y: float) -> Vector2: return sh + ex * x + ey * y

	if st.bare_chest:
		# Pectoral : une plaque à bord inférieur franc, et rien d'autre. Un torse
		# nu de sept pixels de large a de la place pour exactement un repère —
		# sillon sternal et abdominaux à cette taille sont des rayures, pas de
		# l'anatomie, et ils coûtent au buste la valeur large et nette qui le fait
		# lire.
		var pec := Shape.frame_xy(sh, ex, ey, [
			-0.16, 0.04, 0.62, 0.06, 0.98, 0.18, 1.00, 0.34,
			0.66, 0.44, -0.10, 0.40, -0.42, 0.22,
		])
		Shape.cel_poly(ci, pec, Geom.shade(st.skin, 1.10), light, 0.30)
		Shape.contour(ci, at.call(0.96, 0.34), at.call(-0.06, 0.42),
			0.28 * g, Geom.shade(st.skin_dk, 0.62), 0.9)
		Shape.contour(ci, at.call(0.86, 0.62), at.call(-0.10, 0.66),
			0.20 * g, Geom.shade(st.skin_dk, 0.76), 0.42)
	else:
		Shape.contour(ci, at.call(0.90, 0.02), at.call(-0.86, 0.04),
			0.30 * g, Geom.shade(st.cloth, 1.16), 0.8)
		var collar := Shape.frame_xy(sh, ex, ey, [
			1.05, -0.02, 0.98, 0.22, 0.42, 0.10, -0.55, 0.00, -0.55, -0.06,
		])
		Shape.flat(ci, collar, Geom.shade(st.skin_dk, 0.86))
		Shape.contour(ci, at.call(1.02, 0.20), at.call(0.30, 0.02),
			0.20 * g, st.cloth_dk, 0.9)
		for xy in [[0.72, 0.42], [0.34, 0.48]]:
			Shape.contour(ci, at.call(xy[0], xy[1]), at.call(xy[0] - 0.2, xy[1] + 0.26),
				0.18 * g, st.cloth_dk, 0.34)

	# Gilet ouvert : couvre le dos et les flancs, laisse la poitrine nue. De
	# profil, cela se lit comme le bord avant du vêtement descendant le long du
	# corps.
	if st.vest != null:
		var v: Color = st.vest
		var coat := Shape.frame_xy(sh, ex, ey, [
			0.30, -0.05, 0.36, 0.30, 0.26, 0.72,
			-0.74, 0.80, -1.06, 0.34, -1.02, -0.04,
		])
		Shape.cel_poly(ci, coat, v, light, 0.46)
		Shape.contour(ci, at.call(0.32, -0.03), at.call(0.28, 0.74),
			0.24 * g, Geom.shade(v, 1.35), 0.95)
		Shape.contour(ci, at.call(0.30, 0.72), at.call(-0.72, 0.80),
			0.22 * g, Geom.shade(v, 0.62), 0.9)

static func _draw_tunic(ci: CanvasItem, f: Figure, st: Style, pose: Pose,
		light: Vector2) -> void:
	var g := f.girth
	var down := -f.up
	var side := f.fwd
	var hem_len := lerpf(0.0, 14.6, clampf(st.robe, 0.0, 1.0)) * f.unit
	var top := f.hip + f.up * (f.tl * 0.34)
	var w_top := f.chest * 0.78
	var w_bot := lerpf(2.6, 5.4, st.robe) * g
	# La jupe bat contre les jambes et se soulève du côté qui mène.
	var swing: float = (pose.leg[0][0] - pose.leg[1][0]) * 0.030
	var hem := f.hip + down * hem_len
	var off := side * (swing * g)
	var lift := absf(swing) * 0.09 * g
	var hp := func(t: float, drop: float) -> Vector2:
		return hem + side * lerpf(w_bot, -w_bot, t) + off + down * drop

	var pts := PackedVector2Array([
		top + side * w_top,
		f.hip + side * (w_top * 1.10),
		hp.call(0.0, -lift),
		hp.call(0.28, 0.45 * g),
		hp.call(0.56, -0.15 * g),
		hp.call(0.82, 0.36 * g),
		hp.call(1.0, -lift * 0.6),
		f.hip - side * (w_top * 1.10),
		top - side * w_top,
	])
	Shape.cel_poly(ci, pts, st.cloth, light, 0.44)

	# Trois plis suivant le drapé, chacun un coin à bord franc plutôt qu'un
	# frottis : l'étoffe se casse, elle ne s'estompe pas.
	for spec in [[0.24, 0.62], [0.50, 0.80], [0.76, 0.55]]:
		var t: float = spec[0]
		var w: float = spec[1]
		var a := f.hip + side * (lerpf(w_top, -w_top, t) * 0.8)
		var b: Vector2 = hp.call(t, 0.25 * g)
		Shape.flat(ci, PackedVector2Array([
			a + side * (w * 0.30 * g), b + side * (w * 0.75 * g),
			b - side * (w * 0.35 * g), a - side * (w * 0.20 * g),
		]), Color(st.cloth_dk.r, st.cloth_dk.g, st.cloth_dk.b, 0.34))
	for i in 6:
		var t0 := float(i) / 6.0
		var t1 := float(i + 1) / 6.0
		Shape.contour(ci, hp.call(t0, 0.45 * g * absf(sin(t0 * 3.6))),
			hp.call(t1, 0.45 * g * absf(sin(t1 * 3.6))),
			0.36 * g, Geom.shade(st.cloth_dk, 0.88), 0.8)

static func _draw_sash(ci: CanvasItem, f: Figure, st: Style, light: Vector2) -> void:
	var g := f.girth
	var waist := f.hip.lerp(f.shoulder, 0.20)
	var front := Shape.front_of(f.shoulder, f.hip, f.facing)
	var down := (f.hip - f.shoulder).normalized()
	var w := f.chest * 0.95
	# Enroulée deux fois : un bord haut, un bord bas, et un recouvrement visible.
	var band := Shape.frame_xy(waist, front * w, down * g, [
		0.10, -1.35, 1.08, -1.00, 1.16, 0.80,
		0.10, 1.15, -1.06, 0.85, -1.10, -1.05,
	])
	Shape.cel_poly(ci, band, st.sash, light, 0.42)
	Shape.contour(ci, waist + front * (w * 1.05) - down * (0.05 * g),
		waist - front * (w * 0.95) + down * (0.10 * g),
		0.14 * g, st.sash_dk, 0.75)
	var knot_at := waist + front * (w * 0.92) + down * (0.25 * g)
	var knot := Shape.frame_xy(knot_at, front * g, down * g, [
		0.25, -1.00, 1.05, -0.30, 0.90, 0.80,
		-0.15, 1.00, -0.80, 0.25, -0.50, -0.80,
	])
	Shape.cel_poly(ci, knot, Geom.shade(st.sash, 1.10), light, 0.38)

	if st.belt:
		var b := waist + down * (1.5 * g)
		Shape.contour(ci, b + front * w, b - front * w, 0.78 * g, Geom.shade(st.boot, 0.70))
		for k in [-1, 0, 1]:
			Shape.disc(ci, b + front * (k * 1.5 * g), 0.40 * g, Geom.shade(st.metal, 0.85), 8)

static func _draw_sash_tail(ci: CanvasItem, f: Figure, st: Style, pose: Pose) -> void:
	var g := f.girth
	var waist := f.hip.lerp(f.shoulder, 0.20)
	var back := -f.fwd
	var a := waist + back * (1.3 * g)
	var mid := a + back * (1.5 * g * pose.tail) - f.up * (2.4 * g)
	var end := mid + back * (1.0 * g * pose.tail) - f.up * (2.6 * g + pose.tail * 0.6 * g)
	Shape.capsule(ci, a, mid, 0.72 * g, 0.50 * g, st.sash_dk)
	Shape.capsule(ci, mid, end, 0.50 * g, 0.18 * g, Geom.shade(st.sash_dk, 0.85))

## Un long ruban qui flotte par-dessus l'épaule. L'Ombre en porte un, et c'est
## l'essentiel de ce qui la fait lire comme une apparition et non comme un
## recoloriage du prince.
static func _draw_scarf(ci: CanvasItem, f: Figure, st: Style, pose: Pose) -> void:
	if st.scarf == null:
		return
	var col: Color = Geom.shade(st.scarf, 1.25)
	var g := f.girth
	var back := -f.fwd
	var p := f.shoulder + back * (1.2 * g) + f.up * (0.4 * g)
	var wave: float = pose.leg[0][0] * 0.02
	for k in 4:
		var t := float(k)
		var q := p + back * ((3.0 + t * 0.6) * g) + f.up * ((0.9 + sin(t * 1.7 + wave) * 2.2) * g)
		Shape.capsule(ci, p, q, (1.5 - t * 0.3) * g, (1.2 - t * 0.28) * g,
			Geom.shade(col, 1.0 - t * 0.12))
		p = q

# ---------------------------------------------------------------- tête

static func _draw_head(ci: CanvasItem, f: Figure, st: Style, light: Vector2) -> void:
	var hc := f.head
	var hr := f.head_r
	var fw := f.facing
	# Repère local de toutes les formes de tête : +x devant, +y vers le bas, une
	# unité = un rayon de tête.
	var ex := Vector2(fw * hr, 0.0)
	var ey := Vector2(0.0, hr)
	var p := func(x: float, y: float) -> Vector2: return hc + ex * x + ey * y

	# Cou : de la mâchoire jusque bien sous la ligne d'épaule, pour que la tête
	# pose sur les épaules au lieu de tenir en équilibre sur une tige.
	var nb := f.shoulder - f.up * (1.6 * f.girth)
	var nt := hc - f.up * (0.35 * hr)
	Shape.Limb.new(nt, nb, Shape.front_of(nt, nb, fw), Shape.NECK, f.girth) \
		.with_steps(4).draw(ci, Geom.shade(st.skin, 0.84), light)

	Shape.cel_poly(ci, Shape.frame(hc, ex, ey, Shape.HEAD), st.skin, light, 0.44)

	# Oreille, rangée à l'arrière de la mâchoire.
	Shape.cel_poly(ci, Shape.frame_xy(hc, ex, ey, [
		-0.50, 0.02, -0.28, -0.02, -0.20, 0.26, -0.34, 0.46, -0.56, 0.38,
	]), Geom.shade(st.skin, 0.94), light, 0.50)
	Shape.contour(ci, p.call(-0.40, 0.10), p.call(-0.32, 0.32),
		hr * 0.07, Geom.shade(st.skin_dk, 0.7), 0.7)

	if st.head_wrap == null:
		_draw_hair(ci, f, st, light)
	else:
		_draw_wrap(ci, f, st, st.head_wrap, light)

	if st.band != null:
		var bc: Color = st.band
		Shape.cel_poly(ci, Shape.frame_xy(hc, ex, ey, [
			0.88, -0.50, 0.90, -0.22, -0.10, -0.34,
			-1.06, -0.40, -1.10, -0.66, -0.10, -0.60,
		]), bc, light, 0.42)
		Shape.cel_poly(ci, Shape.frame_xy(hc, ex, ey, [
			-0.94, -0.62, -1.52, -0.16, -1.44, 0.42,
			-1.24, 0.30, -1.30, -0.10, -0.90, -0.40,
		]), Geom.shade(bc, 0.84), light, 0.46)

	# ---- visage --------------------------------------------------------
	# Un visage, ce sont deux plans : celui de devant, qui prend la lumière, et
	# celui de côté, qui ne la prend pas. L'arête entre les deux court de la
	# tempe jusqu'au menton en passant par le coin de la bouche, et c'est de la
	# dessiner qui donne une structure à une tête d'une taille où aucun modelé ne
	# survivrait.
	Shape.flat(ci, Shape.frame_xy(hc, ex, ey, [
		0.34, -0.36, 0.54, 0.08, 0.48, 0.56, 0.16, 0.90,
		-0.44, 0.72, -0.86, 0.30, -0.94, -0.32,
	]), _a(Geom.shade(st.skin_dk, 0.96), 0.34))

	# L'orbite : un coin d'ombre sous l'arcade.
	Shape.flat(ci, Shape.frame_xy(hc, ex, ey, [
		0.30, -0.32, 0.82, -0.24, 0.84, 0.02, 0.56, 0.10, 0.32, -0.04,
	]), _a(Geom.shade(st.skin_dk, 0.70), 0.62))
	# Sourcil, couleur cheveux : à cette échelle, il fait plus pour la lecture que
	# n'importe quel modelé.
	Shape.contour(ci, p.call(0.30, -0.38), p.call(0.80, -0.28), hr * 0.10, st.hair, 0.95)
	# Œil : petit, sombre, avec un seul éclat. Un grand blanc d'œil à cette taille
	# donne un personnage aux yeux ronds de dessin animé, pas quelqu'un qui
	# regarde quelque chose.
	Shape.flat(ci, Shape.frame_xy(hc, ex, ey, [
		0.50, -0.08, 0.66, -0.15, 0.80, -0.05, 0.66, 0.05, 0.53, 0.01,
	]), Color8(228, 220, 206))
	Shape.flat(ci, Shape.frame_xy(hc, ex, ey, [
		0.60, -0.12, 0.75, -0.05, 0.72, 0.04, 0.59, 0.00,
	]), st.outline)
	# L'arête du nez accroche la lumière ; le dessous, non.
	Shape.contour(ci, p.call(0.84, -0.06), p.call(1.06, 0.22),
		hr * 0.11, Geom.shade(st.skin, 1.24), 0.7)
	Shape.flat(ci, Shape.frame_xy(hc, ex, ey, [
		0.78, 0.14, 1.08, 0.30, 0.90, 0.36, 0.78, 0.28,
	]), _a(Geom.shade(st.skin_dk, 0.74), 0.6))
	Shape.disc(ci, p.call(0.90, 0.31), hr * 0.065, Geom.shade(st.skin_dk, 0.50), 6)
	# Le sillon de l'aile du nez au coin de la bouche, la bouche, l'ombre sous la
	# lèvre inférieure.
	Shape.contour(ci, p.call(0.78, 0.36), p.call(0.70, 0.54),
		hr * 0.055, Geom.shade(st.skin_dk, 0.84), 0.30)
	Shape.contour(ci, p.call(0.64, 0.57), p.call(0.88, 0.55),
		hr * 0.07, Geom.shade(st.skin_dk, 0.44), 0.90)
	Shape.contour(ci, p.call(0.68, 0.71), p.call(0.86, 0.69),
		hr * 0.055, Geom.shade(st.skin_dk, 0.70), 0.45)
	# La mâchoire, et l'ombre que la tête jette sur le cou.
	Shape.contour(ci, p.call(-0.34, 0.78), p.call(0.62, 0.92),
		hr * 0.13, Geom.shade(st.skin_dk, 0.60), 0.55)

static func _a(c: Color, alpha: float) -> Color:
	return Color(c.r, c.g, c.b, alpha)

static func _draw_hair(ci: CanvasItem, f: Figure, st: Style, light: Vector2) -> void:
	var hc := f.head
	var hr := f.head_r
	var ex := Vector2(f.facing * hr, 0.0)
	var ey := Vector2(0.0, hr)
	# Une masse balayée : basse sur le front, débordant le sommet, retombant en
	# courte mèche à la nuque. Un seul polygone écrit, pour que la silhouette soit
	# une décision et non un accident.
	Shape.cel_poly(ci, Shape.frame(hc, ex, ey, Shape.HAIR), st.hair, light, 0.46)
	# Mèches suivant le balayage. Sans elles, la masse se lit comme un casque.
	for spec in [[0.58, -0.86, -0.28, -1.20, 0.13],
			[0.04, -1.10, -0.80, -0.96, 0.11],
			[-0.68, -0.84, -1.14, -0.20, 0.10]]:
		Shape.contour(ci, hc + ex * spec[0] + ey * spec[1],
			hc + ex * spec[2] + ey * spec[3],
			hr * spec[4], Geom.shade(st.hair, 1.34), 0.75)
	# Une mèche qui tombe sur la tempe, par-dessus le sourcil.
	Shape.cel_poly(ci, Shape.frame_xy(hc, ex, ey, [
		0.16, -1.02, 0.60, -0.86, 0.96, -0.48, 0.78, -0.40, 0.48, -0.70, 0.12, -0.86,
	]), Geom.shade(st.hair, 1.16), light, 0.50)

static func _draw_wrap(ci: CanvasItem, f: Figure, st: Style, wrap: Color,
		light: Vector2) -> void:
	var hc := f.head
	var hr := f.head_r
	var ex := Vector2(f.facing * hr, 0.0)
	var ey := Vector2(0.0, hr)
	var p := func(x: float, y: float) -> Vector2: return hc + ex * x + ey * y
	Shape.cel_poly(ci, Shape.frame_xy(hc, ex, ey, [
		0.94, -0.30, 0.86, -0.70, 0.48, -1.16, -0.10, -1.36,
		-0.70, -1.20, -1.06, -0.78, -1.18, -0.28, -1.08, 0.00,
	]), wrap, light, 0.44)
	# La bande enroulée sur le front, avec un recouvrement visible derrière.
	Shape.cel_poly(ci, Shape.frame_xy(hc, ex, ey, [
		0.98, -0.34, 0.94, -0.02, 0.10, 0.08,
		-0.88, -0.06, -1.14, -0.40, -0.92, -0.62, 0.20, -0.58,
	]), Geom.shade(wrap, 0.88), light, 0.42)
	Shape.contour(ci, p.call(0.90, -0.20), p.call(-0.90, -0.28),
		hr * 0.07, Geom.shade(wrap, 0.58), 0.7)
	# Le pan d'étoffe, rentré derrière l'oreille.
	Shape.cel_poly(ci, Shape.frame_xy(hc, ex, ey, [
		-0.86, -0.62, -1.20, -0.10, -1.14, 0.34, -0.92, 0.24, -0.96, -0.14, -0.72, -0.46,
	]), Geom.shade(wrap, 0.72), light, 0.46)

	# Un plumet signifie que c'est un casque : il lui faut donc un nasal.
	if st.plume != null:
		Shape.contour(ci, p.call(0.72, -0.32), p.call(0.82, 0.48),
			hr * 0.16, Geom.shade(st.metal, 0.9))
		Shape.cel_poly(ci, Shape.frame_xy(hc, ex, ey, [
			-0.05, -1.40, -0.55, -2.10, -1.35, -2.55,
			-1.15, -2.15, -0.60, -1.70, -0.35, -1.30,
		]), st.plume, light, 0.44)

# ---------------------------------------------------------------- armes

static func _draw_blade(ci: CanvasItem, f: Figure, st: Style, pose: Pose, blade: int) -> void:
	if blade == Blade.NONE:
		return
	var g := f.girth
	# On fait tourner la direction de l'avant-bras de l'angle de lame de la pose.
	var base_deg := rad_to_deg(atan2(f.hand_dir.x, f.hand_dir.y)) * f.facing
	var dd := _dir_down(base_deg + pose.sword)
	var dir := Vector2(dd.x * f.facing, dd.y)
	var per := dir.orthogonal()
	var hand: Vector2 = f.hand[0] + f.fwd * (0.55 * g)
	var length := 0.0
	var wide := 0.0
	var curve := 0.0
	var col := st.metal
	match blade:
		Blade.SWORD: length = 14.5; wide = 0.78
		Blade.SCIMITAR: length = 15.5; wide = 1.05; curve = 0.20; col = Color8(226, 216, 180)
		Blade.DAGGER: length = 6.4; wide = 0.70
		Blade.WAND: length = 9.0; wide = 1.0; col = Color8(104, 72, 44)
	length *= f.unit
	var grip_a := hand - dir * (2.6 * g)
	var tip := hand + dir * length + per * (curve * length)
	var mid := hand + dir * (length * 0.5) + per * (curve * length * 0.34)

	Shape.capsule(ci, grip_a, hand + dir * (0.6 * g), 0.70 * g, 0.76 * g, Color8(96, 62, 36))
	Shape.disc(ci, grip_a - dir * (0.35 * g), 0.80 * g, Color8(186, 152, 78), 8)
	if blade == Blade.WAND:
		Shape.capsule(ci, hand, tip, 0.80 * g, 0.60 * g, col)
		Shape.disc(ci, tip, 1.5 * g, Color8(255, 178, 64))
		return
	# Garde et quillons.
	var cg := per * (1.75 * g)
	Shape.capsule(ci, hand + cg + dir * (0.5 * g), hand - cg + dir * (0.5 * g),
		0.42 * g, 0.42 * g, Color8(190, 154, 78))
	# Lame : un losange effilé, gouttière claire et dos sombre — deux valeurs
	# franches, ce qui fait lire l'acier comme de l'acier.
	var root := hand + dir * (1.0 * g)
	Shape.flat(ci, PackedVector2Array([
		root + per * (wide * g), mid + per * (wide * 0.82 * g), tip,
		mid - per * (wide * 0.82 * g), root - per * (wide * g),
	]), Geom.shade(col, 0.62))
	Shape.flat(ci, PackedVector2Array([
		root + per * (wide * g), mid + per * (wide * 0.82 * g), tip,
		mid + per * (wide * 0.05 * g), root + per * (wide * 0.10 * g),
	]), Geom.shade(col, 1.18))
	Shape.contour(ci, root + per * (wide * 0.45 * g), mid.lerp(tip, 0.55),
		0.24 * g, Color.WHITE, 0.7)

# ---------------------------------------------------------------- squelettes

static func _draw_bones(ci: CanvasItem, f: Figure, st: Style, pose: Pose,
		blade: int, light: Vector2) -> void:
	var g := f.girth
	var bone := Color8(232, 226, 204)
	for spec in [[1, true], [0, false]]:
		var i: int = spec[0]
		var far: bool = spec[1]
		var c: Color = recede(bone) if far else bone
		var seg := func(a: Vector2, b: Vector2, k: float, n: int) -> void:
			Shape.Limb.new(a, b, Shape.front_of(a, b, f.facing), Shape.BONE, g * k) \
				.with_steps(n).draw(ci, c, light)
		seg.call(f.hip, f.knee[i], 1.05, 6)
		seg.call(f.knee[i], f.ankle[i], 0.90, 6)
		seg.call(f.ankle[i], f.toe[i], 0.80, 4)
		seg.call(f.shoulder, f.elbow[i], 0.90, 6)
		seg.call(f.elbow[i], f.hand[i], 0.78, 6)
		var dir: Vector2 = (f.hand[i] - f.elbow[i]).normalized()
		for k in [-1, 0, 1]:
			var o: Vector2 = dir.orthogonal() * (float(k) * 0.7 * g)
			Shape.capsule(ci, f.hand[i] + o, f.hand[i] + o + dir * (1.5 * g),
				0.34 * g, 0.24 * g, Geom.shade(c, 0.95))

	var front := Shape.front_of(f.shoulder, f.hip, f.facing)
	Shape.Limb.new(f.shoulder, f.hip, front, Shape.BONE, g).with_steps(6) \
		.draw(ci, Geom.shade(bone, 0.80), light)
	var ex := front * g
	var ey := (f.hip - f.shoulder).normalized() * g
	Shape.cel_poly(ci, Shape.frame_xy(f.hip, ex, ey, [
		2.6, -1.2, 2.9, 0.6, 1.2, 1.9, -1.2, 1.9, -2.7, 0.5, -2.4, -1.2,
	]), bone, light, 0.44)
	Shape.flat(ci, Shape.frame_xy(f.hip, ex, ey, [
		1.3, -0.2, 1.4, 0.9, -1.3, 0.9, -1.4, -0.2,
	]), _a(Color8(38, 30, 34), 0.8))
	var down := -f.up
	for k in 4:
		var t := 0.18 + k * 0.16
		var c2 := f.shoulder + down * (f.tl * t)
		var w := (2.85 - k * 0.22) * g
		Shape.contour(ci, c2 + front * w, c2 - front * (w * 0.75), 0.52 * g, bone)
	Shape.contour(ci, f.shoulder + front * (2.4 * g), f.shoulder - front * (1.8 * g),
		0.58 * g, Geom.shade(bone, 1.05))

	# Crâne : boîte, orbite, épine nasale, mâchoire tombée et ses dents.
	var hc := f.head
	var hr := f.head_r
	var hx := Vector2(f.facing * hr, 0.0)
	var hy := Vector2(0.0, hr)
	var p := func(x: float, y: float) -> Vector2: return hc + hx * x + hy * y
	Shape.cel_poly(ci, Shape.frame(hc, hx, hy, Shape.SKULL), bone, light, 0.44)
	Shape.flat(ci, Shape.frame_xy(hc, hx, hy, [
		0.28, -0.20, 0.74, -0.18, 0.76, 0.16, 0.36, 0.18,
	]), Color8(22, 18, 22))
	Shape.flat(ci, Shape.frame_xy(hc, hx, hy, [0.78, 0.16, 0.94, 0.30, 0.80, 0.40]),
		_a(Color8(22, 18, 22), 0.9))
	Shape.contour(ci, p.call(0.20, 0.60), p.call(0.88, 0.56), hr * 0.09,
		Color8(30, 24, 26), 0.85)
	for k in 4:
		var t := float(k) / 3.0
		Shape.disc(ci, p.call(lerpf(0.24, 0.84, t), lerpf(0.62, 0.54, t)),
			hr * 0.07, Color8(30, 24, 26), 6)
	_draw_blade(ci, f, st, pose, blade)
