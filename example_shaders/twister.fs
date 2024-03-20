#version 450
#pragma tweak_shader(version=1.0)

// Original art by tsone. https://www.shadertoy.com/user/tsone

#pragma utility_block(ShaderInputs)
layout(push_constant) uniform ShaderInputs {
    float time;       // shader playback time (in seconds)
    float time_delta; // elapsed time since last frame in secs
    float frame_rate; // number of frames per second estimates
    uint frame_index;  // frame count
    vec4 mouse;       // xy is last mouse down position,  abs(zw) is current mouse, sign(z) > 0.0 is mouse_down, sign(w) > 0.0 is click_down event
    vec4 date;        // [year, month, day, seconds]
    vec3 resolution;  // viewport resolution in pixels, [w, h, w/h]
    uint pass_index;   // updated to reflect render pass
};

#pragma input(color, name=albedo, default=[0.0, 0.0, 1.0, 1.0])
#pragma input(bool, name=ADDBASE, default=true)
#pragma input(bool, name=ADDNOISE, default=true)
#pragma input(bool, name=OLDSKOOL, default=true)
#pragma input(float, name=RES, min=1.0, default=30.0)
#pragma input(float, name=time_scale, default=1.0, min=0.0001)
#pragma input(float, name=STEPSTART, default=0.777, max=0.9, min=0.5)
#pragma input(float, name=NOISESCALE, default=2.0)
#pragma input(float, name=STEPS, min=1.0, default=10.0)

layout(set = 1, binding = 1) uniform CustomInput {
  int ADDBASE;
  int ADDNOISE;
  int OLDSKOOL;
  float RES;
  float STEPSTART;
  float NOISESCALE;
  float time_scale;
  float STEPS;
  vec4  albedo;
};

layout(location = 0) out vec4 out_color; 

#define PI 3.14159265359

// Based on iq's 'Noise - value noise' shader:
// https://www.shadertoy.com/view/lsf3WH
float hash(in vec2 p)
{
	float h = dot(p, vec2(127.1, 311.7));
	return fract(sin(h) * 43758.5453123);
}

float vnoiseh2(in vec2 p)
{
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	float a = hash(i + vec2(0.0, 0.0));
	float b = hash(i + vec2(1.0, 0.0));
	float c = hash(i + vec2(0.0, 1.0));
	float d = hash(i + vec2(1.0, 1.0));
	return mix(mix(a, b, u.x),
			   mix(c, d, u.x), u.y);
}

// Normal calculation separated from height to reduce loop complexity.
// If both height and normal are needed in same place, then it would make
// sense to combine the calculations.
// Noise derivates/normal based on iq's article:
// https://iquilezles.org/articles/morenoise
// NOTE: Result is unnormalized.
vec3 vnoisen2(in vec2 p)
{
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 dl = 6.0 * f * (1.0 - f);
	vec2 u = f * f * (3.0 - 2.0 * f);
	float a = hash(i + vec2(0.0, 0.0));
	float b = hash(i + vec2(1.0, 0.0));
	float c = hash(i + vec2(0.0, 1.0));
	float d = hash(i + vec2(1.0, 1.0));
	return vec3(
		dl.x * mix((b - a), (d - c), u.y),
		dl.y * mix((c - a), (d - b), u.x),
		-1.0);
}

float baseh(in vec2 a)
{
	vec2 s = sin(a);
	vec2 s2 = s * s;
	return (s2.y * s2.x);
}

// Height map normal calculation explained:
// http://http.developer.nvidia.com/GPUGems/gpugems_ch01.html
vec3 basen(in vec2 a)
{
	vec2 s = sin(a);
	vec2 c = cos(a);
	vec2 s2 = s * s;
	return normalize(vec3(
		2.0 * c.x * s.x * s2.y,
		2.0 * c.y * s.y * s2.x,
		-1.0));
}

float height(in vec2 a)
{
	float h = 0.74;
	if (ADDBASE != 0)
	{
		h += 0.2 * baseh(a);
	}

	if (ADDNOISE != 0) {
    h += 0.06 * vnoiseh2(NOISESCALE * a);
  }
	return h;
}

vec3 normal(in vec2 a)
{
	vec3 n = vec3(0.0);
	if (ADDBASE != 0)
	{
		n += basen(a);
	}
	if (ADDNOISE != 0) {
		 n += 0.25 * vnoisen2(NOISESCALE * a);
  }
	return normalize(n);
}
void run(out float _a, inout vec2 _p, in vec2 uv)
{
	uv *= 1.333;

	_a = -PI;

	float dz = -STEPSTART / STEPS;
	vec3 v = vec3(uv.x, uv.y * RES * 0.25 * PI, STEPSTART);
	if (OLDSKOOL != 0)
	{
		v.y = floor(v.y + 0.5);
	}

  float iTime = time * time_scale;
	vec2 offs = vec2(RES * (0.5 * PI * (0.8 + 0.2 * cos(iTime)) * sin(2.0 * iTime + 0.5 * v.y / RES)),
					 v.y);

	if (OLDSKOOL != 0)
	{
		offs = floor(offs + 0.5);
	}

	for (int i = 0; i < int(STEPS); i++)
	{
		v.z += dz;
		float a = atan(v.x, v.z) * RES;
		if (OLDSKOOL != 0)
		{
			a = floor(a + 0.5);
		}
		vec2 p = offs + vec2(a, 0.0);
		p *= 4.0 / RES;
		float r = length(v.xz);
		float h = height(p);
		if (r < h)
		{
			_a = a / RES;
			_p = p;
			v.x = 1e10;
		}
	}
}

void main()
{
	vec2 uv = 2.0 * gl_FragCoord.xy / resolution.xy - 1.0;
	float a;
	vec2 p;
	run(a, p, uv);
	vec3 n = normal(p);
	vec3 c;
	a = -a;
	float tx = n.x;
	n.x = n.x * cos(a) - n.z * sin(a);
	n.z = n.z * cos(a) + tx * sin(a);
  float iTime = time * time_scale;
	vec3 l = -normalize(vec3(cos(iTime), sin(-iTime), 1.0));
	float ndotl = max(0.0, dot(n, l));
	c = vec3(0.50, 0.35, 0.20) + albedo.xyz + vec3(0.60, 0.70, 0.80) * ndotl * ndotl;
	c *= c * step(a, 0.5 * PI);
	out_color = vec4(c, 1.0);
}
