shader_type canvas_item;
render_mode unshaded, blend_disabled;

uniform sampler2D algae_texture : filter_linear, repeat_enable;

void fragment() {
	vec2 field = UV * vec2(SCREEN_PIXEL_SIZE.y / SCREEN_PIXEL_SIZE.x, 1.0);
	// Integer tile shifts keep the repeating drift continuous when TIME rolls over
	float drift = TIME / 3600.0;
	float near_algae = texture(algae_texture, field * 1.4 + drift * vec2(13.0, -7.0)).r;
	float far_algae = texture(algae_texture, field * 0.65 + drift * vec2(-9.0, 4.0)).r;
	float clusters = smoothstep(0.47, 0.84, near_algae);
	float haze = smoothstep(0.19, 0.81, far_algae);
	vec3 water = mix(vec3(0.018, 0.046, 0.033), vec3(0.050, 0.100, 0.070), 1.0 - UV.y);
	COLOR = vec4(water + clusters * vec3(0.027, 0.057, 0.023) + haze * vec3(0.011, 0.029, 0.015), 1.0);
}
