#version 450 core

layout (location = 0) in vec2 v_pos_world;
layout (location = 1) in vec2 v_pos_curve;
layout (location = 2) in uvec3 v_curve_range;

layout (location = 0) out vec2 a_pos_curve;
layout (location = 1) out uvec3 a_curve_range;
layout (location = 2) out vec2 a_pos_world;

layout (location = 1) uniform vec4 u_viewport;

void main() {
    const vec2 viewport_pos = u_viewport.xy;
    const vec2 viewport_size = u_viewport.zw;

    a_pos_curve = v_pos_curve;
    a_curve_range = v_curve_range;
    a_pos_world = v_pos_world;

    // World -> View
    const vec2 a_pos_view = (v_pos_world - viewport_pos) / (0.5 * viewport_size);
    gl_Position = vec4(a_pos_view, 0.0, 1.0);
}
