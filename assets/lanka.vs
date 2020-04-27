#version 450 core

#define GRR 1

layout (location = 0) in vec2 v_pos_world;
layout (location = 1) in vec2 v_pos_curve;
layout (location = 2) in uvec3 v_curve_range;

layout (location = 0) out vec2 a_pos_curve;
layout (location = 1) out uvec3 a_curve_range;
layout (location = 2) out vec2 a_pos_world;

#if GRR
layout (location = 1) uniform vec4 u_viewport;
#else
layout(set = 0, binding = 2) uniform Locals {
    vec4 u_viewport;
    vec2 u_screen_dim;
    uint u_num_primitives;
};
#endif

void main() {
    const vec2 viewport_pos = u_viewport.xy;
    const vec2 viewport_size = u_viewport.zw;

    a_pos_curve = v_pos_curve;
    a_curve_range = v_curve_range;
    a_pos_world = v_pos_world;

    // World -> View
    const vec2 a_pos_view = (v_pos_world - viewport_pos) / (0.5 * viewport_size);
    gl_Position = vec4(a_pos_view, 0.0, 1.0);

#if (GRR == 0)
    gl_Position.y = -gl_Position.y;
#endif
}
