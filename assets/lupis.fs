#version 450 core

const float FLOAT_MAX = 3.402823466e+38;

const uint PRIMITIVE_LINE = 0x1; // distance field generation (linear curve)
const uint PRIMITIVE_STROKE = 0x2; // distance field stroke shading
const uint PRIMITIVE_QUADRATIC = 0x3; // quadratic curve
const uint PRIMITIVE_FILL = 0x4; // fill shading
const uint PRIMTIIVE_QUADRATIC_MONO = 0x5; // monotonic quadratic curve

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

float filtering(float x, float lower, float upper)
{
    return smoothstep(-1.0, 1.0, x);
}

void main() {
    const vec2 tile_center = f_pos_curve;
    const vec2 tile_extent = fwidth(tile_center);
    const float unit = 1.0 / length(tile_extent);

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
            int sign_y = 0;
            if (p0.x > 0.0) sign_y -= 1;
            if (p1.x > 0.0) sign_y += 1;

            int sign_x = 0;
            if (p0.y > 0.0) sign_x -= 1;
            if (p1.y > 0.0) sign_x += 1;

            const float kx = mix(p0.x, p1.x, (0.0 - p0.y) / (p1.y - p0.y));
            const float ky = mix(p0.y, p1.y, (0.0 - p0.x) / (p1.x - p0.x));

            winding.x -= sign_x * filtering(kx * unit, 0.0, 1.0);
            winding.y += sign_y * filtering(ky * unit, 0.0, 1.0);
        } break;
        case PRIMTIIVE_QUADRATIC_MONO: {
            const vec2 p0 = vertices[base_vertex++] - tile_center;
            const vec2 p1 = vertices[base_vertex++] - tile_center;
            const vec2 p2 = vertices[base_vertex++] - tile_center;

            // intersection check
            int sign_y = 0;
            if (p0.x > 0.0) sign_y -= 1;
            if (p2.x > 0.0) sign_y += 1;

            int sign_x = 0;
            if (p0.y > 0.0) sign_x -= 1;
            if (p2.y > 0.0) sign_x += 1;

            const vec2 a = p0 - 2 * p1 + p2;
            const vec2 b = p0 - p1;
            const vec2 c = p0;

            const vec2 dscr = sqrt(b * b - a * c);
            const float tx = (b.y + sign_x * dscr.y) / a.y;
            const float ty = (b.x + sign_y * dscr.x) / a.x;

            const float kx = (1 - tx) * (1 - tx) * p0.x + 2.0 * (1 - tx) * tx * p1.x + tx * tx * p2.x;
            const float ky = (1 - ty) * (1 - ty) * p0.y + 2.0 * (1 - ty) * ty * p1.y + ty * ty * p2.y;

            winding.x -= sign_x * filtering(kx * unit, 0.0, 1.0);
            winding.y += sign_y * filtering(ky * unit, 0.0, 1.0);
        } break;
        case PRIMITIVE_FILL: {
            float alpha = min(winding.x, winding.y);
            color.rgb = sqrt(vec3(1.0 - alpha));
            color.a = alpha;
            winding = vec2(0.0);
        } break;
        }
    }

    o_frag = color;
}
