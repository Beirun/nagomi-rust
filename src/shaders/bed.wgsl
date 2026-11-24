struct BedUniforms {
  resolution: vec2<f32>,
  vertical_tone: f32,
  grain_scale: f32,
  edge_darkening: f32,
  pad0: f32,
  pad1: f32,
  pad2: f32,
  deep_color: vec4<f32>,
  shallow_color: vec4<f32>,
  speck_color: vec4<f32>,
};

@group(0) @binding(0) var<uniform> bed: BedUniforms;

fn hash21(point: vec2<f32>) -> f32 {
  var p = fract(point * vec2<f32>(123.34, 456.21));
  p = p + dot(p, p + 45.32);
  return fract(p.x * p.y);
}

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
  // The web renderer flips the underwater target when the water pass samples it,
  // so the vertical tone that reads at the top of the screen is the high end.
  let up = uv.y;
  let vertical_tone = smoothstep(0.0, 1.0, up) * bed.vertical_tone;
  var color = mix(bed.deep_color.rgb, bed.shallow_color.rgb, vertical_tone);

  let grain_cell = floor(uv * bed.resolution * bed.grain_scale);
  let grain = hash21(grain_cell);
  let dark_speck = smoothstep(0.975, 0.998, grain);
  color = color - dark_speck * bed.speck_color.rgb;

  let centered = (uv - vec2<f32>(0.5, 0.5)) * vec2<f32>(1.0, 1.25);
  let edge_depth = smoothstep(0.48, 0.82, length(centered));
  color = color * (1.0 - edge_depth * bed.edge_darkening);

  return vec4<f32>(color, 1.0);
}
