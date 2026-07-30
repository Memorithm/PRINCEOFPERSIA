# Le décor : maçonnerie, dalles, herses, pièges, torches, ornements.
#
# Tout est dessiné en polygones dans les unités du monde. Chaque tuile tire ses
# variations d'un hachage stable de ses coordonnées, si bien qu'une salle a
# toujours le même appareil de briques d'une partie à l'autre sans qu'on ait à
# stocker quoi que ce soit.
#
# L'ordre compte : le mur du fond, puis la maçonnerie, puis les dalles (celles du
# rang du dessus servant de corniche au rang du dessous), puis ce qui est posé
# dessus.

class_name TileArt

const TW := Geom.TILE_W
const TH := Geom.TILE_H
const FH := Geom.FLOOR_H

## Position de la flamme d'une torche.
static func torch_flame_pos(tx: int, ty: int) -> Vector2:
	return Vector2(Geom.cx(tx), float(ty) * TH + TH * 0.34)

static func draw_environment(ci: CanvasItem, lv: Level, dy, view: Rect2, t: float) -> void:
	var th_c: Dictionary = lv.theme
	var tx0 := Geom.tx_of(view.position.x) - 1
	var tx1 := Geom.tx_of(view.end.x) + 2
	var ty0 := Geom.ty_of(view.position.y) - 1
	var ty1 := Geom.ty_of(view.end.y) + 2

	# --- 1. fonds de salle ---------------------------------------------
	for ty in range(ty0, ty1):
		for tx in range(tx0, tx1):
			var tile := lv.tile(tx, ty)
			if tile == Level.T.WALL:
				continue
			_back_panel(ci, th_c, tx, ty)

	# --- 2. maçonnerie --------------------------------------------------
	for ty in range(ty0, ty1):
		for tx in range(tx0, tx1):
			match lv.tile(tx, ty):
				Level.T.WALL:
					_bricks(ci, th_c, tx, ty)
				Level.T.PILLAR:
					_pillar(ci, th_c, tx, ty)

	# --- 3. dalles de sol ------------------------------------------------
	for ty in range(ty0, ty1):
		for tx in range(tx0, tx1):
			var tile := lv.tile(tx, ty)
			var below_solid := Level.tile_solid(lv.tile(tx, ty + 1))
			if not (Level.tile_walkable(tile) or below_solid):
				continue
			if tile == Level.T.LOOSE:
				_loose(ci, th_c, tx, ty, dy.a(tx, ty))
			else:
				_slab(ci, th_c, tx, ty, tile == Level.T.RUBBLE)

	# --- 4. mobilier et pièges -------------------------------------------
	for ty in range(ty0, ty1):
		for tx in range(tx0, tx1):
			var tile := lv.tile(tx, ty)
			var a: float = dy.a(tx, ty)
			match tile:
				Level.T.TORCH: _torch(ci, th_c, tx, ty)
				Level.T.MIRROR: _mirror(ci, th_c, tx, ty)
				Level.T.WINDOW: _window(ci, th_c, tx, ty)
				Level.T.ARCH: _arch(ci, th_c, tx, ty)
				Level.T.BONES: _bones(ci, th_c, tx, ty)
				Level.T.SPIKES: _spikes(ci, th_c, tx, ty, a, dy.has(tx, ty, 16))
				Level.T.CHOMPER: _chomper(ci, th_c, tx, ty, a)
				Level.T.GATE: _gate(ci, th_c, tx, ty, a)
				Level.T.EXIT: _exit(ci, th_c, tx, ty, a)
				Level.T.PLATE_RAISE, Level.T.PLATE_DROP:
					_plate(ci, th_c, tx, ty, a, tile == Level.T.PLATE_RAISE)

# ---------------------------------------------------------------- fonds

static func _back_panel(ci: CanvasItem, th_c: Dictionary, tx: int, ty: int) -> void:
	var x := float(tx) * TW
	var y := float(ty) * TH
	var back: Color = th_c["back"]
	var back_dk: Color = th_c["back_dk"]
	ci.draw_rect(Rect2(x, y, TW, TH), back_dk)
	# Une arcade aveugle par case : c'est ce qui empêche le fond d'être un aplat.
	var m := 3.0
	var top := y + 5.0
	var w := TW - m * 2.0
	var pts := PackedVector2Array([
		Vector2(x + m, y + TH),
		Vector2(x + m, top + 7.0),
		Vector2(x + m + w * 0.22, top),
		Vector2(x + m + w * 0.78, top),
		Vector2(x + TW - m, top + 7.0),
		Vector2(x + TW - m, y + TH),
	])
	Shape.poly(ci, pts, back)
	# Un liseré clair au sommet de l'arc et une ombre au pied.
	Shape.poly(ci, PackedVector2Array([
		Vector2(x + m, top + 7.0),
		Vector2(x + m + w * 0.22, top),
		Vector2(x + m + w * 0.78, top),
		Vector2(x + TW - m, top + 7.0),
		Vector2(x + TW - m, top + 9.0),
		Vector2(x + m + w * 0.76, top + 2.2),
		Vector2(x + m + w * 0.24, top + 2.2),
		Vector2(x + m, top + 9.0),
	]), Geom.shade(back, 1.22))
	ci.draw_rect(Rect2(x + m, y + TH - 6.0, w, 6.0), Geom.shade(back, 0.72))

# ---------------------------------------------------------------- maçonnerie

static func _bricks(ci: CanvasItem, th_c: Dictionary, tx: int, ty: int) -> void:
	var x := float(tx) * TW
	var y := float(ty) * TH
	ci.draw_rect(Rect2(x, y, TW, TH), th_c["mortar"])
	var brick: Color = th_c["brick"]
	var brick_dk: Color = th_c["brick_dk"]
	var rows := 4
	var bh := TH / rows
	for r in rows:
		var by := y + r * bh
		# Une assise sur deux est décalée d'une demi-brique.
		var off := 0.0 if (ty * rows + r) % 2 == 0 else -TW * 0.5
		var bx := x + off
		while bx < x + TW:
			var w := TW * 0.5
			var x0 := maxf(bx, x)
			var x1 := minf(bx + w, x + TW)
			if x1 - x0 > 0.6:
				var k := Geom.hashf(int(bx * 0.5), ty * rows + r, 11)
				var c := brick_dk.lerp(brick, 0.35 + k * 0.65)
				ci.draw_rect(Rect2(x0 + 0.5, by + 0.5, x1 - x0 - 1.0, bh - 1.0), c)
				# Chanfrein éclairé en haut, ombre en bas : c'est ce qui donne du
				# relief à une pierre à cette taille.
				ci.draw_rect(Rect2(x0 + 0.5, by + 0.5, x1 - x0 - 1.0, 1.0), Geom.shade(c, 1.28))
				ci.draw_rect(Rect2(x0 + 0.5, by + bh - 1.5, x1 - x0 - 1.0, 1.0), Geom.shade(c, 0.66))
			bx += w

static func _pillar(ci: CanvasItem, th_c: Dictionary, tx: int, ty: int) -> void:
	var x := Geom.cx(tx)
	var y := float(ty) * TH
	var brick: Color = th_c["brick"]
	var w := TW * 0.34
	ci.draw_rect(Rect2(x - w, y, w * 2.0, TH), Geom.shade(brick, 0.78))
	ci.draw_rect(Rect2(x - w * 0.55, y, w * 0.7, TH), Geom.shade(brick, 1.14))
	ci.draw_rect(Rect2(x + w * 0.35, y, w * 0.5, TH), Geom.shade(brick, 0.56))
	# Chapiteau et base.
	for yy in [y + 1.0, y + TH - 5.0]:
		ci.draw_rect(Rect2(x - w * 1.25, yy, w * 2.5, 4.0), Geom.shade(brick, 1.05))
		ci.draw_rect(Rect2(x - w * 1.25, yy + 3.0, w * 2.5, 1.2), Geom.shade(brick, 0.6))

# ---------------------------------------------------------------- sols

static func _slab(ci: CanvasItem, th_c: Dictionary, tx: int, ty: int, rubble: bool) -> void:
	var x := float(tx) * TW
	var surf := Geom.surf(ty)
	var top: Color = th_c["slab_top"]
	var face: Color = th_c["slab_face"]
	var dk: Color = th_c["slab_dk"]
	# Face avant de la dalle, sous la surface d'appui.
	ci.draw_rect(Rect2(x, surf, TW, FH), face)
	# Bandeau supérieur : la tranche vue de trois quarts.
	ci.draw_rect(Rect2(x, surf, TW, 2.2), top)
	ci.draw_rect(Rect2(x, surf + FH - 2.0, TW, 2.0), dk)
	# Joints verticaux, décalés par tuile.
	var k := Geom.hashf(tx, ty, 5)
	for i in 3:
		var jx := x + TW * (0.18 + 0.32 * i + k * 0.06)
		ci.draw_rect(Rect2(jx, surf + 2.2, 0.9, FH - 4.0), Geom.shade(face, 0.72))
	if rubble:
		for i in 5:
			var rx := x + TW * (0.1 + 0.19 * i + Geom.hashf(tx, ty, i) * 0.06)
			var s := 1.4 + Geom.hashf(tx, ty, i + 9) * 2.2
			ci.draw_rect(Rect2(rx, surf - s, s * 1.6, s), Geom.shade(face, 0.85))

static func _loose(ci: CanvasItem, th_c: Dictionary, tx: int, ty: int, wobble: float) -> void:
	var x := float(tx) * TW
	var surf := Geom.surf(ty)
	var face: Color = th_c["slab_face"]
	# Elle s'affaisse et tremble à mesure que la mèche brûle.
	var sag := wobble * 2.6
	var tilt := sin(wobble * 34.0) * wobble * 1.6
	var pts := PackedVector2Array([
		Vector2(x, surf + tilt),
		Vector2(x + TW, surf - tilt + sag),
		Vector2(x + TW, surf - tilt + sag + FH),
		Vector2(x, surf + tilt + FH),
	])
	Shape.poly(ci, pts, Geom.shade(face, 0.92))
	Shape.poly(ci, PackedVector2Array([
		pts[0], pts[1], pts[1] + Vector2(0, 2.0), pts[0] + Vector2(0, 2.0),
	]), Geom.shade(th_c["slab_top"], 0.9))
	# Fente entre deux planches : le signe qu'il ne faut pas s'y attarder.
	ci.draw_rect(Rect2(x + TW * 0.46, surf + 1.0, 1.2, FH - 2.0), Geom.shade(face, 0.42))

# ---------------------------------------------------------------- pièges

static func _spikes(ci: CanvasItem, th_c: Dictionary, tx: int, ty: int, a: float,
		bloody: bool) -> void:
	if a <= 0.01:
		return
	var x := float(tx) * TW
	var surf := Geom.surf(ty)
	var metal: Color = th_c["metal"]
	var h := 13.0 * a
	for i in 4:
		var sx := x + TW * (0.16 + 0.23 * i)
		var w := 2.4
		var pts := PackedVector2Array([
			Vector2(sx - w, surf + 1.0),
			Vector2(sx, surf + 1.0 - h),
			Vector2(sx + w, surf + 1.0),
		])
		Shape.poly(ci, pts, metal)
		Shape.poly(ci, PackedVector2Array([
			Vector2(sx - w * 0.35, surf + 1.0),
			Vector2(sx, surf + 1.0 - h),
			Vector2(sx + w * 0.2, surf + 1.0),
		]), Geom.shade(metal, 1.3))
		if bloody:
			Shape.poly(ci, PackedVector2Array([
				Vector2(sx - w * 0.5, surf + 1.0 - h * 0.65),
				Vector2(sx, surf + 1.0 - h),
				Vector2(sx + w * 0.5, surf + 1.0 - h * 0.65),
			]), Color8(150, 22, 26))

static func _chomper(ci: CanvasItem, th_c: Dictionary, tx: int, ty: int, a: float) -> void:
	var x := float(tx) * TW
	var y := float(ty) * TH
	var metal: Color = th_c["metal"]
	var accent: Color = th_c["accent"]
	# Montants.
	ci.draw_rect(Rect2(x + 1.0, y, 3.5, TH), Geom.shade(accent, 0.8))
	ci.draw_rect(Rect2(x + TW - 4.5, y, 3.5, TH), Geom.shade(accent, 0.8))
	# Deux jeux de lames qui se rejoignent au centre.
	var reach := (TW * 0.5 - 5.0) * a
	for side: float in [-1.0, 1.0]:
		var base: float = x + TW * 0.5 - side * (TW * 0.5 - 4.5)
		for i in 3:
			var by := y + TH * (0.18 + 0.3 * i)
			var pts := PackedVector2Array([
				Vector2(base, by - 3.0),
				Vector2(base + side * reach, by - 0.8),
				Vector2(base + side * reach, by + 0.8),
				Vector2(base, by + 3.0),
			])
			Shape.poly(ci, pts, metal)
			Shape.poly(ci, PackedVector2Array([
				pts[0], pts[1], pts[1] + Vector2(0, 0.7), pts[0] + Vector2(0, 1.2),
			]), Geom.shade(metal, 1.35))

static func _gate(ci: CanvasItem, th_c: Dictionary, tx: int, ty: int, a: float) -> void:
	var x := float(tx) * TW
	var y := float(ty) * TH
	var metal: Color = th_c["metal"]
	# La herse remonte dans son logement : on ne dessine que la part visible.
	var drop := TH * (1.0 - a)
	if drop <= 0.5:
		_gate_housing(ci, th_c, tx, ty)
		return
	var top := y
	for i in 5:
		var bx := x + 2.5 + i * (TW - 5.0) / 4.0
		ci.draw_rect(Rect2(bx - 1.3, top, 2.6, drop), Geom.shade(metal, 0.86))
		ci.draw_rect(Rect2(bx - 1.3, top, 0.9, drop), Geom.shade(metal, 1.25))
	for r in 3:
		var by := top + drop * (0.18 + 0.34 * r)
		if by < top + drop - 1.0:
			ci.draw_rect(Rect2(x + 1.5, by, TW - 3.0, 2.0), Geom.shade(metal, 0.72))
	# Pointes en bas de la herse.
	for i in 5:
		var bx := x + 2.5 + i * (TW - 5.0) / 4.0
		Shape.poly(ci, PackedVector2Array([
			Vector2(bx - 1.6, top + drop - 1.0),
			Vector2(bx + 1.6, top + drop - 1.0),
			Vector2(bx, top + drop + 2.6),
		]), Geom.shade(metal, 1.05))
	_gate_housing(ci, th_c, tx, ty)

static func _gate_housing(ci: CanvasItem, th_c: Dictionary, tx: int, ty: int) -> void:
	var x := float(tx) * TW
	var y := float(ty) * TH
	var accent: Color = th_c["accent"]
	ci.draw_rect(Rect2(x, y - 1.0, TW, 4.0), Geom.shade(accent, 0.7))
	ci.draw_rect(Rect2(x, y - 1.0, TW, 1.2), Geom.shade(accent, 1.2))

static func _exit(ci: CanvasItem, th_c: Dictionary, tx: int, ty: int, a: float) -> void:
	var x := float(tx) * TW
	var y := float(ty) * TH
	var accent: Color = th_c["accent"]
	# Un chambranle sculpté, et la porte qui coulisse dans le linteau.
	ci.draw_rect(Rect2(x + 1.0, y + 2.0, TW - 2.0, TH - 2.0), Color8(16, 12, 20))
	ci.draw_rect(Rect2(x, y, TW, 3.5), Geom.shade(accent, 1.1))
	ci.draw_rect(Rect2(x, y + 2.0, 3.0, TH - 2.0), Geom.shade(accent, 0.85))
	ci.draw_rect(Rect2(x + TW - 3.0, y + 2.0, 3.0, TH - 2.0), Geom.shade(accent, 0.7))
	var drop := (TH - 4.0) * (1.0 - a)
	if drop > 0.5:
		ci.draw_rect(Rect2(x + 3.0, y + 3.0, TW - 6.0, drop), Geom.shade(accent, 0.62))
		for i in 3:
			ci.draw_rect(Rect2(x + 3.0, y + 3.0 + drop * (0.25 + 0.3 * i), TW - 6.0, 1.4),
				Geom.shade(accent, 0.42))
		ci.draw_rect(Rect2(x + 3.0, y + 3.0 + drop - 1.6, TW - 6.0, 1.6),
			Geom.shade(accent, 1.2))

static func _plate(ci: CanvasItem, th_c: Dictionary, tx: int, ty: int, a: float,
		raise_kind: bool) -> void:
	var x := float(tx) * TW
	var surf := Geom.surf(ty)
	var accent: Color = th_c["accent"]
	var down := a * 1.8
	var col: Color = accent if raise_kind else Geom.shade(accent, 0.6)
	ci.draw_rect(Rect2(x + 4.0, surf - 2.2 + down, TW - 8.0, 2.6), Geom.shade(col, 0.9))
	ci.draw_rect(Rect2(x + 4.0, surf - 2.2 + down, TW - 8.0, 0.9), Geom.shade(col, 1.35))
	# La rainure dans laquelle la dalle s'enfonce.
	ci.draw_rect(Rect2(x + 3.0, surf + 0.4, TW - 6.0, 1.0), Geom.shade(col, 0.4))

# ---------------------------------------------------------------- ornements

static func _torch(ci: CanvasItem, th_c: Dictionary, tx: int, ty: int) -> void:
	var p := torch_flame_pos(tx, ty)
	var metal: Color = th_c["metal"]
	# Applique murale : une console et une coupe. La flamme elle-même est faite
	# de particules et d'une lumière, pas d'un dessin.
	Shape.poly(ci, PackedVector2Array([
		Vector2(p.x - 1.2, p.y + 9.0), Vector2(p.x + 1.2, p.y + 9.0),
		Vector2(p.x + 1.2, p.y + 2.5), Vector2(p.x - 1.2, p.y + 2.5),
	]), Geom.shade(metal, 0.6))
	Shape.poly(ci, PackedVector2Array([
		Vector2(p.x - 3.4, p.y + 1.0), Vector2(p.x + 3.4, p.y + 1.0),
		Vector2(p.x + 2.2, p.y + 4.0), Vector2(p.x - 2.2, p.y + 4.0),
	]), Geom.shade(metal, 0.9))
	ci.draw_rect(Rect2(p.x - 3.6, p.y + 0.2, 7.2, 1.2), Geom.shade(metal, 1.3))

static func _mirror(ci: CanvasItem, th_c: Dictionary, tx: int, ty: int) -> void:
	var x := float(tx) * TW
	var y := float(ty) * TH
	var accent: Color = th_c["accent"]
	var m := 4.0
	ci.draw_rect(Rect2(x + m - 2.0, y + m - 2.0, TW - m * 2.0 + 4.0, TH - m * 2.0 + 4.0),
		Geom.shade(accent, 0.9))
	ci.draw_rect(Rect2(x + m, y + m, TW - m * 2.0, TH - m * 2.0), Color8(58, 74, 96))
	# Reflet : deux bandes obliques, ce qui suffit à dire « verre ».
	Shape.poly(ci, PackedVector2Array([
		Vector2(x + m + 2.0, y + TH - m), Vector2(x + m + 9.0, y + TH - m),
		Vector2(x + TW - m, y + m + 3.0), Vector2(x + TW - m - 7.0, y + m + 3.0),
	]), Color8(112, 138, 164))

static func _window(ci: CanvasItem, th_c: Dictionary, tx: int, ty: int) -> void:
	var x := float(tx) * TW
	var y := float(ty) * TH
	var m := 6.0
	ci.draw_rect(Rect2(x + m, y + 4.0, TW - m * 2.0, TH * 0.5), Color8(20, 24, 36))
	var metal: Color = th_c["metal"]
	for i in 3:
		ci.draw_rect(Rect2(x + m + 2.0 + i * (TW - m * 2.0 - 4.0) / 2.0, y + 4.0,
			1.6, TH * 0.5), Geom.shade(metal, 0.7))

static func _arch(ci: CanvasItem, th_c: Dictionary, tx: int, ty: int) -> void:
	var x := float(tx) * TW
	var y := float(ty) * TH
	var brick: Color = th_c["brick"]
	# Claveaux : un arc de six pierres.
	for i in 6:
		var f := float(i) / 5.0
		var ang := PI * (0.12 + 0.76 * f)
		var cx := x + TW * 0.5
		var r := TW * 0.52
		var px := cx - cos(ang) * r
		var py := y + 10.0 - sin(ang) * 9.0
		var c := Geom.shade(brick, 0.8 + Geom.hashf(tx, ty, i) * 0.4)
		Shape.poly(ci, PackedVector2Array([
			Vector2(px - 2.6, py), Vector2(px + 2.6, py),
			Vector2(px + 2.2, py + 5.5), Vector2(px - 2.2, py + 5.5),
		]), c)

static func _bones(ci: CanvasItem, th_c: Dictionary, tx: int, ty: int) -> void:
	var x := Geom.cx(tx)
	var surf := Geom.surf(ty)
	var bone := Color8(206, 200, 178)
	Shape.capsule(ci, Vector2(x - 8.0, surf - 1.5), Vector2(x + 5.0, surf - 2.5), 1.1, 0.9, bone)
	Shape.capsule(ci, Vector2(x - 4.0, surf - 1.0), Vector2(x + 8.0, surf - 3.0), 0.9, 0.8,
		Geom.shade(bone, 0.85))
	Shape.disc(ci, Vector2(x + 8.5, surf - 3.5), 2.6, bone, 10)
	Shape.disc(ci, Vector2(x + 9.4, surf - 3.8), 0.8, Color8(30, 24, 26), 6)

## Sources lumineuses de la portion visible : torches, fenêtres, sortie ouverte.
## Renvoie [[position, rayon, couleur, intensité], ...].
static func collect_lights(lv: Level, dy, view: Rect2, t: float) -> Array:
	var out: Array = []
	var tx0 := Geom.tx_of(view.position.x) - 1
	var tx1 := Geom.tx_of(view.end.x) + 2
	var ty0 := Geom.ty_of(view.position.y) - 1
	var ty1 := Geom.ty_of(view.end.y) + 2
	var torch: Color = lv.theme["torch"]
	for ty in range(ty0, ty1):
		for tx in range(tx0, tx1):
			match lv.tile(tx, ty):
				Level.T.TORCH:
					var flick := 0.86 + Geom.noise1(t * 6.0 + tx * 3.1 + ty * 1.7, 3) * 0.18
					out.append([torch_flame_pos(tx, ty) + Vector2(0, -3.0), 150.0, torch, flick * 1.25])
				Level.T.WINDOW:
					out.append([Vector2(Geom.cx(tx), float(ty) * TH + TH * 0.4), 92.0,
						Color8(180, 200, 236), 0.55])
				Level.T.EXIT:
					if dy.a(tx, ty) > 0.3:
						out.append([Vector2(Geom.cx(tx), float(ty) * TH + TH * 0.5), 80.0,
							Color8(255, 226, 170), 0.7 * dy.a(tx, ty)])
	return out
