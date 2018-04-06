#version 450
out vec3 color;

uniform samplerBuffer tex;
in vec4 v_position;


vec3 hsv2rgb(vec3 c)
{
    vec4 K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    vec3 p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
    return c.z * mix(K.xxx, clamp(p - K.xxx, 0.0, 1.0), c.y);
}

void main() {
	float stretch = 1024.;
	vec2 xy = v_position.xy * stretch;
	xy += 0.5;

	xy = vec2(abs(xy.x), abs(xy.y));
	color = vec3(0., 0., 0.);

	if (xy.x < stretch && xy.y < stretch) {
		int size = textureSize(tex);

		int p = int((size / stretch) * xy.x);
		float val = abs(texelFetch(tex, p).x);

		if (xy.y < val * stretch) {
			color = hsv2rgb(vec3(abs(sin(val + v_position.x)), 1.0, 1.0));
			if (v_position.y < 0.) {
				color -= 0.12;
			} else {
			//	color = 0.8 / color;
			}
//			color *= 1 / (stretch / xy.x);
		} else {
			color = vec3(0.1, 0.1, 0.1);
		}
	}
}
