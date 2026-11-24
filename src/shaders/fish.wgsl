struct FishUniforms {
  resolution: vec2<f32>,
  pad0: vec2<f32>,
  shadow_color: vec4<f32>,
  shadow_opacity: f32,
  pad1: f32,
  pad2: f32,
  pad3: f32,
};

struct VsOut {
  @builtin(position) position: vec4<f32>,
  @location(0) color: vec3<f32>,
};

@group(0) @binding(0) var<uniform> fish: FishUniforms;

@vertex
fn vs_main(
  @location(0) position: vec2<f32>,
  @location(1) color: vec3<f32>,
) -> VsOut {
  var out: VsOut;
  out.position = vec4<f32>(
    position.x / fish.resolution.x * 2.0 - 1.0,
    1.0 - position.y / fish.resolution.y * 2.0,
    0.0,
    1.0,
  );
  out.color = color;
  return out;
}

@fragment
fn fs_main(input: VsOut) -> @location(0) vec4<f32> {
  return vec4<f32>(input.color, 1.0);
}

@fragment
fn fs_shadow(input: VsOut) -> @location(0) vec4<f32> {
  return vec4<f32>(
    fish.shadow_color.rgb,
    fish.shadow_opacity * input.color.r,
  );
}
