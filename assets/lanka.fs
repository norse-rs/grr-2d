#version 450 core

const float FLOAT_MAX = 3.402823466e+38;

const uint PRIMITIVE_LINE = 0x1; // distance field generation (linear curve)
const uint PRIMITIVE_QUADRATIC = 0x2; // quadratic curve
const uint PRIMITIVE_CIRCLE = 0x3;

const uint PRIMITIVE_FILL = 0x4;

layout (location = 0) uniform uint u_num_primitives;
layout (location = 1) uniform vec4 u_viewport;
layout (location = 2) uniform vec2 u_screen_dim;

layout (location = 0) in vec2 f_pos_curve;
layout (location = 1) flat in uvec3 f_curve_range;

layout (binding = 0, std430) readonly buffer SceneVertices {
    vec2 vertices[];
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
    if (t <= min(p0, p2) || t >= max(p0, p2)) {
        return line_raycast(p0, p2, t);
    }

    const float sign = int(p2 > t) - int(p0 > t);

    const float a = p0 - 2.0 * p1 + p2;
    const float b = p0 - p1;
    const float c = p0 - t;

    const float dscr_sq = b * b - a * c;
    if (abs(a) < 0.0001) {
        return line_raycast(p0, p2, t);
    } else {
        return (b + float(sign) * sqrt(dscr_sq)) / a;
    }
}

float cdf(float x) {
    return smoothstep(-0.8, 0.8, x);
}

out vec4 o_frag;

void main() {
    const vec2 tile_center = f_pos_curve;
    const vec2 dxdy = fwidth(tile_center);
    const vec2 unit = 1.0 / dxdy;

    const vec2 tile_min = f_pos_curve - 0.5 * dxdy;
    const vec2 tile_max = f_pos_curve + 0.5 * dxdy;

    vec4 color = vec4(1.0, 0.0, 0.0, 1.0);
    float coverage = 0.0;

    uint base_vertex = f_curve_range.x;
    for (uint i = f_curve_range.y; i < f_curve_range.z; i++) {
        const uint primitive = primitives[i];
        switch (primitive) {
        case PRIMITIVE_LINE: {
            const vec2 p0 = vertices[base_vertex++] - tile_center;
            const vec2 p1 = vertices[base_vertex++] - tile_center;

            const float xx0 = clamp(p0.x, -0.5 * dxdy.x, 0.5 * dxdy.x);
            const float xx1 = clamp(p1.x, -0.5 * dxdy.x, 0.5 * dxdy.x);
            const float xx = (xx1 - xx0) * unit.x;

            float cy = 0.0;
            if (max(p0.y, p1.y) > -0.5 * dxdy.y) {
                if (xx != 0.0 && min(p0.y, p1.y) < 0.5 * dxdy.y) {
                    const float t = line_raycast(p0.x, p1.x, 0.5 * (xx0 + xx1)); // raycast y direction at sample pos
                    const float d = line_eval(p0.y, p1.y, t) * unit.y; // get x value at ray intersection
                    const vec2 tangent = p1 - p0;
                    const float f = d * abs(tangent.x) / length(tangent);
                    cy = cdf(f);
                } else {
                    cy = 1.0;
                }
            }

            coverage += cy * xx;
        } break;
        case PRIMITIVE_QUADRATIC: {
            const vec2 p0 = vertices[base_vertex++] - tile_center;
            const vec2 p1 = vertices[base_vertex++] - tile_center;
            const vec2 p2 = vertices[base_vertex++] - tile_center;

            const float xx0 = clamp(p0.x, -0.5 * dxdy.x, 0.5 * dxdy.x);
            const float xx1 = clamp(p2.x, -0.5 * dxdy.x, 0.5 * dxdy.x);
            const float xx = (xx1 - xx0) * unit.x;

            float cy = 0.0;
            if (max(p0.y, p2.y) > -0.5 * dxdy.y) {
                if (xx != 0.0 && min(p0.y, p2.y) < 0.5 * dxdy.y) {
                    const float t = quad_raycast(p0.x, p1.x, p2.x, 0.5 * (xx0 + xx1)); // raycast y direction at sample pos
                    const float d = quad_eval(p0.y, p1.y, p2.y, t) * unit.y; // get x value at ray intersection
                    const vec2 tangent = mix(p1 - p0, p2 - p1, t);
                    const float f = d * abs(tangent.x) / length(tangent);
                    cy = cdf(f);
                } else {
                    cy = 1.0;
                }
            }

            coverage += cy * xx;
        } break;

        case PRIMITIVE_CIRCLE: {
            const vec2 center = vertices[base_vertex++] - tile_center;
            const float radius = vertices[base_vertex++].x; // I'm lazy ..

            const float xx0 = clamp(center.x - radius, -0.5 * dxdy.x, 0.5 * dxdy.x);
            const float xx1 = clamp(center.x + radius, -0.5 * dxdy.x, 0.5 * dxdy.x);
            const float xx = (xx1 - xx0) * unit.x;

            if (xx == 0.0) {
                continue;
            }

            if ((center.y + radius > -0.5 * dxdy.y) && (center.y - radius < 0.5 * dxdy.y)) {
                const float dx = 0.5 * (xx0 + xx1) - center.x;
                const float dy = sqrt(radius * radius - dx * dx);
                const float dy0 = (center.y - dy) * unit.y * abs(dy) / radius;
                const float dy1 = (center.y + dy) * unit.y * abs(dy) / radius;

                coverage -= xx * cdf(dy0);
                coverage += xx * cdf(dy1);
            }
        } break;

        case PRIMITIVE_FILL: {
            color.rgb = vec3(0.0);
            color.a = clamp(coverage, 0.0, 1.0);

            coverage = 0.0;
        } break;
        }
    }

    o_frag = color;
}
