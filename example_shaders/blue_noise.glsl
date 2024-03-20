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

layout(location = 0) out vec4 out_color; 

#pragma input(image, name="input_image")
layout(set=0, binding=1) uniform sampler default_sampler;
layout(set=0, binding=2) uniform texture2D input_image;

#pragma input(image, name="depth_tex")
layout(set=0, binding=3) uniform texture2D depth_tex;

#pragma input(float, name=threshold, default=0.0001, min=-0.9, max=0.9)
#pragma input(float, name=seed1, default=1.705, min=0.01, max=32.0)
#pragma input(float, name=width, default=1.0, min=0.01, max=32.0)
#pragma input(float, name=dmul, default=8.12235325, min=0.01, max=20.0)
layout(set = 0, binding = 4) uniform custom_inputs {
    float threshold;
    float seed1;
    float dmul;
    float width;
};

//checkerboard noise
vec2 stepnoise(vec2 p, float size) {
    p += 10.0;
    float x = floor(p.x/size)*size;
    float y = floor(p.y/size)*size;
    
    x = fract(x*0.1) + 1.0 + x*0.0002;
    y = fract(y*0.1) + 1.0 + y*0.0003;
    
    float a = fract(1.0 / (0.000001*x*y + 0.00001));
    a = fract(1.0 / (0.000001234*a + 0.00001));
    
    float b = fract(1.0 / (0.000002*(x*y+x) + 0.00001));
    b = fract(1.0 / (0.0000235*b + 0.00001));
    
    return vec2(a, b);
    
}

float poly(float a, float b, float c, float ta, float tb, float tc) {
    return (a*ta + b*tb + c*tc) / (ta+tb+tc);
}


float mask(vec2 p, float block_fac) {
    vec2 r = stepnoise(p, 5.5)-0.5;
    p[0] += r[0]*dmul;
    p[1] += r[1]*dmul;
    
    float f = fract(p[0]*seed1 + p[1]/(seed1+0.15555))*1.03;
    return poly(pow(f, 150.0), f*f, f, 1.0, 0.0, 1.3);
}

float s(float x, float y, vec2 uv) {
    vec4 clr = texture(sampler2D(input_image, default_sampler), vec2(x, y)/resolution.xy + uv);
    float f = clr[0]*0.3 + clr[1]*0.6 + clr[1]*0.1;
    
    return f;
}

// luminosity 
float lum(vec3 color) {
    return dot(color, vec3(0.2126, 0.7152, 0.0722));
}

mat3 mynormalize(mat3 mat) {
    float sum = mat[0][0]+mat[0][1]+mat[0][2]
              + mat[1][0]+mat[1][1]+mat[1][2]
              + mat[2][0]+mat[2][1]+mat[2][2];
    return mat / sum;
}
void main()
{
	vec2 uv = gl_FragCoord.xy;
	
    vec4 depth = texture(sampler2D(depth_tex, default_sampler), gl_FragCoord.xy / resolution.xy);
    float lum = lum(depth.rgb); 
    float scale = floor(10.0 * lum) + 1.0;

   
    //sharpen input.  this is necassary for stochastic
    //ordered dither methods.
    vec2 uv3 =  gl_FragCoord.xy / resolution.xy;
    float d = 0.5;
    mat3 mat = mat3(
        vec3(d, d,   d),
        vec3(d, 2.0, d),
        vec3(d, d,   d)
    );
    
    float f1 = s(0.0, 0.0, uv3);
    
    mat = mynormalize(mat);
    float f = s(-1.0, -1.0, uv3)*mat[0][0] + s(-1.0, 0.0, uv3)*mat[0][1] + s(-1.0, 1.0, uv3)*mat[0][2]
      + s( 0.0, -1.0, uv3)*mat[1][0] + s( 0.0, 0.0, uv3)*mat[1][1] + s( 0.0, 1.0, uv3)*mat[1][2]
      + s( 1.0, -1.0, uv3)*mat[2][0] + s( 1.0, 0.0, uv3)*mat[2][1] + s( 1.0, 1.0, uv3)*mat[2][2];
    
    f = (f-s(0.0, 0.0, uv3));
    f *= 40.0;
    f = f1 - f;
    
    float c = mask(uv, scale);
    
	  c = float(f >= c + threshold);
    
    
	out_color = vec4(c, c, c, 1.0);
}
