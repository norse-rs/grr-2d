#version 450 core

const float FLOAT_MAX = 3.402823466e+38;

const uint PRIMITIVE_LINE = 0x1; // distance field generation (linear curve)
const uint PRIMITIVE_STROKE = 0x2; // distance field stroke shading
const uint PRIMITIVE_QUADRATIC = 0x3; // quadratic curve
const uint PRIMITIVE_FILL = 0x4; // fill shading


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

float acos_approx(float x)
{
    // Eberly
    const float c0 = 1.570796;
    const float c1 = -0.203471;
    const float c2 = 0.0468878;
    const float pi = 3.1415926;

    float abs_x = abs(x);
    float res = (c2 * abs_x + c1) * abs_x + c0;
    res *= sqrt(1.0 -abs_x);

    return (abs_x >= 0) ? res : pi - res;
}

vec2 solve_cubic(vec4 coeffs)
{
    float b = coeffs.y / coeffs.x;
    float c = coeffs.z / coeffs.x;
    float d = coeffs.w / coeffs.x;

    const float p = c - b * b / 3.0;
    const float q = b * (2.0 * b * b - 9.0 * c) / 27.0 + d;

    const float unpress = -b / 3.0;
    const float p3 = p * p * p;
    const float det = q * q + 4.0 * p3 / 27.0;

    if (det > 0.0) {
        const float drt = sqrt(det);
        const vec2 x = 0.5 * (vec2(drt, -drt) - q);
        const vec2 roots = sign(x) * pow(abs(x), vec2(1.0 / 3.0));
        return vec2(roots.x + roots.y + unpress);
    } else {
        const float theta = acos_approx(-sqrt(-27.0 /4.0 * q * q / p3)) / 3.0;
        const vec2 roots = vec2(cos(theta), sin(theta));

        const float x = roots.x * sqrt(-p/3.0);
	    return vec2(
            2.0 * x + unpress,
            - (x + roots.y * sqrt(-p)) + unpress
        );
    }
}

float df_quadratic_bezier(vec2 b0, vec2 b1, vec2 b2, vec2 p)
{
    const vec2 a = b1 - b0;
    const vec2 b = b2 - b1 - a;
    const vec2 c = p - b0;

    const vec4 coeffs = vec4(-dot(b, b), -3.0 * dot(a, b), dot(b, c) - 2.0 * dot(a, a), dot(a, c));
    const vec2 t = clamp(solve_cubic(coeffs), 0.0, 1.0);

    const vec2 d0 = (2.0 * a + b * t.x) * t.x - c;
    const vec2 d1 = (2.0 * a + b * t.y) * t.y - c;

    return min(dot(d0, d0), dot(d1, d1));
}


void main() {
    const vec2 tile_extent = 1.0 / u_screen_dim;
    const vec2 tile_offset = u_viewport.xy + tile_extent * gl_FragCoord.xy;
    const vec2 tile_center = f_pos_curve;

    vec4 color = vec4(0.0, 0.0, 0.0, 0.0);
    float stroke_df = FLOAT_MAX;
    float winding = 0.0;

    const float unit = 1.0 / sqrt(dot(tile_extent, tile_extent));

    uint base_vertex = 0;
    for (uint i = 0; i < u_num_primitives; i++) {
        const uint primitive = primitives[i];
        switch (primitive) {
        case PRIMITIVE_LINE: {
            // line distance field
            const vec2 p0 = vertices[base_vertex++];
            const vec2 p1 = vertices[base_vertex++];

            // intersection check
            const bool y0 = p0.y > tile_center.y;
            const bool y1 = p1.y > tile_center.y;

            if (y0 && !y1) {
                winding -= 1.0;
            } else if (!y0 && y1) {
                winding += 1.0;
            }

            // distance field
            const vec2 line = p1 - p0;
            const vec2 dp = tile_center - p0;
            const float t = clamp(dot(line, dp) / dot(line, line), 0.0, 1.0);
            const float field = length(line * t - dp);
            stroke_df = min(stroke_df, field);
        } break;
        case PRIMITIVE_STROKE: {
            // stroke shading
            const float alpha = clamp(0.5 * stroke_df * unit, 0.0, 1.0);
            // color.rgb += mix(color.rgb, vec3(1.0, 0.0, 0.0), alpha);
            // color.a += alpha;

            color.rgb = vec3(0.5 + 0.5 * winding);
            color.a = 1.0;

            // stroke_df = FLOAT_MAX;
        } break;
        case PRIMITIVE_QUADRATIC: {
            // quadratic curve distance field
            const vec2 p0 = vertices[base_vertex++];
            const vec2 p1 = vertices[base_vertex++];
            const vec2 p2 = vertices[base_vertex++];

            // -- Analytic
            float distsq = df_quadratic_bezier(p0, p1, p2, tile_center);
            stroke_df = min(stroke_df, sqrt(distsq));
        } break;
        }
    }

    o_frag = color;
}
