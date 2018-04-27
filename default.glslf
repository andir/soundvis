#version 450
out vec3 color;

uniform samplerBuffer tex;
uniform samplerBuffer beat;
uniform float time;
in vec4 v_position;

vec3 hsv2rgb(vec3 c)
{
    vec4 K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    vec3 p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
    return c.z * mix(K.xxx, clamp(p - K.xxx, 0.0, 1.0), c.y);
}

vec3 pallete(float v, float p, float size, vec2 xy) {
    return hsv2rgb(vec3(float(p)/float(size), 0.5 + v, 1.0));
}

void main() {
	vec2 xy = v_position.xy;
	//xy.x += 1.0;
	//xy.x /= 2.0;

	//xy += 0.5;

	xy = vec2(abs(xy.x), abs(xy.y));
	xy.y *= 1.25;
	color = vec3(0., 0., 0.);

	vec2 linien = xy * 7;

//	for (int i = 0; i < 6; ++i) {
//		if (linien.x >= i+ 0.99 && linien.x <= i + 1.01) {
//			color = vec3(0.0, 0.0, 0.0);
//			return;
//		}
//	}


	float b = texelFetch(beat, 0).x;

	int size = textureSize(tex);

	int p = int(size * xy.x);
	float val = abs(texelFetch(tex, p).x);

        float sum = 0.0;
	for (int i = 0; i < size; ++i) sum += texelFetch(tex, p).x;
	float avg = sum / float(size);

	if (xy.y < val) {
		color = pallete(avg, float(p), float(size), xy);
	} else {
		if (b > 0.0) {
			color = hsv2rgb(vec3(abs(sin(time)), 0.3, 0.3));
		} else {
			color = vec3(1.0, 1.0, 1.0);
		}
	}

	if (v_position.y < 0.0) {
		color /= abs(v_position.y);
	}
}
