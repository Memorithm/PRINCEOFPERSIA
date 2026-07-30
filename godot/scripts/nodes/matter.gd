# La matière : poussière, sang, gravats.
#
# Peinte après les personnages mais dans le monde éclairé — c'est de la matière,
# pas de la lumière, et elle doit donc s'assombrir avec la salle.

extends Node2D

var world: World

func _draw() -> void:
	if world and world.fx:
		world.fx.draw_matter(self)
