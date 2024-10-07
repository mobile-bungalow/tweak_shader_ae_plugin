#version 450
#pragma tweak_shader(version="1.0")
#pragma stage(compute)

#pragma input(float, name=blue, default=0.0, min=0.0, max=1.0)
#pragma input(float, name=green, default=0.0, min=0.0, max=1.0)
#pragma input(float, name=red, default=0.0, min=0.0, max=1.0)
layout(push_constant) uniform custom_inputs {
    float blue;
    float red;
    float green;
};

#pragma target(name="output_image", screen)
layout(rgba8, set = 0, binding = 1) uniform writeonly image2D output_image;

#pragma input(image, name="input_image")
layout(set = 0, binding = 3) uniform texture2D input_image;

layout(local_size_x = 16, local_size_y = 16) in;
void main() {
    ivec2 pixel_coords = ivec2(gl_GlobalInvocationID.xy);

    // Texture sampling is forbidden (!!!) in compute shaders
    // so use texelfetch.
    vec4 col = texelFetch(input_image, pixel_coords, 0);
    col.b *= blue;
    col.r *= red;
    col.g *= green;
    imageStore(output_image, pixel_coords, col);
}
