#version 450

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
    vec3 lightColor;
    float ambientStrength;
    float specularStrength;
    float shininess;
} light;

layout(location = 0) out vec4 outColor;

void main() {
    vec3 norm = normalize(fragNormal);
    vec3 lightDir = normalize(light.lightPos - fragPos);

    // Ambient
    vec3 ambient = light.ambientStrength * light.lightColor;

    // Diffuse
    float diff = max(dot(norm, lightDir), 0.0);
    vec3 diffuse = diff * light.lightColor;

    // Specular (Phong)
    vec3 viewDir = normalize(camera.cameraPos - fragPos);
    vec3 reflectDir = reflect(-lightDir, norm);
    float spec = pow(max(dot(viewDir, reflectDir), 0.0), light.shininess);
    vec3 specular = light.specularStrength * spec * light.lightColor;

    vec3 result = ambient + diffuse + specular;
    outColor = vec4(result, 1.0);   // later multiply by texture
}