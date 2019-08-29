#version 450 core

//! Quadratic curve visualization
//!
//! Based on https://developer.nvidia.com/gpugems/GPUGems3/gpugems3_ch25.html

layout (location = 0) in vec2 a_uv_coords;
layout (location = 0) out vec4 o_color;

void main() {
    vec2 dx = dFdx(a_uv_coords);
    vec2 dy = dFdy(a_uv_coords);

    float fx = (2.0 * a_uv_coords.x) * dx.x - dx.y;
    float fy = (2.0 * a_uv_coords.x) * dy.x - dy.y;

    float distance = (a_uv_coords.x * a_uv_coords.x - a_uv_coords.y)/sqrt(fx*fx + fy*fy);
    float alpha = clamp(0.5 - distance, 0.0, 1.0);
    o_color = vec4(a_uv_coords, alpha, 1.0);
}
