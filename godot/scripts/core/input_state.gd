# Instantané des commandes pour une image de simulation.
#
# La simulation ne lit jamais le clavier directement : elle reçoit cet
# instantané. C'est ce qui permet de rejouer une partie, de piloter le prince
# depuis un test, ou de brancher une manette sans toucher au jeu.

class_name InputState
extends RefCounted

var left := false
var right := false
var up := false
var down := false
var careful := false
var attack := false
var parry := false
var throw_it := false
var cast := false
var sheathe := false
## Fronts montants — un saut demandé au bon moment ne doit pas être avalé.
var up_edge := false
var down_edge := false
var attack_edge := false

func dir() -> float:
	if right and not left:
		return 1.0
	if left and not right:
		return -1.0
	return 0.0

func any_dir() -> bool:
	return left != right

func clear() -> void:
	left = false; right = false; up = false; down = false
	careful = false; attack = false; parry = false
	throw_it = false; cast = false; sheathe = false
	up_edge = false; down_edge = false; attack_edge = false

## Lit le clavier. Les noms d'action sont déclarés par Main.
func poll() -> void:
	left = Input.is_action_pressed("pop_left")
	right = Input.is_action_pressed("pop_right")
	up = Input.is_action_pressed("pop_up")
	down = Input.is_action_pressed("pop_down")
	careful = Input.is_action_pressed("pop_careful")
	attack = Input.is_action_just_pressed("pop_attack")
	parry = Input.is_action_pressed("pop_parry")
	throw_it = Input.is_action_just_pressed("pop_throw")
	cast = Input.is_action_just_pressed("pop_cast")
	sheathe = Input.is_action_just_pressed("pop_sheathe")
	up_edge = Input.is_action_just_pressed("pop_up")
	down_edge = Input.is_action_just_pressed("pop_down")
	attack_edge = attack
