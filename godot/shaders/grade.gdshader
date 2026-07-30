// Étalonnage final : vignette, grain, éclair de dégât, fondu.
//
// Passé en dernier sur toute l'image. La vignette et le grain sont ce qui empêche
// une scène faite d'aplats de polygones d'avoir l'air d'un diagramme : ils
// remettent de la matière là où le rendu vectoriel n'en a pas.

shader_type canvas_item;
render_mode unshaded;

uniform sampler2D screen : hint_screen_texture, filter_linear_mipmap;

uniform vec3 vignette_tint = vec3(0.03, 0.03, 0.06);
uniform float vignette_amount : hint_range(0.0, 1.0) = 0.52;
uniform float grain : hint_range(0.0, 0.2) = 0.022;
uniform vec3 flash_colour = vec3(0.6, 0.1, 0.1);
uniform float flash : hint_range(0.0, 1.0) = 0.0;
uniform float fade : hint_range(0.0, 1.0) = 0.0;
uniform float time_s = 0.0;

float hash21(vec2 p) {
	p = fract(p * vec2(233.34, 851.73));
	p += dot(p, p + 23.45);
	return fract(p.x * p.y);
}

void fragment() {
	vec2 uv = SCREEN_UV;
	vec3 c = texture(screen, uv).rgb;

	// Vignette : distance au centre, corrigée du rapport de l'écran pour que le
	// halo reste rond sur un format large.
	vec2 d = (uv - 0.5) * vec2(1.0, 0.62);
	float r = length(d) * 1.85;
	float v = smoothstep(0.55, 1.25, r) * vignette_amount;
	c = mix(c, vignette_tint, v);

	// Grain, animé mais discret.
	float g = hash21(uv * vec2(1920.0, 1080.0) + fract(time_s) * 91.7) - 0.5;
	c += g * grain;

	c = mix(c, flash_colour, flash * 0.45);
	c *= 1.0 - fade;

	COLOR = vec4(c, 1.0);
}
