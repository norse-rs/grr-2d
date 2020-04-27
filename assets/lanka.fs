#version 450 core

#define GRR 1

const float FLOAT_MAX = 3.402823466e+38;

const uint PRIMITIVE_LINE = 0x1; // distance field generation (linear curve)
const uint PRIMITIVE_QUADRATIC = 0x2; // quadratic curve
const uint PRIMITIVE_CIRCLE = 0x3;
const uint PRIMITIVE_ARC = 0x4;
const uint PRIMITIVE_RECT = 0x5;
const uint PRIMITIVE_SHADOW_RECT = 0x6;

const uint PRIMITIVE_FILL_COLOR = 0x10;
const uint PRIMITIVE_FILL_LINEAR_GRADIENT = 0x11;

#if GRR
layout (location = 0) uniform uint u_num_primitives;
layout (location = 1) uniform vec4 u_viewport;
layout (location = 2) uniform vec2 u_screen_dim;
# else
layout(binding = 2) uniform Locals {
    vec4 u_viewport;
    vec2 u_screen_dim;
    uint u_num_primitives;
};
#endif

layout (location = 0) in vec2 f_pos_curve;
layout (location = 1) flat in uvec3 f_curve_range;
layout (location = 2) in vec2 f_pos_world;

layout (binding = 0, std430) readonly buffer SceneVertices {
    uint vertices[];
};
layout (binding = 1, std430) readonly buffer ScenePrimitives {
    uint primitives[];
};

float line_eval(float p0, float p1, float t) {
    return mix(p0, p1, t);
}

float line_raycast(float p0, float p1, float p) {
   return (p - p0) / (p1 - p0);
}

float quad_eval(float p0, float p1, float p2, float t) {
    return (1.0 - t) * (1.0 - t) * p0 + 2.0 * t * (1.0 - t) * p1 + t * t * p2;
}

float quad_raycast(float p0, float p1, float p2, float t) {
    const float a = p0 - 2.0 * p1 + p2;

    if (abs(a) < 0.0001) {
        return line_raycast(p0, p2, t);
    }

    const float b = p0 - p1;
    const float c = p0 - t;
    const float dscr_sq = b * b - a * c;
    const float sign = int(p2 > t) - int(p0 > t);

    return (b + float(sign) * sqrt(dscr_sq)) / a;
}

float erf(float x) {
    const float s = sign(x);
    const float a = abs(x);
    x = 1.0 + (0.278393 + (0.230389 + 0.078108 * (a * a)) * a) * a;
    x *= x;
    return s - s / (x * x);
}

// float cdf(float x, float ddx) {
//     return smoothstep(-0.8, 0.8, x * ddx);
// }

// approx trapezoid area
float cdf(float x, float m) {
    return clamp(x*m + 0.5, 0.0, 1.0);
}

layout(location = 0) out vec4 o_frag;

void main() {
    const vec2 tile_center = f_pos_curve;

    vec2 dxdy = fwidth(tile_center);
    const vec2 unit = 1.0 / dxdy;

    vec4 color = vec4(1.0, 0.0, 0.0, 1.0);
    float coverage = 0.0;
    float shadow = 0.0;

    uint base_vertex = f_curve_range.x;
    for (uint i = f_curve_range.y; i < f_curve_range.z; i++) {
        const uint primitive = primitives[i];
        switch (primitive) {
        case PRIMITIVE_LINE: {
            const vec2 p0 = unpackHalf2x16(vertices[base_vertex++]) - tile_center;
            const vec2 p1 = unpackHalf2x16(vertices[base_vertex++]) - tile_center;

            if (max(p0.y, p1.y) < -0.5 * dxdy.y) {
                break;
            }

            const float xx0 = clamp(p0.x, -0.5 * dxdy.x, 0.5 * dxdy.x);
            const float xx1 = clamp(p1.x, -0.5 * dxdy.x, 0.5 * dxdy.x);
            const float xx = (xx1 - xx0) * unit.x;

            float cy = 1.0;
            if (xx != 0.0 && min(p0.y, p1.y) < 0.5 * dxdy.y) {
                const float t = line_raycast(p0.x, p1.x, 0.5 * (xx0 + xx1)); // raycast y direction at sample pos
                const float d = line_eval(p0.y, p1.y, t) * unit.y; // get x value at ray intersection
                const vec2 tangent = abs(p1 - p0);
                const float m = tangent.x / max(tangent.x, tangent.y);
                cy = cdf(d, m);
            }

            coverage += cy * xx;
        } break;
        case PRIMITIVE_QUADRATIC: {
            const vec2 p0 = unpackHalf2x16(vertices[base_vertex++]) - tile_center;
            const vec2 p1 = unpackHalf2x16(vertices[base_vertex++]) - tile_center;
            const vec2 p2 = unpackHalf2x16(vertices[base_vertex++]) - tile_center;

            if (max(p0.y, p2.y) < -0.5 * dxdy.y) {
                break;
            }

            const float xx0 = clamp(p0.x, -0.5 * dxdy.x, 0.5 * dxdy.x);
            const float xx1 = clamp(p2.x, -0.5 * dxdy.x, 0.5 * dxdy.x);
            const float xx = (xx1 - xx0) * unit.x;

            float cy = 1.0;
            if (xx != 0.0 && min(p0.y, p2.y) < 0.5 * dxdy.y) {
                const float t = quad_raycast(p0.x, p1.x, p2.x, 0.5 * (xx0 + xx1)); // raycast y direction at sample pos
                const float d = quad_eval(p0.y, p1.y, p2.y, t) * unit.y; // get x value at ray intersection
                const vec2 tangent = abs(mix(p1 - p0, p2 - p1, t));
                const float m = tangent.x / max(tangent.x, tangent.y);
                cy = cdf(d, m);
            }

            coverage += cy * xx;
        } break;

        case PRIMITIVE_CIRCLE: {
            const vec2 center = unpackHalf2x16(vertices[base_vertex++]) - tile_center;
            const float radius = uintBitsToFloat(vertices[base_vertex++]); // I'm lazy ..

            const float xx0 = clamp(center.x - radius, -0.5 * dxdy.x, 0.5 * dxdy.x);
            const float xx1 = clamp(center.x + radius, -0.5 * dxdy.x, 0.5 * dxdy.x);
            const float xx = (xx1 - xx0) * unit.x;

            if (xx == 0.0) {
                break;
            }

            if ((center.y + radius > -0.5 * dxdy.y) && (center.y - radius < 0.5 * dxdy.y)) {
                const float dx = 0.5 * (xx0 + xx1) - center.x;
                const float dy = sqrt(radius * radius - dx * dx);
                const float ddy = abs(dy) / radius;
                const float dy0 = (center.y - dy) * unit.y;
                const float dy1 = (center.y + dy) * unit.y;

                coverage -= xx * cdf(dy0, ddy); // TODO
                coverage += xx * cdf(dy1, ddy); // TODO
            }
        } break;

        case PRIMITIVE_ARC: {
            const vec2 center = unpackHalf2x16(vertices[base_vertex++]) - tile_center;
            const vec2 d0 = unpackHalf2x16(vertices[base_vertex++]);
            const vec2 d1 = unpackHalf2x16(vertices[base_vertex++]);
            const vec2 p0 = center + d0;
            const vec2 p1 = center + d1;

            const float xx0 = clamp(p0.x, -0.5 * dxdy.x, 0.5 * dxdy.x);
            const float xx1 = clamp(p1.x, -0.5 * dxdy.x, 0.5 * dxdy.x);
            const float xx = (xx1 - xx0) * unit.x;

            if (xx == 0.0) {
                break;
            }

            float cy = 0.0;
            if (max(p0.y, p1.y) > -0.5 * dxdy.y) {
                if (xx != 0.0 && min(p0.y, p1.y) < 0.5 * dxdy.y) {
                    const float diry = d0.y + d1.y;
                    const float sign = int(diry > center.y) - int(center.y > diry);

                    const float dx = 0.5 * (xx0 + xx1) - center.x;
                    const float dy = sqrt(dot(d0, d0) - dx * dx);
                    const float d = (center.y + sign * dy) * unit.y;
                    const float ddy = abs(dy) / length(d0); // todo
                    cy = cdf(d, ddy);
                } else {
                    cy = 1.0;
                }
            }

            coverage += cy * xx;
        } break;

        case PRIMITIVE_RECT: {
            const vec2 p0 = unpackHalf2x16(vertices[base_vertex++]) - tile_center;
            const vec2 p1 = unpackHalf2x16(vertices[base_vertex++]) - tile_center;

            const float xx0 = clamp(p0.x, -0.5 * dxdy.x, 0.5 * dxdy.x);
            const float xx1 = clamp(p1.x, -0.5 * dxdy.x, 0.5 * dxdy.x);
            const float xx = (xx1 - xx0) * unit.x;

            const float dy0 = p0.y * unit.y;
            const float dy1 = p1.y * unit.y;

            coverage -= xx * cdf(dy0, 1.0);
            coverage += xx * cdf(dy1, 1.0);
        } break;

        case PRIMITIVE_SHADOW_RECT: {
            const vec2 p0 = unpackHalf2x16(vertices[base_vertex++]) - tile_center;
            const vec2 p1 = unpackHalf2x16(vertices[base_vertex++]) - tile_center;
            const float sigma = uintBitsToFloat(vertices[base_vertex++]);

            const float norm = sqrt(0.5) / sigma;
            const float sy = 0.5 * (erf(p1.y * norm) - erf(p0.y * norm));
            const float sx = 0.5 * (erf(p1.x * norm) - erf(p0.x * norm));

            coverage += sy * sx;
        } break;

        case PRIMITIVE_FILL_COLOR: {
            const vec4 brush = unpackUnorm4x8(vertices[base_vertex++]);
            color.rgb = brush.rgb;

            color.a = clamp(coverage, 0.0, 1.0);
            coverage = 0.0;
        } break;

        case PRIMITIVE_FILL_LINEAR_GRADIENT: {
            const vec2 p0 = unpackHalf2x16(vertices[base_vertex++]);
            const vec4 c0 = unpackUnorm4x8(vertices[base_vertex++]);
            const vec2 p1 = unpackHalf2x16(vertices[base_vertex++]);
            const vec4 c1 = unpackUnorm4x8(vertices[base_vertex++]);

            const vec2 dir = p1 - p0;
            const float t = clamp(dot(normalize(dir), f_pos_world - p0) / length(dir), 0.0, 1.0);
            color.rgb = mix(c0, c1, t).rgb;

            color.a = clamp(coverage, 0.0, 1.0);
            coverage = 0.0;
        } break;
        }
    }

    o_frag = color; // vec4(shadow, shadow, shadow, 1.0);
}
