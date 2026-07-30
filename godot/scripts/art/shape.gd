# Dessiner les personnages comme des *formes* plutôt que comme des empilements
# de capsules.
#
# Un membre construit à partir d'une capsule est un tube, et un corps fait de
# tubes se lit comme un ballon de baudruche, aussi soigneusement soit-il ombré.
# Deux choses font qu'une figure se lit comme dessinée :
#
# 1. **Une silhouette écrite.** Un vrai bras n'est pas un cône : il enfle au
#    deltoïde, encore au biceps, se pince au coude, enfle à l'avant-bras et
#    s'affine au poignet — et il le fait de façon *asymétrique*, parce que le
#    biceps est devant et le triceps derrière. Chaque os porte donc un `Profile` :
#    une liste de (position le long de l'os, demi-largeur avant, demi-largeur
#    arrière). C'est ce profil que l'œil lit réellement.
#
# 2. **Un ombrage à bords francs.** Un dégradé cylindrique lisse fait du
#    plastique. L'ombrage cel — un ton de base plat, un ton d'ombre plat à
#    frontière *nette*, et un liseré éclairé — est ce qu'utilise l'illustration
#    de personnage depuis toujours.
#
# Tout ce dont un personnage est fait passe par `Limb` ou `cel_poly`, si bien que
# la figure entière partage un seul modèle d'éclairage, et `contour` remet le
# trait dessiné là où deux formes de même couleur se recouvrent.

class_name Shape

## Fraction de la largeur, mesurée depuis le bord ombré, que couvre l'ombre.
const SHADOW := 0.44
## Fraction couverte par le liseré éclairé, mesurée depuis le bord éclairé.
const RIM := 0.15
const SHADE_MUL := 0.56
const RIM_MUL := 1.20

# ---------------------------------------------------------------- profils
# Vector3(t, demi-largeur avant, demi-largeur arrière), t de 0 à 1 le long de l'os.

## Deltoïde, biceps, coude. Le deltoïde fait près du double du coude : c'est ce
## rapport qui rattache un bras à un corps au lieu de l'y épingler.
const UPPER_ARM: Array[Vector3] = [
	Vector3(0.00, 1.02, 0.98), # la calotte, arrondie sur l'articulation
	Vector3(0.11, 1.46, 1.40),
	Vector3(0.28, 1.48, 1.40),
	Vector3(0.54, 1.18, 1.12),
	Vector3(0.80, 0.96, 0.94),
	Vector3(1.00, 0.84, 0.86),
]

## Avant-bras : le long supinateur enfle sous le coude, puis le poignet.
const FOREARM: Array[Vector3] = [
	Vector3(0.00, 0.92, 0.94),
	Vector3(0.26, 1.06, 1.02),
	Vector3(0.64, 0.80, 0.78),
	Vector3(1.00, 0.58, 0.56),
]

## Manche courte coiffant l'épaule.
const SLEEVE: Array[Vector3] = [
	Vector3(0.00, 1.46, 1.42),
	Vector3(0.35, 1.56, 1.50),
	Vector3(0.80, 1.48, 1.46),
	Vector3(1.00, 1.50, 1.48),
]

## Cuisse : lourde à la hanche, s'affinant au genou, plus pleine derrière.
const THIGH: Array[Vector3] = [
	Vector3(0.00, 1.88, 2.08),
	Vector3(0.30, 1.78, 1.94),
	Vector3(0.70, 1.50, 1.56),
	Vector3(1.00, 1.32, 1.32),
]

## Mollet : le gastrocnémien est *derrière*, et le devant du tibia est presque
## droit. Mettre ça dans le bon sens fait l'essentiel de la lecture d'une jambe.
const CALF: Array[Vector3] = [
	Vector3(0.00, 1.32, 1.38),
	Vector3(0.26, 1.26, 1.66),
	Vector3(0.62, 1.00, 1.12),
	Vector3(1.00, 0.70, 0.72),
]

## Pantalon bouffant : plein à la hanche et au genou, puis resserré d'un coup
## dans la botte. Ce resserrement est toute la silhouette — sans lui, c'est un
## pyjama.
const THIGH_BAGGY: Array[Vector3] = [
	Vector3(0.00, 1.94, 2.16),
	Vector3(0.35, 2.06, 2.28),
	Vector3(0.75, 2.10, 2.26),
	Vector3(1.00, 2.02, 2.16),
]

const CALF_BAGGY: Array[Vector3] = [
	Vector3(0.00, 2.04, 2.18),
	Vector3(0.28, 1.96, 2.20),
	Vector3(0.62, 1.52, 1.70),
	Vector3(0.88, 0.92, 0.98),
	Vector3(1.00, 0.82, 0.84),
]

## Torse : épaules, pectoral, le pincement à la taille, l'évasement aux hanches.
## Le départ étroit est la pente du trapèze qui monte vers le cou : un torse à
## pleine largeur dès l'épaule a le haut scié et pas d'épaules du tout.
const TORSO: Array[Vector3] = [
	Vector3(0.00, 0.50, 0.58),
	Vector3(0.15, 1.04, 1.16),
	Vector3(0.32, 1.10, 1.02),
	Vector3(0.58, 0.88, 0.88),
	Vector3(0.78, 0.76, 0.86),
	Vector3(1.00, 0.96, 1.10),
]

## Un corps en robe : pas de taille, juste une colonne qui s'élargit.
const TORSO_ROBE: Array[Vector3] = [
	Vector3(0.00, 0.52, 0.60),
	Vector3(0.15, 1.00, 1.10),
	Vector3(0.35, 1.04, 1.10),
	Vector3(0.70, 1.10, 1.20),
	Vector3(1.00, 1.26, 1.36),
]

## Un corps lourd — le geôlier. Le ventre devant, pas de taille du tout.
const TORSO_HEAVY: Array[Vector3] = [
	Vector3(0.00, 0.54, 0.62),
	Vector3(0.16, 1.00, 1.06),
	Vector3(0.34, 1.10, 1.04),
	Vector3(0.60, 1.24, 1.08),
	Vector3(0.84, 1.34, 1.10),
	Vector3(1.00, 1.14, 1.04),
]

## Le cou, dessiné de la mâchoire vers le bas : étroit sous l'oreille, évasé d'un
## coup dans le trapèze pour que la tête pose *sur* les épaules au lieu de tenir
## sur une tige. L'arrière s'évase plus que l'avant, là où est le trapèze.
const NECK: Array[Vector3] = [
	Vector3(0.00, 1.05, 1.22),
	Vector3(0.45, 1.12, 1.44),
	Vector3(1.00, 1.50, 2.05),
]

## Un os long : renflé aux deux bouts, mince dans la diaphyse. Ce profil *est* la
## lecture — une capsule de largeur constante est un bâton, pas un os.
const BONE: Array[Vector3] = [
	Vector3(0.00, 1.30, 1.30),
	Vector3(0.14, 0.84, 0.84),
	Vector3(0.50, 0.66, 0.66),
	Vector3(0.86, 0.80, 0.80),
	Vector3(1.00, 1.14, 1.14),
]

# ---------------------------------------------------------------- têtes
# En unités de rayon de tête, face vers +x, +y vers le bas.

## La tête de profil : front, arcade, nez, lèvres, menton, mâchoire, occiput. La
## forme d'un crâne ne distingue pas deux personnes à douze pixels de haut, mais
## un crâne *dessiné* au lieu d'un œuf distingue un personnage d'une bosse.
const HEAD: Array[Vector2] = [
	Vector2(-0.10, -1.02), # sommet du crâne
	Vector2(0.38, -0.94),
	Vector2(0.72, -0.64),  # front
	Vector2(0.84, -0.32),  # arcade sourcilière
	Vector2(0.76, -0.14),  # le creux à la racine du nez
	Vector2(0.92, 0.06),
	Vector2(1.12, 0.30),   # pointe du nez
	Vector2(0.92, 0.38),   # sous le nez
	Vector2(0.94, 0.48),   # lèvre supérieure
	Vector2(0.84, 0.56),   # la bouche
	Vector2(0.92, 0.64),   # lèvre inférieure
	Vector2(0.80, 0.74),   # le sillon au-dessus du menton
	Vector2(0.88, 0.86),   # menton
	Vector2(0.64, 0.98),   # sous le menton
	Vector2(0.08, 0.98),   # ligne de mâchoire
	Vector2(-0.46, 0.74),  # l'angle de la mâchoire
	Vector2(-0.84, 0.34),  # derrière l'oreille
	Vector2(-1.02, -0.10), # occiput
	Vector2(-0.96, -0.62),
	Vector2(-0.60, -0.94),
]

## La chevelure du prince en masse pleine, pas en couronne : la silhouette
## extérieure de la tempe au sommet puis à la nuque, refermée le long de
## l'implantation. Dessinée en bandeau autour du crâne, elle laisse le sommet nu —
## c'est la différence entre des cheveux et un bord de chapeau.
const HAIR: Array[Vector2] = [
	Vector2(0.86, -0.52),  # implantation, à la tempe
	Vector2(0.94, -0.80),  # la masse qui décolle du front
	Vector2(0.58, -1.14),
	Vector2(-0.08, -1.34), # sommet
	Vector2(-0.78, -1.16),
	Vector2(-1.20, -0.66),
	Vector2(-1.32, -0.08),
	Vector2(-1.34, 0.42),
	Vector2(-1.08, 0.88),  # la mèche qui rebique à la nuque
	Vector2(-0.72, 0.48),  # dessous de la mèche
	Vector2(-0.44, 0.06),  # derrière l'oreille
	Vector2(-0.08, -0.30), # l'implantation qui remonte sur la tempe
	Vector2(0.44, -0.46),
]

## Un crâne : même construction, mais la mâchoire pend et la boîte est ronde.
const SKULL: Array[Vector2] = [
	Vector2(-0.04, -1.08),
	Vector2(0.48, -0.96),
	Vector2(0.82, -0.56),
	Vector2(0.88, -0.16),
	Vector2(0.74, 0.02),
	Vector2(0.92, 0.20),
	Vector2(0.98, 0.42), # l'épine nasale à nu
	Vector2(0.80, 0.44),
	Vector2(0.86, 0.62), # dents du haut
	Vector2(0.88, 0.92), # la mâchoire, tombée
	Vector2(0.36, 1.06),
	Vector2(-0.10, 0.92),
	Vector2(-0.30, 0.60),
	Vector2(-0.72, 0.46),
	Vector2(-1.02, 0.02),
	Vector2(-1.04, -0.56),
	Vector2(-0.62, -1.00),
]


# ---------------------------------------------------------------- émission

## Le seul endroit d'où sort un polygone.
##
## Le trianguleur de Godot refuse les polygones dégénérés — sommets répétés,
## aire nulle — et il le fait bruyamment. Ces cas-là ne sont pas des bugs mais la
## conséquence normale d'un découpage (une ombre cel dont la coupe passe pile sur
## un sommet) ou d'un membre vu exactement de bout. On les filtre ici, une fois,
## plutôt que de les prévenir à trente endroits.
static func poly(ci: CanvasItem, pts: PackedVector2Array, col: Color) -> void:
	if pts.size() < 3 or col.a <= 0.004:
		return
	var clean := PackedVector2Array()
	for q in pts:
		if clean.is_empty() or clean[clean.size() - 1].distance_squared_to(q) > 1e-7:
			clean.append(q)
	if clean.size() >= 2 and clean[0].distance_squared_to(clean[clean.size() - 1]) <= 1e-7:
		clean.remove_at(clean.size() - 1)
	if clean.size() < 3:
		return
	var area := 0.0
	var n := clean.size()
	for i in n:
		var a := clean[i]
		var b := clean[(i + 1) % n]
		area += a.x * b.y - b.x * a.y
	if absf(area) < 2e-3:
		return
	# Un polygone qui se recoupe a une aire non nulle et échoue quand même à la
	# triangulation. Plutôt que de deviner, on demande sa réponse au trianguleur :
	# c'est le même travail que fera le serveur de rendu, et cela nous évite de
	# lui envoyer une forme qu'il refusera bruyamment.
	if Geometry2D.triangulate_polygon(clean).is_empty():
		return
	ci.draw_colored_polygon(clean, col)

# ---------------------------------------------------------------- membres

## Échantillonne un profil en `t` : renvoie (avant, arrière).
static func sample(prof: Array, t: float) -> Vector2:
	var n := prof.size()
	if n == 0:
		return Vector2.ZERO
	var p0: Vector3 = prof[0]
	if t <= p0.x:
		return Vector2(p0.y, p0.z)
	for i in range(n - 1):
		var a: Vector3 = prof[i]
		var b: Vector3 = prof[i + 1]
		if t <= b.x:
			var k := (t - a.x) / (b.x - a.x) if b.x > a.x else 0.0
			return Vector2(lerpf(a.y, b.y, k), lerpf(a.z, b.z, k))
	var l: Vector3 = prof[n - 1]
	return Vector2(l.y, l.z)

## L'« avant » anatomique d'un os allant de `a` à `b` sur une figure tournée vers
## `facing` : perpendiculaire à l'os et tournant avec lui, de sorte qu'une cuisse
## levée garde son quadriceps du bon côté.
static func front_of(a: Vector2, b: Vector2, facing: float) -> Vector2:
	var d := b - a
	if d.length() < 1e-4:
		return Vector2(-1.0 if facing < 0.0 else 1.0, 0.0)
	# orthogonal() envoie (0,1) — droit vers le bas — sur (-1,0), d'où le signe.
	return d.normalized().orthogonal() * (1.0 if facing < 0.0 else -1.0)

## Un os dessinable : un segment et la silhouette enroulée autour.
class Limb:
	var a: Vector2
	var b: Vector2
	## Vecteur unitaire vers l'avant anatomique du membre.
	var front: Vector2
	var prof: Array
	## Multiplicateur de demi-largeur, déjà en pixels écran.
	var girth: float
	## Échantillons le long de l'os. 8 suffit pour un bras, 10 pour un torse.
	var steps := 8
	## Multiplicateur de l'ombre propre. Une grande forme qui se détourne de la
	## lumière — le dos d'un torse — a besoin d'une ombre plus profonde qu'un
	## avant-bras, sans quoi les membres proches n'ont rien devant quoi passer.
	var shade := Shape.SHADE_MUL

	func _init(p_a: Vector2, p_b: Vector2, p_front: Vector2, p_prof: Array, p_girth: float) -> void:
		a = p_a
		b = p_b
		front = p_front
		prof = p_prof
		girth = p_girth

	func with_steps(n: int) -> Limb:
		steps = maxi(n, 3)
		return self

	func with_shade(k: float) -> Limb:
		shade = k
		return self

	## Un point du bord : `t` le long de l'os, `k` en travers (0 au bord arrière,
	## 1 au bord avant), poussé de `pad` pixels vers l'extérieur.
	func edge_pt(t: float, k: float, pad: float) -> Vector2:
		var w := Shape.sample(prof, t)
		var off := lerpf(-(w.y * girth + pad), w.x * girth + pad, k)
		return a.lerp(b, t) + front * off

	## La bande du membre comprise entre deux fractions de sa largeur.
	func slice(lo: float, hi: float, pad: float) -> PackedVector2Array:
		var pts := PackedVector2Array()
		var n := steps
		for i in n:
			pts.append(edge_pt(float(i) / (n - 1), hi, pad))
		for i in range(n - 1, -1, -1):
			pts.append(edge_pt(float(i) / (n - 1), lo, pad))
		return pts

	func outline(pad: float) -> PackedVector2Array:
		return slice(0.0, 1.0, pad)

	func lit_front(light: Vector2) -> bool:
		return front.dot(light) >= 0.0

	## Une forme sombre un peu plus grande que le membre : le trait dessiné qui le
	## sépare de ce qu'il recouvre.
	func edge(ci: CanvasItem, pad: float, col: Color) -> void:
		Shape.poly(ci, outline(pad), col)

	## Ton de base, ombre propre à bord franc, liseré éclairé.
	func draw(ci: CanvasItem, base: Color, light: Vector2) -> void:
		Shape.poly(ci, outline(0.0), base)
		var s_lo := 0.0
		var s_hi := Shape.SHADOW
		var r_lo := 1.0 - Shape.RIM
		var r_hi := 1.0
		if not lit_front(light):
			s_lo = 1.0 - Shape.SHADOW
			s_hi = 1.0
			r_lo = 0.0
			r_hi = Shape.RIM
		Shape.poly(ci, slice(s_lo, s_hi, 0.0), Geom.shade(base, shade))
		Shape.poly(ci, slice(r_lo, r_hi, 0.0), Geom.shade(base, Shape.RIM_MUL))

	## Point du bord éclairé en `t` — où accrocher un reflet ou démarrer un pli.
	func lit_edge(t: float, light: Vector2) -> Vector2:
		return edge_pt(t, 1.0 if lit_front(light) else 0.0, 0.0)

	func dark_edge(t: float, light: Vector2) -> Vector2:
		return edge_pt(t, 0.0 if lit_front(light) else 1.0, 0.0)

# ---------------------------------------------------------------- polygones

## Ombre un polygone écrit à la main — têtes, mains, étoffe, cheveux.
##
## L'ombre est tout ce qui est au-delà d'une droite perpendiculaire à `light`,
## placée pour couvrir `shade_at` de l'étendue de la forme. Un polygone ombré
## ainsi vit dans la même lumière que les membres qui l'entourent.
static func cel_poly(ci: CanvasItem, pts: PackedVector2Array, base: Color,
		light: Vector2, shade_at: float) -> void:
	if pts.size() < 3:
		return
	poly(ci, pts, base)
	var lo := INF
	var hi := -INF
	for p in pts:
		var d := p.dot(light)
		lo = minf(lo, d)
		hi = maxf(hi, d)
	# `light` pointe vers la lumière : « à l'opposé » est le côté des petits d.
	var cut := lerpf(lo, hi, clampf(shade_at, 0.03, 0.95))
	var dark := clip_half(pts, light, cut, true)
	if dark.size() >= 3:
		poly(ci, dark, Geom.shade(base, SHADE_MUL))
	# Le liseré doit rester *étroit*. Sur une forme allongée, un liseré généreux
	# couvre la moitié du volume et l'ensemble devient crayeux.
	var band := clip_half(pts, light, lerpf(lo, hi, 0.90), false)
	if band.size() >= 3:
		poly(ci, band, Geom.shade(base, RIM_MUL))

## Aplat sans ombrage — pour ce qui est déjà d'une seule valeur (un œil, le noir
## d'une bouche ouverte).
static func flat(ci: CanvasItem, pts: PackedVector2Array, col: Color) -> void:
	poly(ci, pts, col)

## Découpe de Sutherland–Hodgman contre le demi-plan `dot(p, n) <= cut`
## (ou `>=` si `keep_low` est faux).
static func clip_half(pts: PackedVector2Array, n: Vector2, cut: float,
		keep_low: bool) -> PackedVector2Array:
	var out := PackedVector2Array()
	var count := pts.size()
	for i in count:
		var cur := pts[i]
		var prev := pts[(i + count - 1) % count]
		var dc := cur.dot(n)
		var dp := prev.dot(n)
		var ci_in := dc <= cut if keep_low else dc >= cut
		var pi_in := dp <= cut if keep_low else dp >= cut
		if ci_in != pi_in:
			var k := (cut - dp) / (dc - dp) if absf(dc - dp) > 1e-6 else 0.0
			out.append(prev.lerp(cur, clampf(k, 0.0, 1.0)))
		if ci_in:
			out.append(cur)
	return out

## Un trait sombre le long d'un contour — là où un bras croise la poitrine, où
## l'écharpe rencontre le ventre, sous un pectoral. Le trait est ce qui sépare
## une forme d'une autre quand les deux sont de la même couleur, et c'est la
## différence entre un dessin et une masse.
static func contour(ci: CanvasItem, a: Vector2, b: Vector2, w: float,
		col: Color, alpha := 1.0) -> void:
	if w <= 1e-3:
		return
	var c := col
	c.a *= alpha
	var d := b - a
	if d.length() < 1e-4:
		return
	var n := d.normalized().orthogonal()
	var w2 := maxf(w * 0.7, 1e-3)
	poly(ci, PackedVector2Array([
		a + n * w, a - n * w, b - n * w2, b + n * w2,
	]), c)

## Dilate un polygone de `pad` pixels en poussant chaque sommet radialement
## depuis le centroïde. Sur les formes rondes dont un personnage est fait — une
## tête, une botte, une main — c'est indiscernable d'un vrai décalage de contour,
## et cela coûte une soustraction par sommet au lieu d'une passe de Clipper.
static func grow(pts: PackedVector2Array, pad: float) -> PackedVector2Array:
	var n := pts.size()
	if n < 3:
		return pts
	var c := Vector2.ZERO
	for q in pts:
		c += q
	c /= float(n)
	var out := PackedVector2Array()
	out.resize(n)
	for i in n:
		var d := pts[i] - c
		var l := d.length()
		out[i] = pts[i] + (d / l) * pad if l > 1e-4 else pts[i]
	return out

## Construit un polygone dans un repère local : `o` est l'origine, `+x` suit `ex`
## et `+y` suit `ey`. Toutes les formes écrites à la main passent par là, ce qui
## fait que les chiffres se lisent comme un dessin sur papier quadrillé.
static func frame(o: Vector2, ex: Vector2, ey: Vector2, pts: Array) -> PackedVector2Array:
	var out := PackedVector2Array()
	out.resize(pts.size())
	for i in pts.size():
		var p: Vector2 = pts[i]
		out[i] = o + ex * p.x + ey * p.y
	return out

## Idem, pour une liste de paires (x, y) écrites en dur.
static func frame_xy(o: Vector2, ex: Vector2, ey: Vector2, xy: Array) -> PackedVector2Array:
	var out := PackedVector2Array()
	out.resize(xy.size() / 2)
	for i in out.size():
		out[i] = o + ex * float(xy[i * 2]) + ey * float(xy[i * 2 + 1])
	return out

## Disque approché par un polygone — moins de primitives à raisonner que
## draw_circle, et il s'antialiase comme le reste.
static func disc(ci: CanvasItem, c: Vector2, r: float, col: Color, seg := 12) -> void:
	if r <= 1e-3:
		return
	var pts := PackedVector2Array()
	pts.resize(seg)
	for i in seg:
		var a := TAU * i / seg
		pts[i] = c + Vector2(cos(a), sin(a)) * r
	poly(ci, pts, col)

## Capsule effilée, pour les rubans d'étoffe et les manches d'arme : un
## demi-disque à chaque bout, reliés par les deux flancs.
static func capsule(ci: CanvasItem, a: Vector2, b: Vector2, ra: float, rb: float,
		col: Color) -> void:
	var d := b - a
	if d.length() < 1e-4 or maxf(ra, rb) <= 1e-3:
		return
	var t := d.normalized()
	var n := t.orthogonal()
	var pts := PackedVector2Array()
	# Bout `a` : de -n à +n en passant derrière.
	for i in range(7):
		var ang := lerpf(-PI * 0.5, PI * 0.5, float(i) / 6.0)
		pts.append(a - t * (cos(ang) * ra) + n * (sin(ang) * ra))
	# Bout `b` : de +n à -n en passant devant.
	for i in range(7):
		var ang := lerpf(PI * 0.5, -PI * 0.5, float(i) / 6.0)
		pts.append(b + t * (cos(ang) * rb) + n * (sin(ang) * rb))
	poly(ci, pts, col)
