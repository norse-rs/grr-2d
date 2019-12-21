#version 450 core

const float FLOAT_MAX = 3.402823466e+38;

const uint PRIMITIVE_LINE = 0x1; // distance field generation (linear curve)
const uint PRIMITIVE_STROKE = 0x2; // distance field stroke shading
const uint PRIMITIVE_QUADRATIC = 0x3; // quadratic curve
const uint PRIMITIVE_FILL = 0x4; // fill shading
const uint PRIMTIIVE_QUADRATIC_MONO = 0x5; // monotonic quadratic curve

const uint QUADRATIC_LUT = 0x535ACA0;

layout (location = 0) uniform uint u_num_primitives;
layout (location = 1) uniform vec4 u_viewport;
layout (location = 2) uniform vec2 u_screen_dim;

layout (location = 0) in vec2 f_pos_curve;

layout (binding = 0, std430) readonly buffer SceneVertices {
    vec2 vertices[];
};
layout (binding = 1, std430) readonly buffer ScenePrimitives {
    uint primitives[];
};

out vec4 o_frag;

float min4(vec4 v) {
    return min(min(v.x, v.y), min(v.z, v.w));
}

float max4(vec4 v) {
    return max(max(v.x, v.y), max(v.z, v.w));
}

float filtering(float x, float lower, float upper)
{
    return smoothstep(-1.0, 1.0, x);
    // return clamp(x, 0.0, 1.0);
}

void main() {
    const vec2 tile_center = f_pos_curve;
    const vec2 tile_extent = fwidth(tile_center);
    const float unit = 1.0 / tile_extent.y;

    vec4 color = vec4(0.0, 0.0, 0.0, 0.0);
    float stroke_df = FLOAT_MAX;
    vec2 winding = vec2(0.0);

    uint base_vertex = 0;
    for (uint i = 0; i < u_num_primitives; i++) {
        const uint primitive = primitives[i];
        switch (primitive) {
        case PRIMITIVE_LINE: {
            // line distance field
            const vec2 p0 = vertices[base_vertex++] - tile_center;
            const vec2 p1 = vertices[base_vertex++] - tile_center;

            // intersection check
            const bvec4 bounds = greaterThan(vec4(p0.x, p1.x, p0.y, p1.y), vec4(0.0));
            const float kx = mix(p0.x, p1.x, (0.0 - p0.y) / (p1.y - p0.y));
            const float ky = mix(p0.y, p1.y, (0.0 - p0.x) / (p1.x - p0.x));

            {
                if (bounds.z && !bounds.w) { // moving down
                    winding.x += filtering(kx * unit, 0.0, 1.0);
                } else if (!bounds.z && bounds.w) { // moving up
                    winding.x -= filtering(kx * unit, 0.0, 1.0);
                }
            }
            {
                if (bounds.x && !bounds.y) {
                    winding.y -= filtering(ky * unit, 0.0, 1.0);
                } else if (!bounds.x && bounds.y) {
                    winding.y += filtering(ky * unit, 0.0, 1.0);
                }
            }

            // winding += local_winding * 0.5;
            // stroke_df = min(stroke_df, field);
        } break;
        case PRIMITIVE_STROKE: {
            // stroke shading
            float alpha = min(winding.x, winding.y);
            color.rgb = sqrt(vec3(1.0 - alpha));
            color.a = alpha;

            // stroke_df = FLOAT_MAX;
            winding = vec2(0.0);
        } break;

        case PRIMTIIVE_QUADRATIC_MONO: {
            const vec2 p0 = vertices[base_vertex++] - tile_center;
            const vec2 p1 = vertices[base_vertex++] - tile_center;
            const vec2 p2 = vertices[base_vertex++] - tile_center;

            // intersection check
            const bvec4 bounds = greaterThan(vec4(p0.x, p2.x, p0.y, p2.y), vec4(0.0));

            {
                const vec2 a = p0 - 2 * p1 + p2;
                const vec2 b = p0 - p1;
                const vec2 c = p0;

                const vec2 dscr = sqrt(max(b * b - a * c, 0.0));
                const vec2 t0 = (b - dscr) / a;
                const vec2 t1 = (b + dscr) / a;

                const vec2 ty = vec2(t0.x, t1.x);
                const vec2 tx = vec2(t0.y, t1.y);
                const vec2 x = (1 - tx) * (1 - tx) * p0.x + 2.0 * (1 - tx) * tx * p1.x + tx * tx * p2.x;
                const vec2 y = (1 - ty) * (1 - ty) * p0.y + 2.0 * (1 - ty) * ty * p1.y + ty * ty * p2.y;

                // const vec2 dxy = 2 * (-p0 * (1 - tx.y) - 2.0 * p1 * tx.y  + p1 +  tx.y * p2);
                // const vec2 dxx = 2 * (-p0 * (1 - tx.x) - 2.0 * p1 * tx.x  + p1 +  tx.x * p2);
                // const vec2 dyy = 2 * (-p0 * (1 - ty.y) - 2.0 * p1 * ty.y  + p1 +  ty.y * p2);
                // const vec2 dyx = 2 * (-p0 * (1 - ty.x) - 2.0 * p1 * ty.x  + p1 +  ty.x * p2);

                // float m = 1.0;

                if (bounds.z && !bounds.w) { // moving down
                    // if (abs(dxx.x) > 0.001) {
                    //     m = abs(dxx.y) / abs(dxx.x);
                    // }
                    winding.x += filtering(x.x * unit, 0.0, 1.0);
                } else if (!bounds.z && bounds.w) { // moving up
                    // if (abs(dxy.x) > 0.001) {
                    //     m = abs(dxy.y) / abs(dxy.x);
                    // }
                    winding.x -= filtering(x.y * unit, 0.0, 1.0);
                }

                if (bounds.x && !bounds.y) { // moving to left
                    // if (abs(dyx.y) > 0.0 && abs(dyy.x) > 0.0) {
                    //     m = abs(dyx.x) / abs(dyx.y);
                    // }
                    winding.y -= filtering(y.x * unit, 0.0, 1.0);
                } else if (!bounds.x && bounds.y) { // moving to right
                    // if (abs(dyy.y) > 0.0 && abs(dyy.x) > 0.0) {
                    //     m = abs(dyy.x) / abs(dyy.y);
                    // }
                    winding.y += filtering(y.y * unit, 0.0, 1.0);
                }
            }
        } break;
        }
    }

    o_frag = color;
}
