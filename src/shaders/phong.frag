#version 450
// Reference GLSL. The live shader is phong.wgsl (compiled by build.rs).

layout(location = 0) in vec3 fragPos;
layout(location = 1) in vec3 fragNormal;
layout(location = 2) in vec2 fragUV;

layout(set = 0, binding = 0) uniform CameraUBO {
    mat4 view;
    mat4 proj;
    vec3 cameraPos;
} camera;

layout(set = 0, binding = 1) uniform LightUBO {
    vec3 lightPos;
    float ambientStrength;
    vec3 lightColor;
    float specularStrength;
    float shininess;
} light;

layout(set = 1, binding = 0) uniform texture2D tDiffuse;
layout(set = 1, binding = 1) uniform sampler sDiffuse;

layout(location = 0) out vec4 outColor;

void main() {
    vec4 albedo = texture(sampler2D(tDiffuse, sDiffuse), fragUV);
    vec3 norm = normalize(fragNormal);
    vec3 lightDir = normalize(light.lightPos - fragPos);

    vec3 ambient = light.ambientStrength * light.lightColor;
    vec3 diffuse = max(dot(norm, lightDir), 0.0) * light.lightColor;

    vec3 viewDir = normalize(camera.cameraPos - fragPos);
    vec3 reflectDir = reflect(-lightDir, norm);
    float spec = pow(max(dot(viewDir, reflectDir), 0.0), max(light.shininess, 1.0));
    vec3 specular = light.specularStrength * spec * light.lightColor;

    outColor = vec4((ambient + diffuse) * albedo.rgb + specular, albedo.a);
}
