#version 450

#pragma utility_block(ShaderInputs)
layout(set = 0, binding = 0) uniform ShaderInputs {
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

const float TAU = 6.283185307179586;
const float aRatio = 4.0 / 3.0;

const vec2 cNumber = vec2(26., 20.);
const vec2 cSize = vec2(15., 15.);

vec3 colorBars(float x) {
    return step(.5, fract(vec3(1. - x) * vec3(2., 1., 4.)));
}

vec3 checkerboard(vec2 p) {
    return vec3(mod((p.x + p.y), 2.));
}

vec3 cellFrame(vec2 p, vec3 bg) {
    return (cSize.x - p.x) <= 1. || (cSize.y - p.y) <= 1. ? vec3(.9) : bg;
}

bool rectangle(vec2 p, vec2 size) {
    return 0. <= p.x &&
        0. <= p.y &&
        p.x < size.x &&
        p.y < size.y && (p.x < 1. ||
        p.y < 1. ||
        (size.x - 1.) <= p.x ||
        (size.y - 1.) <= p.y);
}

vec3 comb(float freq, float t) {
    return vec3((sin(freq * t * TAU) + 1.) * .45);
}

bool circle(vec2 pos, float r) {
    return length(pos) <= r;
}

vec3 letter(int c, vec2 pos, vec3 color, vec3 bg) {
    // Limited KOI7 character set
    int x = int(pos.x) - 2;
    int y = int(pos.y) - 3;
    if(x < 0 || x >= 4 ||
        y < 0 || y >= 8)
        return bg;
    mat4 tile = c == 0x30 ? (y < 4 ? mat4(0., 0., 0., 0., 0., 1., 1., 0., 1., 0., 0., 1., 1., 0., 1., 1.) : mat4(1., 1., 0., 1., 1., 0., 0., 1., 0., 1., 1., 0., 0., 0., 0., 0.)) : c == 0x31 ? (y < 4 ? mat4(0., 0., 0., 0., 0., 0., 1., 0., 0., 1., 1., 0., 0., 0., 1., 0.) : mat4(0., 0., 1., 0., 0., 0., 1., 0., 0., 1., 1., 1., 0., 0., 0., 0.)) : c == 0x32 ? (y < 4 ? mat4(0., 0., 0., 0., 0., 1., 1., 0., 1., 0., 0., 1., 0., 0., 0., 1.) : mat4(0., 0., 1., 0., 0., 1., 0., 0., 1., 1., 1., 1., 0., 0., 0., 0.)) : c == 0x33 ? (y < 4 ? mat4(0., 0., 0., 0., 1., 1., 1., 1., 0., 0., 1., 0., 0., 0., 1., 0.) : mat4(0., 0., 0., 1., 1., 0., 0., 1., 0., 1., 1., 0., 0., 0., 0., 0.)) : c == 0x34 ? (y < 4 ? mat4(0., 0., 0., 0., 0., 0., 1., 0., 0., 1., 1., 0., 1., 0., 1., 0.) : mat4(1., 1., 1., 1., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 0., 0.)) : c == 0x35 ? (y < 4 ? mat4(0., 0., 0., 0., 1., 1., 1., 1., 1., 0., 0., 0., 1., 1., 1., 0.) : mat4(0., 0., 0., 1., 1., 0., 0., 1., 0., 1., 1., 0., 0., 0., 0., 0.)) : c == 0x36 ? (y < 4 ? mat4(0., 0., 0., 0., 0., 0., 1., 0., 0., 1., 0., 0., 1., 1., 1., 0.) : mat4(1., 0., 0., 1., 1., 0., 0., 1., 0., 1., 1., 0., 0., 0., 0., 0.)) : c == 0x37 ? (y < 4 ? mat4(0., 0., 0., 0., 1., 1., 1., 1., 0., 0., 0., 1., 0., 0., 0., 1.) : mat4(0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 0., 0., 0., 0., 0.)) : c == 0x38 ? (y < 4 ? mat4(0., 0., 0., 0., 0., 1., 1., 0., 1., 0., 0., 1., 0., 1., 1., 0.) : mat4(1., 0., 0., 1., 1., 0., 0., 1., 0., 1., 1., 0., 0., 0., 0., 0.)) : c == 0x39 ? (y < 4 ? mat4(0., 0., 0., 0., 0., 1., 1., 0., 1., 0., 0., 1., 1., 0., 0., 1.) : mat4(0., 1., 1., 1., 0., 0., 0., 1., 0., 1., 1., 0., 0., 0., 0., 0.)) : c == 0x63 ? (y < 4 ? mat4(0., 0., 0., 0., 1., 0., 1., 0., 1., 0., 1., 0., 1., 0., 1., 0.) : mat4(1., 0., 1., 0., 1., 0., 1., 0., 1., 0., 1., 0., 1., 1., 1., 1.)) : c == 0x67 ? (y < 4 ? mat4(0., 0., 0., 0., 1., 1., 1., 0., 1., 0., 0., 0., 1., 0., 0., 0.) : mat4(1., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0.)) : c == 0x72 ? (y < 4 ? mat4(0., 0., 0., 0., 1., 1., 1., 0., 1., 0., 1., 0., 1., 0., 1., 0.) : mat4(1., 1., 1., 0., 1., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0.)) : c == 0x74 ? (y < 4 ? mat4(0., 0., 0., 0., 1., 1., 1., 0., 0., 1., 0., 0., 0., 1., 0., 0.) : mat4(0., 1., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0.)) : mat4(0.);
    y = y - y / 4 * 4;
    vec4 row = (y == 0 ? tile[0] : y == 1 ? tile[1] : y == 2 ? tile[2] : y == 3 ? tile[3] : vec4(0.));
    float cell = (x == 0 ? row[0] : x == 1 ? row[1] : x == 2 ? row[2] : x == 3 ? row[3] : 0.);
    return mix(bg, color, cell);
}

vec3 clock() {
    float t = date.q;
    float s = mod(t, 60.);
    t = floor(t / 60.);
    float m = mod(t, 60.);
    t = floor(t / 60.);
    float h = mod(t, 24.);
    return vec3(h, m, s);
}

int digitBase10(int n, float x) {
    int d = int(x);
    if(n == 1)
        d /= 10;
    return 0x30 + (d - d / 10 * 10);
}

vec3 ueit(vec2 uv) {
    uv = (uv - vec2(.5, .5)) * (576. / 600.) + vec2(.5, .5);
    if(abs(uv.x - .5) > .5 || abs(uv.y - .5) > .5)
        return vec3(0.);
    vec2 pcc = uv * cNumber;
    vec2 ppc = pcc * cSize;
    vec2 pc = floor(ppc);
    float ht = uv.x * .8333 * 0.000064;
    vec2 pcpc = mod(ppc, cSize);
    vec2 cpc = mod(pc, cSize);
    vec2 cc = floor(pcc);
    vec2 iuv = (pcc - 1.) / (cNumber - 2.);
    bool bc = circle(uv * cNumber - cNumber * .5, 8. - .5 / 15.);
    bool sctl = circle(pcc - vec2(4., 3.), 2. - .5 / 15.);
    bool sctr = circle(pcc - vec2(22., 3.), 2. - .5 / 15.);
    bool scbl = circle(pcc - vec2(4., 17.), 2. - .5 / 15.);
    bool scbr = circle(pcc - vec2(22., 17.), 2. - .5 / 15.);
    vec3 clockTime = clock();
    return rectangle(cc, cNumber) ? (rectangle(pc - cSize + 1., cSize * (cNumber - 2.0)) ? .9 * vec3(.9) : checkerboard(cc)) : .9 * (sctl ? (rectangle(cc - vec2(3., 2.), vec2(2.)) ? (cc.y == 1. && cpc.y == 14. ? comb(3e6, ht) : cc.y == 2. && cpc.y <= 8. ? comb(3e6, ht) : cc.y == 3. && cpc.y == 14. ? vec3(.9) : cc.y == 3. && cpc.y >= 5. ? comb(4e6, ht) : cellFrame(cpc, vec3(.5))) : cc.y == 2. && cc.x == 2. ? letter(0x33, cpc - vec2(8., .0), vec3(.0), vec3(.9)) : cc.y == 3. && cc.x == 2. ? letter(0x34, cpc - vec2(8., .0), vec3(.0), vec3(.9)) : vec3(.9)) : sctr ? (rectangle(cc - vec2(21., 2.), vec2(2.)) ? (cc.y == 1. && cpc.y == 14. ? comb(3e6, ht) : cc.y == 2. && cpc.y <= 8. ? comb(3e6, ht) : cc.y == 3. && cpc.y == 14. ? vec3(.9) : cc.y == 3. && cpc.y >= 5. ? comb(4e6, ht) : cellFrame(cpc, vec3(.5))) : cc.y == 2. && cc.x == 20. ? letter(0x33, cpc - vec2(8., .0), vec3(.0), vec3(.9)) : cc.y == 3. && cc.x == 20. ? letter(0x34, cpc - vec2(8., .0), vec3(.0), vec3(.9)) : vec3(.9)) : scbl ? (rectangle(cc - vec2(3., 16.), vec2(2.)) ? (cc.y == 15. && cpc.y == 14. ? comb(4e6, ht) : cc.y == 16. && cpc.y <= 8. ? comb(4e6, ht) : cc.y == 17. && cpc.y == 14. ? vec3(.9) : cc.y == 17. && cpc.y >= 5. ? comb(3e6, ht) : cellFrame(cpc, vec3(.5))) : cc.y == 16. && cc.x == 2. ? letter(0x34, cpc - vec2(8., .0), vec3(.0), vec3(.9)) : cc.y == 17. && cc.x == 2. ? letter(0x33, cpc - vec2(8., .0), vec3(.0), vec3(.9)) : vec3(.9)) : scbr ? (rectangle(cc - vec2(21., 16.), vec2(2.)) ? (cc.y == 15. && cpc.y == 14. ? comb(4e6, ht) : cc.y == 16. && cpc.y <= 8. ? comb(4e6, ht) : cc.y == 17. && cpc.y == 14. ? vec3(.9) : cc.y == 17. && cpc.y >= 5. ? comb(3e6, ht) : cellFrame(cpc, vec3(.5))) : cc.y == 16. && cc.x == 20. ? letter(0x34, cpc - vec2(8., .0), vec3(.0), vec3(.9)) : cc.y == 17. && cc.x == 20. ? letter(0x33, cpc - vec2(8., .0), vec3(.0), vec3(.9)) : vec3(.9)) : (((cc.y == 2. || cc.y == 16.) && cpc.y >= 7.) ||
        ((cc.y == 3. || cc.y == 17.) && cpc.y <= 6.)) && abs(cc.x - 12.5) <= 2. ? cellFrame(cpc, vec3(.5)) : cc.y == 4. && abs(cc.x - 12.5) <= 5. ? (cpc.y == 14. ? vec3(.9) : cc.x == 8. ? letter(0x67, cpc - vec2(4., 0.), vec3(.9), cellFrame(cpc, vec3(.5))) : cc.x == 9. ? letter(0x63, cpc - vec2(4., 0.), vec3(.9), cellFrame(cpc, vec3(.5))) : cc.x == 10. ? letter(0x72, cpc - vec2(4., 0.), vec3(.9), cellFrame(cpc, vec3(.5))) : cc.x == 11. ? letter(0x74, cpc - vec2(4., 0.), vec3(.9), cellFrame(cpc, vec3(.5))) : cc.x == 12. ? letter(digitBase10(1, clockTime.x), cpc - vec2(7., 0.), vec3(.9), vec3(.5)) : cc.x == 13. ? letter(digitBase10(0, clockTime.x), cpc - vec2(0., 0.), vec3(.9), cellFrame(cpc, vec3(.5))) : cc.x == 14. ? letter(digitBase10(1, clockTime.y), cpc - vec2(7., 0.), vec3(.9), vec3(.5)) : cc.x == 15. ? letter(digitBase10(0, clockTime.y), cpc - vec2(0., 0.), vec3(.9), cellFrame(cpc, vec3(.5))) : cc.x == 16. ? letter(digitBase10(1, clockTime.z), cpc - vec2(7., 0.), vec3(.9), vec3(.5)) : cc.x == 17. ? letter(digitBase10(0, clockTime.z), cpc - vec2(0., 0.), vec3(.9), cellFrame(cpc, vec3(.5))) : vec3(.5)) : cc.y == 5. || cc.y == 6. ? .5 * colorBars(iuv.x) + .4 : cc.y == 7. ? (cc.x >= 2. && cc.x <= 21. ? vec3(floor(1. + (cc.x - 2.) / 2.) / 10.) : vec3(0.)) : cc.y == 12. ? ((pc.x >= 21. * cSize.x + 7. ||
        cc.x <= 4. && pc.x >= 1. * cSize.x + 6.) ? comb(2e6, ht) : (cc.x == 1. || cc.x == 21.) ? letter(0x32, cpc - vec2(-1., 0.), vec3(.9), vec3(.5)) : (cc.x <= 20. && pc.x >= 18. * cSize.x + 6. ||
        cc.x <= 7. && pc.x >= 5. * cSize.x + 6.) ? comb(3e6, ht) : (cc.x == 5. || cc.x == 18.) ? letter(0x33, cpc - vec2(-1., 0.), vec3(.9), vec3(.5)) : (cc.x <= 17. && pc.x >= 15. * cSize.x + 6. ||
        cc.x <= 10. && pc.x >= 8. * cSize.x + 6.) ? comb(4e6, ht) : (cc.x == 8. || cc.x == 15.) ? letter(0x34, cpc - vec2(-1., 0.), vec3(.9), vec3(.5)) : cc.x <= 14. && pc.x >= 11. * cSize.x + 6. ? comb(5e6, ht) : cc.x == 11. ? letter(0x35, cpc - vec2(-1., 0.), vec3(.9), vec3(.5)) : vec3(.5)) : cc.y == 13. || cc.y == 14. ? .9 * colorBars(iuv.x) : cc.y == 15. ? (cpc.y == 0. || cpc.y == 14. ? vec3(.9) : (cc.x <= 6. || cc.x >= 19.) ? (abs(cc.x - 3.5) <= 2. || abs(cc.x - 21.5) <= 2. ? vec3(.0) : vec3(.9)) : bc ? vec3(.9 * mod(cc.x, 2.)) : vec3(.9 * mod(1. + cc.x, 2.))) : bc ? (cc.y == 8. ? (cpc.y == 14. ? vec3(.9) : cc.x <= 9. ? (vec3(.4, .9, .4) + step(7.5, pcpc.x) * vec3(.5, -.5, .5)) : cc.x <= 15. ? (vec3(.4, .4, .9) + step(7.5, pcpc.x) * vec3(.5, .5, -.5)) : (vec3(.4, .9, .9) + step(7.5, pcpc.x) * vec3(.5, -.5, -.5))) : cc.y == 9. ? (cc.x == 5. && cpc.x == 8. ? vec3(.0) : cpc.y == 14. && cpc.x == 14. && mod(cc.x - 5., 2.) == 1. ? vec3(.9) : cc.x <= 9. ? (cpc.y == 14. ? vec3(.0) : vec3(.9)) : cc.x >= 16. ? (cpc.y < 14. && (abs(pcpc.y - 14.0 + (pcc.x - 16.5) * (15. / 4.5)) < .25) ? vec3(.9) : vec3(.0)) : cc.x == 11. && cpc.x == 14. ? vec3(.9) : cc.x >= 12. && cc.x <= 13. ? cellFrame(cpc, vec3(.5)) : vec3(.5)) : cc.y == 10. ? (cc.x == 5. && cpc.x == 8. ? vec3(.9) : cpc.y == 14. ? vec3(.9) : cc.x <= 9. ? (cpc.y == 14. ? vec3(.9) : (cpc.y < 14. && (abs(pcpc.y - 14.0 + (pcc.x - 5.75) * (15. / 4.5)) < .25) ? vec3(.9) : vec3(.0))) : cc.x >= 16. ? vec3(.9) : cc.x == 11. && cpc.x == 14. ? vec3(.9) : cc.x >= 12. && cc.x <= 13. ? cellFrame(cpc, vec3(.5)) : vec3(.5)) : cc.y == 11. ? mix(vec3(0., 1., 0.), vec3(1., 0., 1.), (pcc.x - 5.) / 16.) : vec3(.9)) : cellFrame(cpc, vec3(.5)));
}

void main() {
	// vec2 uv = gl_FragCoord.xy / resolution.xy;
    float scale = min(resolution.x, resolution.y * aRatio);
    vec2 uv = vec2(.5, .5) + (gl_FragCoord.xy - resolution.xy * .5) * vec2(1., aRatio) / scale;
    uv.y = 1. - uv.y;
    out_color = vec4(ueit(uv), 1.);
}
