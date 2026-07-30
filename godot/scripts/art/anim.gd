# La bibliothèque d'animation : angles d'articulation clés pour chaque action.
#
# Les clips sont construits une fois au démarrage plutôt que déclarés en
# constantes, ce qui permet d'écrire les poses en GDScript ordinaire et de les
# retoucher au même endroit. Les durées sont en secondes jusqu'à la clé suivante.

class_name Anim

class Clip:
	var keys: Array = []   ## [[Pose, durée], ...]
	var looping := false

	func _init(p_keys: Array, p_looping: bool) -> void:
		keys = p_keys
		looping = p_looping

	func total() -> float:
		var n := keys.size()
		if n == 0:
			return 0.0
		var s := 0.0
		var last := n if looping else n - 1
		for i in last:
			s += keys[i][1]
		return s

	## Échantillonne à l'instant `t` (secondes). Un clip non bouclé tient sa
	## dernière pose.
	func sample(t: float) -> Skel.Pose:
		if keys.is_empty():
			return Skel.rest()
		if keys.size() == 1:
			return keys[0][0]
		var tot := total()
		var u := maxf(t, 0.0)
		if looping:
			if tot > 0.0:
				u = fmod(u, tot)
		elif u >= tot:
			return keys[keys.size() - 1][0]
		var i := 0
		while true:
			var d: float = keys[i][1]
			if u < d or i + 1 >= keys.size():
				var nxt := (i + 1) % keys.size() if looping else mini(i + 1, keys.size() - 1)
				var fr := minf(u / d, 1.0) if d > 0.0 else 1.0
				return keys[i][0].blend(keys[nxt][0], fr)
			u -= d
			i += 1
		return keys[0][0]

	## Avancement normalisé 0..1 d'un clip non bouclé.
	func progress(t: float) -> float:
		var tot := total()
		return 1.0 if tot <= 0.0 else minf(t / tot, 1.0)

static var _clips: Dictionary = {}

static func get_clip(n: String) -> Clip:
	if _clips.is_empty():
		_build()
	return _clips.get(n, _clips["stand"])

static func _build() -> void:
	# ---------------------------------------------------------- immobile
	var stand_a: Skel.Pose = Skel.ps(13.6, 0.3, -3.0, 3.0, [7.0, 17.0], [-15.0, 15.0], [11.0, 15.0], [-9.0, 8.0])
	var stand_b: Skel.Pose = Skel.ps(13.4, 0.3, -1.5, 4.0, [5.0, 15.0], [-12.0, 13.0], [11.0, 15.0], [-9.0, 8.0])
	_clips["stand"] = Clip.new([[stand_a, 1.6], [stand_b, 1.6]], true)

	var alert: Skel.Pose = Skel.ps(13.0, 0.9, 6.0, -3.0, [28.0, 44.0], [-20.0, 30.0], [13.0, 14.0], [-13.0, 14.0])
	var alert_b: Skel.Pose = Skel.ps(12.8, 1.0, 8.0, -4.0, [32.0, 48.0], [-22.0, 34.0], [15.0, 16.0], [-15.0, 16.0])
	_clips["stand_alert"] = Clip.new([[alert, 0.5], [alert_b, 0.5]], true)

	# ---------------------------------------------------------- demi-tour
	_clips["turn"] = Clip.new([
		[stand_a, 0.09],
		[Skel.ps(12.6, -0.6, 2.0, -14.0, [14.0, 26.0], [10.0, 24.0], [6.0, 8.0], [-4.0, 12.0]), 0.10],
		[Skel.ps(12.8, -0.4, 0.0, -6.0, [9.0, 18.0], [2.0, 16.0], [2.0, 5.0], [-3.0, 8.0]), 0.09],
		[stand_a, 0.0],
	], false)

	# ---------------------------------------------------------- course
	var r0: Skel.Pose = Skel.ps(13.0, 1.0, 15.0, -7.0, [-40.0, 26.0], [46.0, 60.0], [33.0, 13.0], [-31.0, 40.0])
	var r1: Skel.Pose = Skel.ps(11.9, 1.3, 17.0, -8.0, [-20.0, 32.0], [28.0, 66.0], [15.0, 22.0], [-14.0, 64.0])
	var r2: Skel.Pose = Skel.ps(13.5, 1.0, 13.0, -6.0, [6.0, 40.0], [8.0, 70.0], [-14.0, 14.0], [19.0, 76.0])
	_clips["run"] = Clip.new([
		[r0, 0.075], [r1, 0.075], [r2, 0.075],
		[r0.mirrored(), 0.075], [r1.mirrored(), 0.075], [r2.mirrored(), 0.075],
	], true)

	# Une allure plus lente et plus lourde pour les gardes en ronde.
	var w0: Skel.Pose = Skel.ps(12.9, 0.4, 6.0, -2.0, [-22.0, 18.0], [24.0, 30.0], [22.0, 8.0], [-20.0, 24.0])
	var w1: Skel.Pose = Skel.ps(12.4, 0.5, 8.0, -3.0, [-10.0, 22.0], [12.0, 34.0], [8.0, 14.0], [-8.0, 44.0])
	var w2: Skel.Pose = Skel.ps(13.1, 0.4, 5.0, -2.0, [4.0, 26.0], [2.0, 36.0], [-9.0, 9.0], [12.0, 52.0])
	_clips["walk"] = Clip.new([
		[w0, 0.17], [w1, 0.17], [w2, 0.17],
		[w0.mirrored(), 0.17], [w1.mirrored(), 0.17], [w2.mirrored(), 0.17],
	], true)

	_clips["run_start"] = Clip.new([
		[Skel.ps(12.2, 1.4, 20.0, -10.0, [-16.0, 22.0], [26.0, 48.0], [12.0, 34.0], [-22.0, 30.0]), 0.09],
		[Skel.ps(12.6, 1.8, 22.0, -10.0, [-30.0, 24.0], [40.0, 56.0], [26.0, 16.0], [-28.0, 46.0]), 0.09],
		[r0, 0.0],
	], false)

	_clips["run_stop"] = Clip.new([
		[Skel.ps(12.0, -1.6, -8.0, 6.0, [30.0, 40.0], [-24.0, 30.0], [26.0, 10.0], [-16.0, 54.0]), 0.11],
		[Skel.ps(12.6, -0.8, -4.0, 4.0, [18.0, 28.0], [-14.0, 22.0], [12.0, 6.0], [-8.0, 26.0]), 0.11],
		[stand_a, 0.0],
	], false)

	_clips["step"] = Clip.new([
		[stand_a, 0.12],
		[Skel.ps(12.8, 0.6, 4.0, -2.0, [-8.0, 16.0], [10.0, 22.0], [17.0, 8.0], [-10.0, 20.0]), 0.14],
		[Skel.ps(13.0, 0.4, 2.0, 0.0, [-4.0, 14.0], [5.0, 18.0], [7.0, 5.0], [-6.0, 12.0]), 0.12],
		[stand_a, 0.0],
	], false)

	# ---------------------------------------------------------- accroupi
	var crouch_pose: Skel.Pose = Skel.ps(6.6, 1.6, 34.0, -22.0, [34.0, 62.0], [26.0, 70.0], [62.0, 96.0], [48.0, 104.0])
	_clips["crouch"] = Clip.new([[crouch_pose, 1.0]], false)
	_clips["crouch_in"] = Clip.new([
		[stand_a, 0.07],
		[Skel.ps(10.0, 1.0, 20.0, -12.0, [20.0, 40.0], [14.0, 44.0], [34.0, 52.0], [26.0, 58.0]), 0.07],
		[crouch_pose, 0.0],
	], false)

	# ---------------------------------------------------------- en l'air
	_clips["jump_up"] = Clip.new([
		[Skel.ps(9.4, 0.4, 16.0, -8.0, [-18.0, 28.0], [-14.0, 30.0], [40.0, 62.0], [34.0, 66.0]), 0.09],
		[Skel.ps(13.6, 0.6, 4.0, -6.0, [120.0, 30.0], [128.0, 26.0], [10.0, 14.0], [4.0, 20.0]), 0.10],
		[Skel.ps(14.4, 0.4, -4.0, -10.0, [162.0, 14.0], [168.0, 12.0], [-6.0, 12.0], [-14.0, 26.0]), 0.16],
		[Skel.ps(14.0, 0.2, 0.0, -6.0, [150.0, 20.0], [156.0, 18.0], [6.0, 22.0], [-4.0, 34.0]), 0.0],
	], false)

	_clips["jump_run"] = Clip.new([
		[Skel.ps(11.6, 1.8, 24.0, -12.0, [-34.0, 26.0], [52.0, 54.0], [26.0, 46.0], [-30.0, 26.0]), 0.08],
		[Skel.ps(13.4, 1.6, 14.0, -8.0, [58.0, 40.0], [-32.0, 36.0], [56.0, 74.0], [-24.0, 48.0]), 0.14],
		[Skel.ps(13.8, 1.2, 6.0, -6.0, [96.0, 30.0], [-40.0, 30.0], [38.0, 30.0], [-32.0, 70.0]), 0.16],
		[Skel.ps(13.4, 1.0, 12.0, -8.0, [60.0, 44.0], [-20.0, 40.0], [44.0, 20.0], [-18.0, 60.0]), 0.0],
	], false)

	_clips["fall"] = Clip.new([
		[Skel.ps(13.2, 0.0, -6.0, -8.0, [144.0, 34.0], [152.0, 30.0], [16.0, 30.0], [-14.0, 44.0]), 0.18],
		[Skel.ps(13.4, 0.0, -10.0, -10.0, [156.0, 28.0], [140.0, 36.0], [24.0, 24.0], [-20.0, 52.0]), 0.18],
	], true)

	var land_squash: Skel.Pose = Skel.ps(7.4, 1.0, 30.0, -18.0, [40.0, 66.0], [32.0, 70.0], [54.0, 88.0], [44.0, 96.0]).with_squash(0.9)
	_clips["land"] = Clip.new([
		[land_squash, 0.10],
		[Skel.ps(11.0, 0.6, 14.0, -8.0, [22.0, 40.0], [16.0, 44.0], [24.0, 40.0], [18.0, 46.0]), 0.12],
		[stand_a, 0.0],
	], false)

	# ------------------------------------------------- suspension & escalade
	# Suspendu : jambes tendues, bras droits au-dessus de la tête. La portée que
	# cela produit est exactement Geom.HANG_DROP, pour que les mains tombent sur
	# la lèvre de la corniche.
	var hang_a: Skel.Pose = Skel.ps(13.0, -0.5, -2.0, 6.0, [172.0, 6.0], [176.0, 8.0], [1.0, 2.0], [-2.0, 5.0])
	var hang_b: Skel.Pose = Skel.ps(12.8, -0.5, -4.0, 7.0, [174.0, 7.0], [178.0, 9.0], [4.0, 4.0], [-5.0, 8.0])
	_clips["hang"] = Clip.new([[hang_a, 1.1], [hang_b, 1.1]], true)

	_clips["climb"] = Clip.new([
		[hang_a, 0.16],
		[Skel.ps(11.0, -0.2, 4.0, 0.0, [152.0, 54.0], [156.0, 56.0], [54.0, 92.0], [44.0, 86.0]), 0.18],
		[Skel.ps(9.0, 1.6, 26.0, -14.0, [118.0, 46.0], [124.0, 48.0], [66.0, 104.0], [30.0, 62.0]), 0.18],
		[Skel.ps(10.4, 1.4, 18.0, -8.0, [60.0, 40.0], [66.0, 42.0], [34.0, 56.0], [12.0, 28.0]), 0.16],
		[stand_a, 0.0],
	], false)

	# ---------------------------------------------------------- escrime
	var ready: Skel.Pose = Skel.ps(12.4, 0.6, 4.0, -4.0, [52.0, 34.0], [-30.0, 46.0], [16.0, 12.0], [-18.0, 30.0]).with_sword(-34.0)
	var ready_b: Skel.Pose = Skel.ps(12.2, 0.7, 6.0, -5.0, [56.0, 30.0], [-32.0, 44.0], [18.0, 14.0], [-20.0, 32.0]).with_sword(-30.0)
	_clips["sword_ready"] = Clip.new([[ready, 0.42], [ready_b, 0.42]], true)

	_clips["sword_adv"] = Clip.new([
		[ready, 0.09],
		[Skel.ps(12.0, 1.6, 10.0, -6.0, [58.0, 26.0], [-28.0, 44.0], [34.0, 14.0], [-24.0, 42.0]).with_sword(-30.0), 0.10],
		[ready, 0.0],
	], false)

	_clips["sword_ret"] = Clip.new([
		[ready, 0.09],
		[Skel.ps(12.2, -0.8, -2.0, 0.0, [46.0, 40.0], [-34.0, 48.0], [-14.0, 22.0], [22.0, 20.0]).with_sword(-40.0), 0.10],
		[ready, 0.0],
	], false)

	_clips["sword_strike"] = Clip.new([
		# armé
		[Skel.ps(12.4, -0.6, -6.0, -2.0, [24.0, 66.0], [-30.0, 50.0], [-6.0, 16.0], [14.0, 26.0]).with_sword(-58.0), 0.10],
		# botte
		[Skel.ps(11.8, 2.6, 18.0, -8.0, [86.0, 6.0], [-40.0, 40.0], [46.0, 12.0], [-30.0, 50.0]).with_sword(4.0), 0.09],
		# allongé
		[Skel.ps(11.9, 2.8, 20.0, -8.0, [90.0, 2.0], [-42.0, 38.0], [48.0, 10.0], [-32.0, 52.0]).with_sword(6.0), 0.07],
		# retour en garde
		[ready, 0.14],
		[ready, 0.0],
	], false)

	_clips["sword_parry"] = Clip.new([
		[ready, 0.06],
		[Skel.ps(12.4, -0.2, -8.0, 2.0, [74.0, 62.0], [-26.0, 44.0], [4.0, 16.0], [-14.0, 28.0]).with_sword(-96.0), 0.14],
		[ready, 0.0],
	], false)

	# ---------------------------------------------------------- coups & mort
	_clips["hurt"] = Clip.new([
		[Skel.ps(12.0, -1.8, -16.0, 14.0, [-30.0, 24.0], [-38.0, 20.0], [-16.0, 24.0], [18.0, 20.0]), 0.14],
		[Skel.ps(12.4, -0.8, -8.0, 8.0, [-16.0, 20.0], [-20.0, 18.0], [-6.0, 14.0], [8.0, 16.0]), 0.14],
		[stand_a, 0.0],
	], false)

	var fallen: Skel.Pose = Skel.ps(3.4, -2.0, 84.0, -34.0, [-56.0, 30.0], [34.0, 40.0], [76.0, 24.0], [58.0, 62.0]).with_tail(0.2)
	_clips["dead"] = Clip.new([
		[Skel.ps(9.0, -2.4, 46.0, -20.0, [-40.0, 30.0], [30.0, 36.0], [30.0, 40.0], [24.0, 50.0]), 0.18],
		[fallen, 0.0],
	], false)

	# ---------------------------------------------------------- divers
	_clips["drink"] = Clip.new([
		[stand_a, 0.14],
		[Skel.ps(12.8, 0.2, -4.0, 10.0, [128.0, 92.0], [-8.0, 16.0], [4.0, 6.0], [-8.0, 12.0]), 0.36],
		[Skel.ps(12.6, 0.0, -12.0, 22.0, [140.0, 104.0], [-6.0, 14.0], [2.0, 6.0], [-6.0, 12.0]), 0.30],
		[stand_a, 0.0],
	], false)

	_clips["throw"] = Clip.new([
		[Skel.ps(12.6, -0.8, -10.0, -2.0, [-46.0, 96.0], [-20.0, 30.0], [-8.0, 14.0], [12.0, 22.0]).with_sword(-70.0), 0.10],
		[Skel.ps(12.2, 2.0, 16.0, -8.0, [104.0, 12.0], [-30.0, 34.0], [30.0, 12.0], [-22.0, 40.0]).with_sword(10.0), 0.10],
		[stand_a, 0.0],
	], false)

	_clips["cast"] = Clip.new([
		[Skel.ps(12.6, -0.4, -8.0, -4.0, [120.0, 40.0], [-22.0, 32.0], [-6.0, 14.0], [10.0, 22.0]).with_sword(-20.0), 0.14],
		[Skel.ps(12.4, 1.4, 8.0, -6.0, [96.0, 8.0], [-28.0, 36.0], [22.0, 12.0], [-16.0, 34.0]).with_sword(6.0), 0.16],
		[stand_a, 0.0],
	], false)

	_clips["bow"] = Clip.new([
		[stand_a, 0.5],
		[Skel.ps(12.0, 1.0, 34.0, 12.0, [30.0, 30.0], [24.0, 34.0], [10.0, 12.0], [-10.0, 18.0]), 0.6],
		[stand_a, 0.0],
	], false)
