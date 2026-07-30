# Palettes de niveau. Chacune reteinte tout le décor : briques, mortier, dalles,
# mur du fond, ferronnerie, et la lumière ambiante de la salle.

class_name Themes

static func _c(r: int, g: int, b: int) -> Color:
	return Color8(r, g, b)

static func make(
	name: String,
	brick: Color, brick_dk: Color, mortar: Color,
	slab_top: Color, slab_face: Color, slab_dk: Color,
	back: Color, back_dk: Color,
	accent: Color, metal: Color,
	ambient: Color, torch: Color, vignette: Color
) -> Dictionary:
	return {
		"name": name,
		"brick": brick, "brick_dk": brick_dk, "mortar": mortar,
		"slab_top": slab_top, "slab_face": slab_face, "slab_dk": slab_dk,
		"back": back, "back_dk": back_dk,
		"accent": accent, "metal": metal,
		"ambient": ambient, "torch": torch, "vignette": vignette,
	}

static func by_name(n: String) -> Dictionary:
	match n:
		"cistern": return CISTERN()
		"palace": return PALACE()
		"tower": return TOWER()
		"garden": return GARDEN()
		"sanctum": return SANCTUM()
		_: return DUNGEON()

static func DUNGEON() -> Dictionary:
	return make("dungeon",
		_c(96, 92, 104), _c(58, 56, 70), _c(34, 33, 42),
		_c(140, 134, 140), _c(88, 84, 94), _c(40, 38, 48),
		_c(66, 62, 82), _c(32, 30, 43),
		_c(120, 104, 72), _c(158, 162, 174),
		Color(0.60, 0.58, 0.70), _c(255, 168, 78), _c(8, 8, 16))

static func CISTERN() -> Dictionary:
	return make("cistern",
		_c(76, 100, 104), _c(42, 62, 68), _c(24, 38, 44),
		_c(122, 148, 148), _c(70, 94, 98), _c(30, 46, 52),
		_c(44, 68, 78), _c(22, 34, 44),
		_c(102, 132, 118), _c(150, 168, 172),
		Color(0.52, 0.62, 0.70), _c(255, 176, 96), _c(6, 12, 18))

static func PALACE() -> Dictionary:
	return make("palace",
		_c(178, 148, 106), _c(128, 102, 70), _c(92, 72, 50),
		_c(226, 200, 156), _c(170, 140, 100), _c(96, 74, 52),
		_c(112, 88, 68), _c(62, 48, 38),
		_c(226, 184, 84), _c(206, 200, 190),
		Color(0.66, 0.62, 0.54), _c(255, 190, 110), _c(22, 14, 10))

static func TOWER() -> Dictionary:
	return make("tower",
		_c(112, 96, 118), _c(64, 54, 76), _c(38, 30, 48),
		_c(166, 148, 166), _c(104, 88, 112), _c(46, 36, 56),
		_c(64, 50, 84), _c(32, 24, 46),
		_c(150, 122, 190), _c(176, 168, 196),
		Color(0.55, 0.49, 0.72), _c(190, 150, 255), _c(10, 6, 20))

static func GARDEN() -> Dictionary:
	return make("garden",
		_c(158, 152, 116), _c(104, 104, 76), _c(72, 76, 56),
		_c(212, 208, 168), _c(150, 148, 112), _c(80, 84, 60),
		_c(92, 116, 110), _c(46, 64, 66),
		_c(120, 176, 116), _c(196, 198, 186),
		Color(0.70, 0.70, 0.60), _c(255, 196, 128), _c(14, 20, 16))

static func SANCTUM() -> Dictionary:
	return make("sanctum",
		_c(84, 62, 78), _c(48, 32, 46), _c(28, 18, 28),
		_c(148, 116, 132), _c(88, 64, 80), _c(38, 24, 36),
		_c(52, 32, 52), _c(26, 15, 28),
		_c(214, 168, 88), _c(182, 172, 190),
		Color(0.50, 0.41, 0.60), _c(255, 132, 96), _c(12, 4, 12))
