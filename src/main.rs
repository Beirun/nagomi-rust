mod fish;
mod butterflies;
mod control;
mod duckweed;
mod gpu;
mod lotus;
mod tiny;
mod weather;
mod sim;

use std::{
    path::PathBuf,
    ptr::NonNull,
    time::{Duration, Instant},
};

use anyhow::Result;
use calloop::EventLoop;
use raw_window_handle::{
    RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    output::{OutputHandler, OutputState},
    reexports::calloop_wayland_source::WaylandSource,
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
        Capability, SeatHandler, SeatState,
    },
    shell::{
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
        WaylandSurface,
    },
};
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_output, wl_pointer, wl_seat, wl_surface},
    Connection, Proxy, QueueHandle,
};

use fish::KoiDraw;
use gpu::{FishFrame, SurfaceFrame};
use sim::{FishConfig, School, FIXED_STEP};


fn state_path() -> PathBuf {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/state")
        });
    base.join("nagomi/state")
}

fn load_state() -> (Option<usize>, Option<String>, Option<u32>) {
    let Ok(text) = std::fs::read_to_string(state_path()) else {
        return (None, None, None);
    };
    let mut fish = None;
    let mut ambience = None;
    let mut scale = None;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("fish=") {
            fish = value.trim().parse::<usize>().ok();
        } else if let Some(value) = line.strip_prefix("ambience=") {
            ambience = Some(value.trim().to_string());
        } else if let Some(value) = line.strip_prefix("scale=") {
            scale = value.trim().parse::<u32>().ok();
        }
    }
    (fish, ambience, scale)
}

fn save_state(fish: usize, ambience: &str, scale: u32) {
    let path = state_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(
        path,
        format!("fish={fish}\nambience={ambience}\nscale={scale}\n"),
    );
}

const FRAME_INTERVAL: Duration = Duration::from_millis(33);

struct App {
    // Field order matters: the wgpu surface must go before the layer surface.
    renderer: gpu::Renderer,
    layer: LayerSurface,
    layer_shell: LayerShell,
    compositor_state: CompositorState,

    registry_state: RegistryState,
    output_state: OutputState,
    seat_state: SeatState,
    pointer: Option<wl_pointer::WlPointer>,
    pending_call: Option<(f32, f32)>,

    width: u32,
    height: u32,
    configured: bool,
    exit: bool,
    dump_path: Option<String>,

    school: School,
    draw: KoiDraw,
    lotus: lotus::Lotus,
    lotus_frame: lotus::LotusFrame,
    duckweed: duckweed::Duckweed,
    duckweed_frame: duckweed::DuckweedFrame,
    tiny_frame: tiny::TinyFrame,
    butterflies: butterflies::Butterflies,
    butterfly_frame: butterflies::ButterflyFrame,
    weather: weather::Weather,
    config: FishConfig,
    accumulator: f32,
    sim_time: f32,
    last_sim: Instant,
    started: Instant,
}

impl CompositorHandler for App {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for App {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}

    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}

    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl SeatHandler for App {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer && self.pointer.is_none() {
            self.pointer = self.seat_state.get_pointer(qh, &seat).ok();
        }
    }

    fn remove_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer {
            if let Some(pointer) = self.pointer.take() {
                pointer.release();
            }
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl PointerHandler for App {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            match event.kind {
                PointerEventKind::Press { button: 0x110, .. } => {
                    if self.width > 0 && self.height > 0 {
                        let x = event.position.0 / self.width as f64 * gpu::POND_WIDTH as f64;
                        let y = event.position.1 / self.height as f64 * gpu::POND_HEIGHT as f64;
                        self.pending_call = Some((x as f32, y as f32));
                    }
                }
                _ => {}
            }
        }
    }
}

impl LayerShellHandler for App {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        self.exit = true;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let (width, height) = configure.new_size;
        if width > 0 {
            self.width = width;
        }
        if height > 0 {
            self.height = height;
        }
        log::info!("configure {}x{}", self.width, self.height);
        self.renderer.configure(self.width, self.height);
        self.configured = true;
    }
}

impl App {
    fn rebuild_fish(&mut self) {
        self.draw.clear();
        for index in 0..self.school.count {
            let mut fish = self.school.fish[index];
            let depth = fish::visual_depth(&self.config, fish.depth);
            let appearance =
                fish::depth_appearance(&fish::appearance_for(index), &self.config, depth);
            fish::build_render_spine(&mut fish);
            fish::draw_koi(&self.config, &fish, &appearance, &mut self.draw);
        }
    }

    fn ripple_uniforms(&self) -> ([[f32; 4]; 64], u32) {
        let mut ripple_data = [[0.0f32; 4]; 64];
        let mut ripple_count = 0;
        for ripple in &self.school.ripples.instances {
            if !ripple.alive || ripple.age < 0.0 || ripple_count >= 64 {
                continue;
            }
            ripple_data[ripple_count] = [
                ripple.center.x,
                ripple.center.y,
                ripple.age,
                ripple.kind.order_index() as f32 * 2.0 + ripple.sim_strength(),
            ];
            ripple_count += 1;
        }
        (ripple_data, ripple_count as u32)
    }
}

impl App {
    fn handle_commands(&mut self, listener: &std::os::unix::net::UnixListener) {
        use std::io::Write;
        loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let Some(request) = control::read_request(&stream) else {
                        continue;
                    };
                    let response = self.handle_command(request.trim());
                    let _ = stream.write_all(response.as_bytes());
                    let _ = stream.flush();
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }
    }

    fn handle_command(&mut self, request: &str) -> String {
        let mut words = request.split_whitespace();
        match words.next().unwrap_or("") {
            "" => String::new(),
            "fish" => match words.next() {
                None => format!("fish {}\n", self.school.count),
                Some(value) => match value.parse::<usize>() {
                    Ok(count) => {
                        let count = count.clamp(1, sim::MAX_FISH);
                        self.school.count = count;
                        save_state(count, self.weather.name(), self.renderer.scale());
                        format!("fish {count}\n")
                    }
                    Err(_) => format!(
                        "nagomi: fish needs a number between 1 and {}\n",
                        sim::MAX_FISH
                    ),
                },
            },
            "weather" | "ambience" => match words.next() {
                None => format!(
                    "ambience {}\navailable: {}\n",
                    self.weather.name(),
                    weather::PRESET_NAMES.join(", ")
                ),
                Some(name) => match self.weather.set(name) {
                    Ok(()) => {
                        save_state(self.school.count, self.weather.name(), self.renderer.scale());
                        format!("ambience {name}\n")
                    }
                    Err(error) => format!("nagomi: {error}\n"),
                },
            },
            "scale" => match words.next() {
                None => format!("scale {}\n", self.renderer.scale()),
                Some(value) => match value.parse::<u32>() {
                    Ok(scale) => match self.renderer.set_scale(scale) {
                        Ok(()) => {
                            save_state(
                                self.school.count,
                                self.weather.name(),
                                self.renderer.scale(),
                            );
                            format!("scale {scale}\n")
                        }
                        Err(error) => format!("nagomi: {error}\n"),
                    },
                    Err(_) => "nagomi: scale needs a number between 1 and 4\n".to_string(),
                },
            },
            "scatter" => {
                self.school.scatter();
                "ok\n".to_string()
            }
            "status" => format!(
                "fish {}\nambience {}\nscale {}\npid {}\nuptime {:.0}s\n",
                self.school.count,
                self.weather.name(),
                self.renderer.scale(),
                std::process::id(),
                self.started.elapsed().as_secs_f32()
            ),
            "help" | "--help" | "-h" => control::help(),
            other => format!("nagomi: unknown command '{other}'\n{}", control::help()),
        }
    }
}

impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState];
}

smithay_client_toolkit::delegate_dispatch2!(App);
smithay_client_toolkit::delegate_registry!(App);

fn main() -> Result<()> {
    env_logger::init();

    let args: Vec<String> = std::env::args().skip(1).collect();
    if matches!(
        args.first().map(String::as_str),
        Some("help" | "--help" | "-h")
    ) {
        print!("{}", control::help());
        return Ok(());
    }
    if !args.is_empty() {
        std::process::exit(control::run_client(&args));
    }

    let dump_path = std::env::var("NAGOMI_DUMP").ok();
    if dump_path.is_none()
        && std::os::unix::net::UnixStream::connect(control::socket_path()).is_ok()
    {
        eprintln!("nagomi: wallpaper already running");
        return Ok(());
    }

    let conn = Connection::connect_to_env()?;
    let (globals, event_queue) = registry_queue_init(&conn)?;
    let qh = event_queue.handle();

    let compositor_state = CompositorState::bind(&globals, &qh)?;
    let layer_shell = LayerShell::bind(&globals, &qh)?;
    let output_state = OutputState::new(&globals, &qh);
    let seat_state = SeatState::new(&globals, &qh);

    let surface = compositor_state.create_surface(&qh);
    let layer = layer_shell.create_layer_surface(
        &qh,
        surface,
        Layer::Background,
        Some("nagomi-pond"),
        None,
    );
    layer.set_anchor(Anchor::all());
    layer.set_exclusive_zone(-1);
    layer.set_keyboard_interactivity(KeyboardInteractivity::None);
    layer.set_margin(0, 0, 0, 0);
    layer.commit();

    let mut instance_descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    instance_descriptor.backends = wgpu::Backends::all();
    let instance = wgpu::Instance::new(instance_descriptor);
    let surface_id = layer.wl_surface().id();
    let display_ptr = surface_id
        .display_ptr()
        .expect("wayland display pointer unavailable");
    let window_ptr = surface_id.as_ptr();
    let raw_display_handle =
        RawDisplayHandle::Wayland(WaylandDisplayHandle::new(display_ptr.cast()));
    let raw_window_handle = RawWindowHandle::Wayland(WaylandWindowHandle::new(
        NonNull::new(window_ptr.cast()).expect("surface proxy pointer"),
    ));
    let wgpu_surface = unsafe {
        instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
            raw_display_handle: Some(raw_display_handle),
            raw_window_handle,
        })?
    };

    let (saved_fish, saved_ambience, saved_scale) = load_state();
    let initial_scale = saved_scale
        .or_else(|| {
            std::env::var("NAGOMI_SCALE")
                .ok()
                .and_then(|value| value.parse::<u32>().ok())
        })
        .unwrap_or(4)
        .clamp(1, 4);
    let renderer =
        pollster::block_on(gpu::Renderer::new(instance, wgpu_surface, initial_scale));
    let config = FishConfig::default();
    let mut school = School::new(config);
    let mut weather = weather::Weather::new();
    if let Some(name) = saved_ambience {
        let _ = weather.set(&name);
    }
    if let Ok(name) = std::env::var("NAGOMI_WEATHER") {
        if weather.set(&name).is_ok() {
            weather.snap();
        }
    }
    if let Some(count) = saved_fish {
        school.count = count.clamp(1, sim::MAX_FISH);
    }

    let mut app = App {
        renderer,
        layer,
        layer_shell,
        compositor_state,
        registry_state: RegistryState::new(&globals),
        output_state,
        seat_state,
        pointer: None,
        pending_call: None,
        width: 0,
        height: 0,
        configured: false,
        exit: false,
        dump_path: dump_path.clone(),
        school,
        draw: KoiDraw::new(),
        lotus: lotus::Lotus::new(),
        lotus_frame: lotus::LotusFrame::new(),
        duckweed: duckweed::Duckweed::new(),
        duckweed_frame: duckweed::DuckweedFrame::new(),
        tiny_frame: tiny::TinyFrame::new(),
        butterflies: butterflies::Butterflies::new(),
        butterfly_frame: butterflies::ButterflyFrame::new(),
        weather,
        config,
        accumulator: 0.0,
        sim_time: 0.0,
        last_sim: Instant::now(),
        started: Instant::now(),
    };

    let mut event_loop: EventLoop<App> = EventLoop::try_new()?;
    WaylandSource::new(conn.clone(), event_queue).insert(event_loop.handle())?;

    if dump_path.is_none() {
        let listener = control::listen()?;
        event_loop.handle().insert_source(
            calloop::generic::Generic::new(
                listener,
                calloop::Interest::READ,
                calloop::Mode::Level,
            ),
            |_, listener, app: &mut App| {
                app.handle_commands(listener);
                Ok(calloop::PostAction::Continue)
            },
        )?;
    }

    let mut last_frame = Instant::now();
    loop {
        event_loop.dispatch(Duration::from_millis(8), &mut app)?;
        if app.exit {
            break;
        }
        if app.configured {
            if let Some(path) = app.dump_path.clone() {
                for _ in 0..600 {
                    app.school.update(FIXED_STEP, app.sim_time);
                    app.sim_time += FIXED_STEP;
                }
                app.rebuild_fish();
                let (ripple_data, ripple_count) = app.ripple_uniforms();
                let time = app.started.elapsed().as_secs_f32();
                app.lotus.update(time, &mut app.lotus_frame);
                app.duckweed.update(time, &mut app.duckweed_frame);
                tiny::draw(&app.school.tiny, &mut app.tiny_frame);
                app.butterflies.draw(time, &mut app.butterfly_frame);
                app.renderer.dump_pond(
                    &path,
                    time,
                    &ripple_data,
                    ripple_count,
                    FishFrame {
                        outer: &app.draw.outer.vertices,
                        body: &app.draw.body.vertices,
                        shadow: &app.draw.shadow.vertices,
                        disturbance: &app.draw.disturbance,
                    },
                    SurfaceFrame {
                        shadow: &app.lotus_frame.shadow,
                        leaves: &app.lotus_frame.leaves,
                        veins: &app.lotus_frame.veins,
                        flowers: &app.lotus_frame.flowers,
                        duckweed_shadow: &app.duckweed_frame.shadow,
                        duckweed_leaves: &app.duckweed_frame.leaves,
                        duckweed_highlights: &app.duckweed_frame.highlights,
                        tiny_shadow: &app.tiny_frame.shadow,
                        tiny_shapes: &app.tiny_frame.shapes,
                        tiny_details: &app.tiny_frame.details,
                        butterfly_shadow: &app.butterfly_frame.shadow,
                        butterflies: &app.butterfly_frame.shapes,
                    },
                    app.weather.current(),
                )?;
                break;
            }
        }
        // Fixed-step simulation, independent of the wallpaper frame rate.
        let now = Instant::now();
        let delta = (now - app.last_sim).as_secs_f32().min(0.1);
        app.last_sim = now;
        app.weather.update(delta);
        app.school
            .ripples
            .set_rain_intensity(app.weather.current().rain.clamp(0.0, 1.0));
        app.accumulator += delta;
        while app.accumulator >= FIXED_STEP {
            app.school.update(FIXED_STEP, app.sim_time);
            app.butterflies.update(FIXED_STEP, app.sim_time);
            app.sim_time += FIXED_STEP;
            app.accumulator -= FIXED_STEP;
        }
        if let Some((x, y)) = app.pending_call.take() {
            app.school.call_to(sim::vec2(x, y));
        }

        if app.configured && last_frame.elapsed() >= FRAME_INTERVAL {
            app.rebuild_fish();
            let (ripple_data, ripple_count) = app.ripple_uniforms();

            let time = app.started.elapsed().as_secs_f32();
            app.lotus.update(time, &mut app.lotus_frame);
            app.renderer.draw(
                time,
                &ripple_data,
                ripple_count,
                FishFrame {
                    outer: &app.draw.outer.vertices,
                    body: &app.draw.body.vertices,
                    shadow: &app.draw.shadow.vertices,
                    disturbance: &app.draw.disturbance,
                },
                SurfaceFrame {
                    shadow: &app.lotus_frame.shadow,
                    leaves: &app.lotus_frame.leaves,
                    veins: &app.lotus_frame.veins,
                    flowers: &app.lotus_frame.flowers,
                    duckweed_shadow: &app.duckweed_frame.shadow,
                    duckweed_leaves: &app.duckweed_frame.leaves,
                    duckweed_highlights: &app.duckweed_frame.highlights,
                    tiny_shadow: &app.tiny_frame.shadow,
                    tiny_shapes: &app.tiny_frame.shapes,
                    tiny_details: &app.tiny_frame.details,
                    butterfly_shadow: &app.butterfly_frame.shadow,
                    butterflies: &app.butterfly_frame.shapes,
                },
                app.weather.current(),
            );
            last_frame = Instant::now();
        }
    }

    Ok(())
}
