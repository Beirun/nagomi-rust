struct WaterConstants {
  resolution_time_count: vec4<f32>,
  flags: vec4<f32>,
  tint: vec4<f32>,
  large_color: vec4<f32>,
  large_core_color: vec4<f32>,
  secondary_color: vec4<f32>,
  secondary_core_color: vec4<f32>,
  detail_color: vec4<f32>,
  detail_core_color: vec4<f32>,
  cells: vec4<f32>,
  layer_opacity: vec4<f32>,
  wave_dir_a: vec4<f32>,
  wave_dir_b: vec4<f32>,
  wave_dir_c: vec4<f32>,
  wave_frequency: vec4<f32>,
  wave_speed: vec4<f32>,
  wave_strength: vec4<f32>,
  layer_times: vec4<f32>,
  physics_a: vec4<f32>,
  physics_b: vec4<f32>,
  physics_c: vec4<f32>,
  curves_a: vec4<f32>,
  curves_b: vec4<f32>,
  curves_c: vec4<f32>,
};

const MAX_RIPPLES: u32 = 64u;

@group(0) @binding(0) var underwater_texture: texture_2d<f32>;
@group(0) @binding(1) var disturbance_texture: texture_2d<f32>;
@group(0) @binding(2) var water_sampler: sampler;
@group(0) @binding(3) var<uniform> water: WaterConstants;
@group(0) @binding(4) var<uniform> ripples: array<vec4<f32>, 64>;

fn ripple_physics(type_index: i32) -> vec4<f32> {
  if (type_index == 1) { return water.physics_b; }
  if (type_index == 2) { return water.physics_c; }
  return water.physics_a;
}

fn ripple_curves(type_index: i32) -> vec4<f32> {
  if (type_index == 1) { return water.curves_b; }
  if (type_index == 2) { return water.curves_c; }
  return water.curves_a;
}

fn hash22(point: vec2<f32>) -> vec2<f32> {
  let value = vec2<f32>(
    dot(point, vec2<f32>(127.1, 311.7)),
    dot(point, vec2<f32>(269.5, 183.3)),
  );
  return fract(sin(value) * 43758.5453);
}

fn value_noise(point: vec2<f32>) -> f32 {
  let cell = floor(point);
  var local = fract(point);
  local = local * local * (3.0 - 2.0 * local);

  let bottom_left = hash22(cell).x;
  let bottom_right = hash22(cell + vec2<f32>(1.0, 0.0)).x;
  let top_left = hash22(cell + vec2<f32>(0.0, 1.0)).x;
  let top_right = hash22(cell + vec2<f32>(1.0, 1.0)).x;
  return mix(
    mix(bottom_left, bottom_right, local.x),
    mix(top_left, top_right, local.x),
    local.y,
  );
}

fn warp_water(pixel: vec2<f32>) -> vec2<f32> {
  let warp_x = value_noise(pixel * 0.010);
  let warp_y = value_noise(pixel * 0.012 + vec2<f32>(19.4, 7.8));
  let small_warp_x = value_noise(pixel * 0.022 + vec2<f32>(31.8, -12.1));
  let small_warp_y = value_noise(pixel * 0.019 + vec2<f32>(-8.2, 26.6));

  let broad_bend = vec2<f32>(
    sin(pixel.y * 0.025 + warp_y * 5.2),
    cos(pixel.x * 0.022 + warp_x * 5.6),
  );
  let small_bend = vec2<f32>(
    sin((pixel.x + pixel.y) * 0.034 + small_warp_y * 4.8),
    cos((pixel.x - pixel.y) * 0.030 + small_warp_x * 5.1),
  );
  return pixel
    + (vec2<f32>(warp_x, warp_y) - vec2<f32>(0.5)) * 48.0
    + (vec2<f32>(small_warp_x, small_warp_y) - vec2<f32>(0.5)) * 18.0
    + broad_bend * 11.0
    + small_bend * 4.5;
}

fn cellular_border_distance(point: vec2<f32>) -> f32 {
  let cell = floor(point);
  let local = fract(point);
  var nearest = 10.0;
  var second_nearest = 10.0;

  for (var y = -1; y <= 1; y += 1) {
    for (var x = -1; x <= 1; x += 1) {
      let neighbour = vec2<f32>(f32(x), f32(y));
      let seed = hash22(cell + neighbour);
      let animated_point = vec2<f32>(0.5) + 0.32 * sin(6.2831853 * seed);
      let point_delta = neighbour + animated_point - local;
      let influence = mix(
        0.68,
        1.38,
        hash22(cell + neighbour + vec2<f32>(41.7, 13.2)).x,
      );
      let distance_to_point = length(point_delta) / influence;
      if (distance_to_point < nearest) {
        second_nearest = nearest;
        nearest = distance_to_point;
      } else if (distance_to_point < second_nearest) {
        second_nearest = distance_to_point;
      }
    }
  }

  return second_nearest - nearest;
}

fn directional_wave_pixel(pixel: vec2<f32>, resolution: vec2<f32>, time: f32) -> vec2<f32> {
  let aspect = resolution.x / resolution.y;
  let centered = (pixel / resolution - vec2<f32>(0.5)) * vec2<f32>(aspect, 1.0);

  let phase_a =
    dot(centered, water.wave_dir_a.xy) * water.wave_frequency.x
    - time * water.wave_speed.x;
  let phase_b =
    dot(centered, water.wave_dir_b.xy) * water.wave_frequency.y
    - time * water.wave_speed.y;
  let phase_c =
    dot(centered, water.wave_dir_c.xy) * water.wave_frequency.z
    - time * water.wave_speed.z;

  let slope =
    water.wave_dir_a.xy * sin(phase_a) * water.wave_strength.x
    + water.wave_dir_b.xy * sin(phase_b) * water.wave_strength.y
    + water.wave_dir_c.xy * sin(phase_c) * water.wave_strength.z;
  let uv_offset = slope * vec2<f32>(1.0 / aspect, 1.0) * water.flags.z;
  return pixel + uv_offset * resolution;
}

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
  let resolution = water.resolution_time_count.xy;
  let time = water.resolution_time_count.z;
  let ripple_count = water.resolution_time_count.w;

  let pixel = uv * resolution;
  var displacement = vec2<f32>(
    sin(pixel.y * 0.051 + time * 0.31) + sin(pixel.y * 0.017 - time * 0.19),
    -(cos(pixel.x * 0.043 - time * 0.23) + sin(pixel.x * 0.014 + time * 0.16)),
  ) * 0.13 / resolution;

  for (var index = 0u; index < MAX_RIPPLES; index += 1u) {
    if (f32(index) >= ripple_count) { break; }

    let ripple = ripples[index];
    let type_index = i32(floor(ripple.w * 0.5));
    let strength = ripple.w % 2.0;
    let physics = ripple_physics(type_index);
    let curves = ripple_curves(type_index);
    let delta = pixel - ripple.xy;
    let distance_to_center = length(delta);
    let radial = delta / max(distance_to_center, 0.001);
    let radius = physics.y + ripple.z * physics.z;
    let signed_distance = distance_to_center - radius;
    let fade = 1.0 - smoothstep(physics.x * curves.y, physics.x, ripple.z);
    let life = clamp(ripple.z / max(physics.x, 0.001), 0.0, 1.0);
    let distortion_band =
      exp(-abs(signed_distance) * curves.x)
      * fade
      * strength
      * (1.0 - life * curves.z);
    var direction = 1.0;
    if (signed_distance < 0.0) { direction = -1.0; }

    displacement = displacement
      + radial * direction * distortion_band * physics.w / resolution;
  }

  let local_disturbance = textureSample(disturbance_texture, water_sampler, uv).xy;
  displacement = displacement + vec2<f32>(local_disturbance.x, local_disturbance.y) / resolution;

  let sample_uv = clamp(uv + displacement, vec2<f32>(0.002), vec2<f32>(0.998));
  var color = textureSample(underwater_texture, water_sampler, sample_uv).rgb;

  let distorted_pixel = sample_uv * resolution;

  color = color * water.tint.rgb;

  if (water.flags.y > 0.5) {
    let large_wave_pixel = directional_wave_pixel(
      distorted_pixel,
      resolution,
      water.layer_times.x,
    );
    let warped_pixel = warp_water(large_wave_pixel);
    let secondary_large_wave_pixel = directional_wave_pixel(
      distorted_pixel,
      resolution,
      water.layer_times.y,
    );
    let detail_wave_pixel = directional_wave_pixel(
      distorted_pixel,
      resolution,
      water.layer_times.z,
    );
    let secondary_large_pixel = warped_pixel + secondary_large_wave_pixel - large_wave_pixel;
    let detail_pixel = warped_pixel + detail_wave_pixel - large_wave_pixel;

    let large_border = cellular_border_distance(warped_pixel / max(water.cells.x, 0.001));
    let large_width_noise = value_noise(warped_pixel * 0.018 + vec2<f32>(3.7, 11.2));
    let large_vein = 1.0 - smoothstep(
      0.032 + large_width_noise * 0.010,
      0.125 + large_width_noise * 0.022,
      large_border,
    );
    let large_core = 1.0 - smoothstep(0.010, 0.052, large_border);

    let secondary_large_border = cellular_border_distance(
      secondary_large_pixel / max(water.cells.y, 0.001) + vec2<f32>(5.2, 8.4),
    );
    let secondary_large_width_noise = value_noise(
      secondary_large_pixel * 0.015 + vec2<f32>(17.6, -6.8),
    );
    let secondary_large_vein = 1.0 - smoothstep(
      0.032 + secondary_large_width_noise * 0.010,
      0.125 + secondary_large_width_noise * 0.022,
      secondary_large_border,
    );
    let secondary_large_core =
      1.0 - smoothstep(0.010, 0.052, secondary_large_border);

    let detail_border = cellular_border_distance(
      detail_pixel / max(water.cells.z, 0.001) + vec2<f32>(9.6, 4.3),
    );
    let detail_region = smoothstep(
      0.48,
      0.75,
      value_noise(detail_pixel * 0.008 + vec2<f32>(-5.1, 17.8)),
    );
    let detail_vein =
      (1.0 - smoothstep(0.030, 0.105, detail_border)) * detail_region;
    let detail_core =
      (1.0 - smoothstep(0.008, 0.043, detail_border)) * detail_region;

    let clarity = 1.0 - water.flags.x;
    color = color
      + (large_vein * water.large_color.rgb + large_core * water.large_core_color.rgb)
        * water.layer_opacity.x * clarity;
    color = color
      + (secondary_large_vein * water.secondary_color.rgb
        + secondary_large_core * water.secondary_core_color.rgb)
        * water.layer_opacity.y * clarity;
    color = color
      + (detail_vein * water.detail_color.rgb + detail_core * water.detail_core_color.rgb)
        * water.layer_opacity.z * clarity;
  }

  return vec4<f32>(color, 1.0);
}
