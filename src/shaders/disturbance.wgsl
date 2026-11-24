struct DisturbanceUniforms {
  resolution: vec2<f32>,
  time: f32,
  strength: f32,
  wave_frequency: f32,
  wave_speed: f32,
  pad0: f32,
  pad1: f32,
};

struct VsOut {
  @builtin(position) position: vec4<f32>,
  @location(0) local: vec2<f32>,
  @location(1) forward: vec2<f32>,
  @location(2) depth: vec2<f32>,
};

@group(0) @binding(0) var<uniform> disturbance: DisturbanceUniforms;

@vertex
fn vs_main(
  @location(0) center: vec2<f32>,
  @location(1) direction: vec2<f32>,
  @location(2) size: vec2<f32>,
  @location(3) depth: vec2<f32>,
  @location(4) local: vec2<f32>,
) -> VsOut {
  let normal = vec2<f32>(-direction.y, direction.x);
  let point = center + direction * local.x * size.x + normal * local.y * size.y;
  var out: VsOut;
  out.position = vec4<f32>(
    point.x / disturbance.resolution.x * 2.0 - 1.0,
    1.0 - point.y / disturbance.resolution.y * 2.0,
    0.0,
    1.0,
  );
  out.local = local;
  out.forward = direction;
  out.depth = depth;
  return out;
}

@fragment
fn fs_main(input: VsOut) -> @location(0) vec2<f32> {
  let radius_squared = dot(input.local, input.local);
  let mask =
    exp(-radius_squared * 2.35)
    * (1.0 - smoothstep(0.82, 1.06, radius_squared));
  let normal = vec2<f32>(-input.forward.y, input.forward.x);
  let phase = input.depth.y + disturbance.time * disturbance.wave_speed;
  let wave_a = sin(
    (input.local.x * 0.82 + input.local.y * 0.31) * disturbance.wave_frequency + phase
  );
  let wave_b = cos(
    (input.local.y * 0.91 - input.local.x * 0.24) * (disturbance.wave_frequency * 0.73)
    - phase * 1.17
  );
  let displacement =
    (normal * wave_a + input.forward * wave_b * 0.62)
    * mask
    * input.depth.x
    * disturbance.strength;
  return displacement;
}
