# Le calque des personnages.
#
# Peint après le décor et avant la matière. Chaque figure y pose d'abord sa
# silhouette détourée, puis ses formes par-dessus : c'est ce qui détache un
# personnage d'un mur de la même valeur, et sans quoi une figure sombre dans une
# salle sombre n'a pas de contour du tout.

extends Node2D

var world: World

func _draw() -> void:
	if world:
		world.draw_figures(self)
