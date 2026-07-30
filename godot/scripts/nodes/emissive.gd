# La passe émissive : flammes, étincelles, halos d'objets, traits de lame.
#
# Elle vit dans un CanvasLayer distinct qui suit la caméra mais n'est pas soumis
# au CanvasModulate du monde. C'est ce qui fait qu'une flamme reste une flamme
# dans une salle plongée dans le noir, au lieu d'être une flamme assombrie.

extends Node2D

var world: World

func _draw() -> void:
	if world:
		world.draw_emissive(self)
