#version 450
#pragma tweak_shader(version="1.0")

// Original Shader by movAX13h, https://www.shadertoy.com/user/movAX13h
// Used unattributed by VidVox. modified by mobile bungalow.

#pragma utility_block(ShaderInputs)
layout(push_constant) uniform ShaderInputs {
    float time; // shader playback time (in seconds)
    float time_delta; // elapsed time since last frame in secs
    float frame_rate; // number of frames per second estimates
    uint frame_index; // frame count
    vec4 mouse; // xy is last mouse down position,  abs(zw) is current mouse, sign(z) > 0.0 is mouse_down, sign(w) > 0.0 is click_down event
    vec4 date; // [year, month, day, seconds]
    vec3 resolution; // viewport resolution in pixels, [w, h, w/h]
    uint pass_index; // updated to reflect render pass
};

layout(location = 0) out vec4 out_color;

#pragma input(image, name="input_image")
layout(set = 0, binding = 1) uniform sampler default_sampler;
layout(set = 0, binding = 2) uniform texture2D input_image;

#pragma input(float, name=gamma, default=0.5, min=0.0, max=1.0)
#pragma input(float, name=size, default=0.5, min=0.0, max=10.0)
#pragma input(float, name=tint, default=1.0, min=0.0, max=1.0)
#pragma input(color, name=tintColor, default=[0.0, 1.0, 0.0, 1.0])
layout(set = 0, binding = 3) uniform custom_inputs {
    float gamma;
    float size;
    float tint;
    vec4 tintColor;
};

uint getNthBit(uvec4 vector, int n) {
    int componentIndex = n / 32; // Each ivec4 component has 32 bits
    int bitIndex = n - componentIndex * 32;

    return (vector[componentIndex] >> bitIndex) & 1;
}

const float char_width = 8.0;
const float char_height = 16.0;
float character(uvec4 n, vec2 p)
{
    if (getNthBit(n, int(p.x * char_width) + int(p.y * char_height) * int(char_width)) == 1) return 1.0;
    return 0.0;
}
void main() {
    float _size = floor(size + 1.0);
    vec2 size_dim = vec2(_size * char_width, _size * char_height);

    vec2 uv = gl_FragCoord.xy;
    vec2 grid_v = floor(uv / size_dim) * size_dim / resolution.xy;
    vec2 grid_v_right = floor((uv + vec2(size_dim.x / 2.0, 0)) / size_dim) * size_dim / resolution.xy;
    vec2 grid_v_down = floor((uv + vec2(0, size_dim.y)) / size_dim) * size_dim / resolution.xy;
    vec4 inputColor = texture(sampler2D(input_image, default_sampler), grid_v);
    vec4 inputColor_r = texture(sampler2D(input_image, default_sampler), grid_v_right);
    vec4 inputColor_d = texture(sampler2D(input_image, default_sampler), grid_v_down);
    vec3 col = inputColor.rgb;
    float gray = (col.r + col.g + col.b) / 3.0;
    gray = pow(gray, gamma);
    col = mix(tintColor.rgb, col.rgb, 1.0 - tint);

    uvec4 n = uvec4(0);
    vec2 p = mod(uv, size_dim) / size_dim;
    if (length(inputColor - inputColor_d) > 0.5) {
        n = uvec4(4294967295, 16777215, 0, 0);
        if (character(n, p) == 1) {
            col = min(inputColor, inputColor_r).rgb;
        }
    }
    if (length(inputColor - inputColor_r) > 0.5) {
        n = uvec4(252645135);
        if (character(n, p) == 1) {
            col = min(inputColor, inputColor_r).rgb;
        }
    } else {
        if (gray > 0.3) n = uvec4(289673540); // light grid
        if (gray > 0.4) n = uvec4(1437226410); // darker grid
        if (gray > 0.7) n = uvec4(0xFFFFFFFF); // no hatching
        if (character(n, p) == 1) {
            col = inputColor.rgb - 0.2;
        }
    }

    out_color = vec4(col, 1.0);
}
