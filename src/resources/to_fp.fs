#version 460
#pragma tweak_shader(version="1.0")

#pragma input(float, name="width", default=0, max=10000)
#pragma input(float, name="height", default=0, max=10000)
layout(push_constant) uniform AeUtils {
  float width;
  float height;
};

#pragma input(image, name="input_image")
layout(set=0, binding=1) uniform sampler default_sampler;
layout(set=0, binding=2) uniform texture2D input_image;

layout(location = 0) out vec4 out_color; 

void main() {
	vec2 uv = gl_FragCoord.xy / vec2(width, height);
	out_color = texture(sampler2D(input_image, default_sampler), uv) * 1.99996948242;
}
