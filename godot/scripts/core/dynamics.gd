# État animé par case : course d'une herse, sortie des pointes, phase des lames,
# enfoncement d'une dalle, tremblement d'une planche descellée.
#
# Rangé en tableaux plats parallèles à la carte, pour que le rendu puisse
# interroger n'importe quelle case en temps constant sans une table de hachage
# par tuile et par image.

class_name Dynamics
extends RefCounted

## Une planche descellée est amorcée et compte à rebours.
const F_TRIGGERED := 1
## Les pointes ont été armées par un passage à proximité.
const F_ARMED := 2
## Cette herse est verrouillée ouverte pour de bon.
const F_LATCHED := 4
## Quelque chose repose sur cette dalle en ce moment.
const F_PRESSED := 8
## Des pointes qui ont déjà goûté au sang — dessinées avec la pointe rouge.
const F_BLOODY := 16

var tw := 0
var th := 0
## Valeur animée principale, selon la tuile : ouverture d'une herse, sortie des
## pointes, fermeture des lames, enfoncement d'une dalle.
var va: PackedFloat32Array
## Secondaire : minuteries (maintien d'une herse, mèche d'une planche, phase des
## lames).
var vb: PackedFloat32Array
var flags: PackedInt32Array

func _init(p_tw: int, p_th: int) -> void:
	tw = p_tw
	th = p_th
	var n := maxi(tw * th, 1)
	va.resize(n)
	vb.resize(n)
	flags.resize(n)

func _idx(tx: int, ty: int) -> int:
	if tx < 0 or ty < 0 or tx >= tw or ty >= th:
		return -1
	return ty * tw + tx

func a(tx: int, ty: int) -> float:
	var i := _idx(tx, ty)
	return va[i] if i >= 0 else 0.0

func b(tx: int, ty: int) -> float:
	var i := _idx(tx, ty)
	return vb[i] if i >= 0 else 0.0

func set_a(tx: int, ty: int, v: float) -> void:
	var i := _idx(tx, ty)
	if i >= 0:
		va[i] = v

func set_b(tx: int, ty: int, v: float) -> void:
	var i := _idx(tx, ty)
	if i >= 0:
		vb[i] = v

func has(tx: int, ty: int, f: int) -> bool:
	var i := _idx(tx, ty)
	return i >= 0 and (flags[i] & f) != 0

func set_flag(tx: int, ty: int, f: int, on: bool) -> void:
	var i := _idx(tx, ty)
	if i < 0:
		return
	if on:
		flags[i] |= f
	else:
		flags[i] &= ~f

func clear(tx: int, ty: int) -> void:
	var i := _idx(tx, ty)
	if i >= 0:
		va[i] = 0.0
		vb[i] = 0.0
		flags[i] = 0
