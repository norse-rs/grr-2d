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
layout (location = 1) flat in uvec3 f_curve_range;

layout (binding = 0, std430) readonly buffer SceneVertices {
    vec2 vertices[];
};
layout (binding = 1, std430) readonly buffer ScenePrimitives {
    uint primitives[];
};

out vec4 o_frag;

float filtering(float x)
{
    return clamp(x, 0.0, 1.0);
    // return smoothstep(-1.0, 1.0, x);
}

void main() {
    const vec2 tile_center = f_pos_curve;
    const vec2 tile_extent = fwidth(tile_center);
    const float unit = 1.0 / (tile_extent.x);

    vec4 color = vec4(0.0, 0.0, 0.0, 0.0);
    float stroke_df = FLOAT_MAX;
    vec2 winding = vec2(0.0);

    uint base_vertex = f_curve_range.x;
    for (uint i = f_curve_range.y; i < f_curve_range.z; i++) {
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

            winding.x -= sign_x * filtering(0.5 - kx * unit);
            winding.y += sign_y * filtering(0.5 - ky * unit);
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

            const float kx = (a.x * tx - 2 * b.x) * tx + c.x;
            const float ky = (a.y * ty - 2 * b.y) * ty + c.y;

            winding.x -= sign_x * filtering(0.5 - kx * unit);
            winding.y += sign_y * filtering(0.5 - ky * unit);
        } break;
        case PRIMITIVE_FILL: {
            winding = abs(winding);
            float alpha = max(abs(winding.x), abs(winding.y));
            if (winding.x < -1.0) {
                color.rgb = vec3(1.0, 0.0, 0.0);
            } else if (winding.x < 0.0 && winding.x > -1.0) {
                color.rgb = vec3(0.0, 1.0, winding.x);
            } else {
                color.rgb = vec3(0.0, 0.0, 0.0);
            }

            color.rgb = vec3(winding.x, winding.y, 0.0); // vec3(1.0 - winding.x, 1.0 - winding.y, (1.0 - winding.x) * (1.0 - winding.y));
            color.a = 1.0;
            // color = pow(color, vec4(1.0));
            winding = vec2(0.0);
        } break;
        }
    }

    o_frag = color;
}
