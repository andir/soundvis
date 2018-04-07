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
	vec2 xy = v_position.xy;
	//xy.x += 1.0;
	//xy.x /= 2.0;

	//xy += 0.5;

	xy = vec2(abs(xy.x), abs(xy.y));
	color = vec3(0., 0., 0.);

	int size = textureSize(tex);

	int p = int(size * xy.x);
//	float pval = abs(texelFetch(tex, p - 1).x);
	float val = abs(texelFetch(tex, p).x);
//	float nval = abs(texelFetch(tex, p + 1).x);

	if (xy.y < val) {
		color = vec3(float(p)/float(size), 0., xy.y);
		//color = vec3(0.0, 0.0, 1.0);
	//} else if (xy.y <= nval) {
	//	color = vec3(float(p)/size, 0., xy.y);
	//	color *= 0.8;
	//} else if (xy.y <= pval) {
	//	color = vec3(float(p)/size, 0., xy.y);
	//	color *= 0.8;
	} else {
		color = vec3(1.0, 1.0, 1.0);
	}
}
