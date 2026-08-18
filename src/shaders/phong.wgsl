// Combined vertex + fragment Phong pipeline (naga compiles both entry points).

struct Camera {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _pad: f32,
}

struct Light {
    light_pos: vec3<f32>,
    ambient_strength: f32,
    light_color: vec3<f32>,
    specular_strength: f32,
    shininess: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}

struct Push {
    model: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var<uniform> light: Light;
@group(1) @binding(0) var t_diffuse: texture_2d<f32>;
@group(1) @binding(1) var s_diffuse: sampler;

var<push_constant> pc: Push;

struct VsIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VsOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

@vertex
fn vs_main(v: VsIn) -> VsOut {
    let world = pc.model * vec4<f32>(v.position, 1.0);
    let n = normalize((pc.model * vec4<f32>(v.normal, 0.0)).xyz);
    var out: VsOut;
    out.world_pos = world.xyz;
    out.world_normal = n;
    out.uv = v.uv;
    out.clip_position = camera.proj * camera.view * world;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let albedo = textureSample(t_diffuse, s_diffuse, in.uv);
    let norm = normalize(in.world_normal);
    let light_dir = normalize(light.light_pos - in.world_pos);

    let ambient = light.ambient_strength * light.light_color;

    let diff = max(dot(norm, light_dir), 0.0);
    let diffuse = diff * light.light_color;

    let view_dir = normalize(camera.camera_pos - in.world_pos);
    let reflect_dir = reflect(-light_dir, norm);
    let spec = pow(max(dot(view_dir, reflect_dir), 0.0), max(light.shininess, 1.0));
    let specular = light.specular_strength * spec * light.light_color;

    let lit = (ambient + diffuse) * albedo.rgb + specular;
    return vec4<f32>(lit, albedo.a);
}
