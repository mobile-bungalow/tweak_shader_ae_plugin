#pragma stage(compute)

#pragma input(image, name="input_image")
layout(set = 0, binding = 0) uniform texture2D input_image;

#pragma target(name="output_image", screen)
layout(set = 0, binding = 1, rgba16ui) uniform writeonly image2D output_image;

layout(local_size_x = 16, local_size_y = 16) in;
void main() {
    ivec2 pixel_coords = ivec2(gl_GlobalInvocationID.xy);
    vec4 input_value = texelFetch(input_image, pixel_coords, 0);
    // drop MSB. for adobe reasons
    uvec4 out_color = uvec4(input_value * 32767.0);
    out_color = min(out_color, uvec4(65535));

    imageStore(output_image, pixel_coords, out_color);
}
