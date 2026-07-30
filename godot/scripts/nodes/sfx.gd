# Bruitages synthétisés au démarrage.
#
# Aucun fichier audio n'est stocké : chaque son est un petit tampon PCM calculé à
# l'ouverture du jeu. C'est ce qui permet au dépôt de rester du texte, et cela
# suffit largement pour un jeu dont la palette sonore est faite de pas, d'acier et
# de pierre.

class_name Sfx
extends Node

const RATE := 22050

var _bank := {}
var _players: Array[AudioStreamPlayer] = []
var _next := 0

func _ready() -> void:
	_bank["step"] = _noise(0.06, 900.0, 0.35, 0.55)
	_bank["land"] = _noise(0.16, 260.0, 0.60, 0.75)
	_bank["jump"] = _tone(0.12, 300.0, 620.0, 0.32, 0.0)
	_bank["clash"] = _mix(_noise(0.14, 4200.0, 0.55, 0.35), _tone(0.14, 1800.0, 900.0, 0.25, 6.0))
	_bank["hit"] = _mix(_noise(0.20, 700.0, 0.75, 0.6), _tone(0.20, 220.0, 90.0, 0.35, 0.0))
	_bank["gate"] = _noise(0.55, 180.0, 0.35, 0.5)
	_bank["spike"] = _tone(0.10, 1500.0, 380.0, 0.40, 0.0)
	_bank["potion"] = _tone(0.42, 520.0, 1180.0, 0.24, 3.0)
	_bank["death"] = _tone(0.90, 340.0, 60.0, 0.42, 0.0)
	_bank["pickup"] = _tone(0.26, 880.0, 1500.0, 0.22, 0.0)
	for i in 10:
		var p := AudioStreamPlayer.new()
		p.bus = "Master"
		add_child(p)
		_players.append(p)

func play(name: String, volume_db := -8.0, pitch := 1.0) -> void:
	if not _bank.has(name):
		return
	var p := _players[_next]
	_next = (_next + 1) % _players.size()
	p.stream = _bank[name]
	p.volume_db = volume_db
	p.pitch_scale = pitch
	p.play()

# ---------------------------------------------------------------- synthèse

static func _wav(data: PackedByteArray) -> AudioStreamWAV:
	var s := AudioStreamWAV.new()
	s.format = AudioStreamWAV.FORMAT_16_BITS
	s.mix_rate = RATE
	s.stereo = false
	s.data = data
	return s

static func _pack(samples: PackedFloat32Array) -> AudioStreamWAV:
	var b := PackedByteArray()
	b.resize(samples.size() * 2)
	for i in samples.size():
		var v := int(clampf(samples[i], -1.0, 1.0) * 32000.0)
		b.encode_s16(i * 2, v)
	return _wav(b)

## Bruit filtré : un pas, une réception, une herse qui coulisse.
static func _noise(dur: float, cut: float, decay: float, amp: float) -> AudioStreamWAV:
	var n := int(dur * RATE)
	var out := PackedFloat32Array()
	out.resize(n)
	var rng := RandomNumberGenerator.new()
	rng.seed = int(cut * 977.0)
	var lp := 0.0
	var k := clampf(cut / float(RATE), 0.01, 0.9)
	for i in n:
		var t := float(i) / n
		lp += (rng.randf_range(-1.0, 1.0) - lp) * k
		out[i] = lp * amp * pow(1.0 - t, 1.0 + decay * 6.0)
	return _pack(out)

## Sinus glissant, avec un peu de vibrato.
static func _tone(dur: float, f0: float, f1: float, amp: float, vib: float) -> AudioStreamWAV:
	var n := int(dur * RATE)
	var out := PackedFloat32Array()
	out.resize(n)
	var ph := 0.0
	for i in n:
		var t := float(i) / n
		var f := lerpf(f0, f1, t) * (1.0 + (sin(t * TAU * vib) * 0.03 if vib > 0.0 else 0.0))
		ph += TAU * f / RATE
		out[i] = sin(ph) * amp * pow(1.0 - t, 2.2)
	return _pack(out)

static func _mix(a: AudioStreamWAV, b: AudioStreamWAV) -> AudioStreamWAV:
	var na := a.data.size() / 2
	var nb := b.data.size() / 2
	var n := maxi(na, nb)
	var out := PackedFloat32Array()
	out.resize(n)
	for i in n:
		var v := 0.0
		if i < na:
			v += a.data.decode_s16(i * 2) / 32768.0
		if i < nb:
			v += b.data.decode_s16(i * 2) / 32768.0
		out[i] = clampf(v, -1.0, 1.0)
	return _pack(out)
