#version 450 core

layout (location = 0) in vec2 v_position_obj;
layout (location = 1) in vec2 v_uv_coords;

layout (location = 0) out vec2 a_uv_coords;

void main() {
    gl_Position = vec4(v_position_obj, 0.0, 1.0);
    a_uv_coords = v_uv_coords;
}
