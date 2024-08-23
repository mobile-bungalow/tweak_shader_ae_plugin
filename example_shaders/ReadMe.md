# Lib Tweak Shader

The Tweak Shader Library provides a rendering and bookkeeping context for an interactive screen shader format. It allows users to create shaders reminiscent of ShaderToy or ISF shaders with custom uniforms that can be tweaked at runtime. The library features support for image inputs as well as various other types, including colors, floats, integers, 2D points, and more. The design and functionality of this library were inspired by the ISF (Interactive Shader Format) project.

 ## Usage

 ```rust, ignore
 use tweak_shader::RenderContext;
 use wgpu::TextureFormat;

 let src =  r#"
#version 450
#pragma tweak_shader(version=1.0)

layout(location = 0) out vec4 out_color;

#pragma input(float, name="foo", default=0.0, min=0.0, max=1.0)
#pragma input(float, name="bar")
#pragma input(float, name="baz", default=0.5)
layout(set = 0, binding = 0) uniform Inputs {
    float foo;
    float bar;
    float baz;
};

void main()
{
    out_color = vec4(foo, bar, baz, 1.0);
}
 "#;

 let format = TextureFormat::Rgba8UnormSrgb;
 let device = // your wgpu::Device here;
 let queue = // your wgpu::Queue here;

 let render_context = RenderContext::new(isf_shader_source, format, &device, &queue).unwrap();

 // Congratulations! You now have a 255x255 blue square.
 let output = render_context.render_to_vec(&queue, &device, 255, 255);

 ```

 The valid document pragmas are as follows.

 ### Input Types

 Input pragmas provide information about shader inputs, including type, name, and optional attributes such as default values, minimum and maximum bounds, labels, and valid values. 

| Pragma Type| Underlying Type | fields                                  | Description                                                                                                                   |
|-------------|-----------------|-----------------------------------------|-------------------------------------------------------------------------------------------------------------------------------|
| float       | float           | name, min, max, default                 | A single float input                                                                                                          |
| int         | int             | name, min, max, default, labels, values | A single int input with an optional list of labels and valid values                                                           |
| bool        | int             | name, default                           | A boolean, represented by an int (0 - false, 1 - true) for Naga related reasons.                                              |
| Point       | vec2            | name, min, max, default                 | A 2d point.                                                                                                                   |
| Color       | vec4            | name, default                           | A color input with straight alpha.                                                                                            |
| Image       | Texture2d       | name | A free texture slot to load with images or image sequences.                                                                            |


 Here are some examples 

 - Float Input:

 ```glsl
 #pragma input(float, name="foo", default=0.0, min=0.0, max=1.0)
 ```

 - Integer Input with Labels:

 ```glsl
 #pragma input(int, name="mode", default=0, values=[0, 1, 2], labels=["A", "B", "C"])
 ```

 - Image Input:

 ```glsl
 #pragma input(image, name="input_image", path="./demo.png")
layout(set=1, binding=1) uniform sampler default_sampler;
layout(set=1, binding=2) uniform texture2D input_image;
 ```


 Each input pragma corresponds to a uniform variable in the shader code, with the `name` field specifying the matching struct field in the global uniform value or the texture name that maps to the input.


### Sampler Configuration

sampler pragmas allow you to configure the sampling mode of a sampler binding.


```glsl
// Sampler pragmas must be of the form
// #pragma sampler(name='foo', linear|nearest, <clamp|repeat|mirror>)
#pragma sampler(name='linear_sampler', linear, mirror)
layout(set=1, binding=1) uniform sampler linear_sampler;

```



### Utility Blocks

The Tweak Shader Library allows you to utilize utility blocks to access specific uniform or push constant fields efficiently. Here's an example of how to use utility blocks:

 ```glsl
#pragma utility_block(ShaderInputs)
layout(push_constant) uniform ShaderInputs {
    float time;       // shader playback time (in seconds)
    float time_delta; // elapsed time since the last frame in seconds
    float frame_rate; // estimated number of frames per second
    uint frame_index; // frame count
    vec4 mouse;       // ShaderToy mouse scheme
    vec4 date;        // [year, month, day, seconds]
    vec3 resolution;  // viewport resolution in pixels, [width, height, aspect ratio]
    uint pass_index;   // updated to reflect the current render pass index
};
 ```

 You can use the `#pragma utility_block` to access members from the specialized utility functions, such as [`RenderContext::update_time`] and [`RenderContext::update_resolution`]. The field names may vary between blocks safely, they are accessed by field offset.


### Additional Render Passes and Persistent Buffers

 You can define additional render passes and specify output targets for each pass using the `#pragma pass` pragma. Here's an example of how to create an additional pass:

 ```glsl
 #pragma pass(0, persistent, target="single_pixel", height=1, width=1)
layout(set=0, binding=1) uniform sampler default_sampler;
layout(set=0, binding=2) uniform texture2D single_pixel;
 ```

 The `#pragma pass` pragma allows you to add passes that run in the order specified by their index before the main pass. If a `target` is specified, the pass will write to a context-managed texture mapped to the specified variable. You can also specify custom `height` and `width` for the output texture; otherwise, it defaults to the render target's size.

### Compute Shader Specific Pragmas

Compute shaders have unique constraints and some unique pragmas for managing storage textures.

### Stage

```glsl
#pragma stage("compute")

// default
#pragma stage("fragment")

```
The stage pragma can be used to indicate to the context that this is a compute shader, this will not be deduced on it's own. 


### Targets

The target pragma creates a storage texture which can be written into. The availability of readable storage textures is not guaranteed, if you 
enabled the appropriate flags during device setup then you should be able to assign targets as `readwrite` as well as `writeonly`. Additionally, these targets will fail to validate unless the format specifier matches that of your render context. 

```glsl
// must be of the form:
// #pragma target(name="foo", <persistent>, <width=k>, <height=k>)

#pragma target(name="output_image")
layout(rgba8, set=0, binding=1) uniform writeonly image2D output_image;
```

### Relays

Because targets are limited to write only on the web, we need to use multiple passes to 
read from them, this is done by copying from write only textures into normal textures after every pass. To do this we have included the `relay` pragma.

```glsl
// Relays must be of the form
// #pragma relay(name="foo", target="bar" <persistent>, <width=k>, <height=k>)

// this means that in compute shaders, specifying persistence 
// and size of a renderpass is illegal.
#pragma pass(0)

#pragma relay(name="relay", target="relay_target")
layout(rgba8, set=0, binding=1) uniform writeonly image2D relay;
layout(set=0, binding=2) uniform texture2D relay_target;
```
