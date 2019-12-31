#version 460

vec2 triangle_vertices[3] = vec2[] (
    vec2(0.0, -0.5),
    vec2(0.5, 0.5),
    vec2(-0.5, 0.5)
);

vec3 triangle_colors[3] = vec3[] (
    vec3(1.0, 0.0, 0.0),
    vec3(0.0, 1.0, 0.0),
    vec3(0.0, 0.0, 1.0)
);

layout(push_constant) uniform PushConstant {
    float number[4];
} push_constant;

layout(location = 0) out vec3 vertex_color;

void main() {
    gl_Position = vec4(triangle_vertices[gl_VertexIndex] * push_constant.number[gl_VertexIndex], 0.0, 1.0)
        + vec4(push_constant.number[3], push_constant.number[3], 0.0, 0.0);
    vertex_color = triangle_colors[gl_VertexIndex];
}
