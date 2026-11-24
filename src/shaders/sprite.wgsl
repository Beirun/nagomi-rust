struct SpriteUniforms {
  resolution: vec2<f32>,
  pad: vec2<f32>,
};

struct VsOut {
  @builtin(position) position: vec4<f32>,
  @location(0) uv: vec2<f32>,
  @location(1) tint: vec3<f32>,
};

@group(0) @binding(0) var sprite_texture: texture_2d<f32>;
@group(0) @binding(1) var sprite_sampler: sampler;
@group(1) @binding(0) var<uniform> sprite: SpriteUniforms;

@vertex
fn vs_main(
  @location(0) position: vec2<f32>,
  @location(1) uv: vec2<f32>,
  @location(2) tint: vec3<f32>,
) -> VsOut {
  var out: VsOut;
  out.position = vec4<f32>(
    position.x / sprite.resolution.x * 2.0 - 1.0,
    1.0 - position.y / sprite.resolution.y * 2.0,
    0.0,
    1.0,
  );
  out.uv = uv;
  out.tint = tint;
  return out;
}

@fragment
fn fs_main(input: VsOut) -> @location(0) vec4<f32> {
  let texel = textureSample(sprite_texture, sprite_sampler, input.uv);
  if (texel.a < 0.5) {
    discard;
  }
  return vec4<f32>(texel.rgb * input.tint, 1.0);
}
