#version 450

// Original Shader by movAX13h, https://www.shadertoy.com/user/movAX13h
// Used unattributed by VidVox.

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
#pragma input(float, name=size, default=1.0, min=0.0, max=10.0)
#pragma input(float, name=tint, default=1.0, min=0.0, max=1.0)
#pragma input(bool, name=alphaMode, default=false)
#pragma input(color, name=tintColor, default=[0.0, 1.0, 0.0, 1.0])
layout(set = 0, binding = 3) uniform custom_inputs {
    float gamma;
    float size;
    float tint;
    int alphaMode;
    vec4 tintColor;
};

int getNthBit(uint value, int n) {
    return int((value >> uint(n)) & uint(1));
}

float character(uint j, vec2 p) // some compilers have the word "char" reserved
{
    p = floor(p * vec2(-4.0, 4.0) + 4.0);
    if (clamp(p.x, 0.0, 4.0) == p.x && clamp(p.y, 0.0, 4.0) == p.y)
    {
        int index = int(p.x + (p.y * 5.0));
        int i = getNthBit(j, index);
        return float(i);
    } else {
        return 0.0;
    }
}

void main() {
    float _size = size * 36.0 + 8.0;
    //vec2 uv = gl_FragCoord.xy;

    vec2 uv = gl_FragCoord.xy;
    vec4 inputColor = texture(sampler2D(input_image, default_sampler), (floor(uv / _size) * _size / resolution.xy));
    vec3 col = inputColor.rgb;
    float gray = (col.r + col.g + col.b) / 3.0;
    gray = pow(gray, gamma);
    col = mix(tintColor.rgb, col.rgb, 1.0 - tint);

    int l = 0;
    uint j = 65536;
    if (gray > 0.2) j = 65600; // :
    if (gray > 0.3) j = 332772; // *
    if (gray > 0.4) j = 15255086; // o
    if (gray > 0.5) j = 23385164; // &
    if (gray > 0.6) j = 15252014; // 8
    if (gray > 0.7) j = 13199452; // @
    if (gray > 0.8) j = 11512810; // #

    vec2 p = mod(uv / (_size / 2.0), 2.0) - vec2(1.0);
    col = col * character(j, p);
    float alpha = mix(tintColor.a * inputColor.a, inputColor.a, 1.0 - tint);
    if (alphaMode != 0) {
        alpha = (col.r + col.g + col.b) / 3.0;
        alpha = (alpha > 0.01) ? tintColor.a : alpha;
    }

    out_color = vec4(col, alpha);
}
