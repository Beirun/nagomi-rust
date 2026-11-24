struct WeatherUniforms {
  time: f32,
  brightness: f32,
  contrast: f32,
  saturation: f32,
  vignette: f32,
  cloud_strength: f32,
  light_strength: f32,
  pad0: f32,
  light_direction: vec2<f32>,
  pad1: vec2<f32>,
  tint: vec4<f32>,
  light_color: vec4<f32>,
};

@group(0) @binding(0) var source_texture: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;
@group(0) @binding(2) var<uniform> weather: WeatherUniforms;

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
  var color = textureSample(source_texture, source_sampler, uv).rgb;

  var cloud_wave =
    sin(uv.x * 5.2 + uv.y * 2.1 + weather.time * 0.035)
    + sin(uv.x * 2.3 - uv.y * 4.7 - weather.time * 0.022)
    + sin((uv.x + uv.y) * 8.1 + weather.time * 0.016);
  cloud_wave = cloud_wave / 6.0 + 0.5;
  color = color * (1.0 - weather.cloud_strength * (0.055 + cloud_wave * 0.075));

  color = color * weather.tint.rgb * weather.brightness;
  let luminance = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
  color = mix(vec3<f32>(luminance), color, weather.saturation);
  color = (color - vec3<f32>(0.5)) * weather.contrast + vec3<f32>(0.5);

  let centered = vec2<f32>(uv.x - 0.5, 0.5 - uv.y);
  let light_dir = normalize(weather.light_direction);
  var directional_light = dot(centered, light_dir) + 0.5;
  directional_light = smoothstep(0.05, 0.95, directional_light);
  color = color + weather.light_color.rgb * directional_light * weather.light_strength;

  let edge_len = length(vec2<f32>(centered.x, centered.y) * vec2<f32>(1.0, 1.3));
  let edge = smoothstep(0.36, 0.76, edge_len);
  color = color * (1.0 - edge * weather.vignette);

  return vec4<f32>(max(color, vec3<f32>(0.0)), 1.0);
}
