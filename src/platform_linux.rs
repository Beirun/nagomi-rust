
use std::{
    ptr::NonNull,
    time::{Duration, Instant},
};

use anyhow::Result;

use crate::commands;
use crate::control;
use crate::gpu;
use crate::sim;
use crate::state;
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

use crate::scene::{self, Scene};
use sim::FishConfig;


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

    scene: Scene,
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
    fn handle_commands(&mut self, listener: &std::os::unix::net::UnixListener) {
        use std::io::Write;
        loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let Some(request) = control::read_request(&stream) else {
                        continue;
                    };
                    let response = commands::handle(
                        request.trim(),
                        &mut self.scene,
                        &mut self.renderer,
                        self.started,
                    );
                    let _ = stream.write_all(response.as_bytes());
                    let _ = stream.flush();
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
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

pub fn run() -> Result<()> {

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

    let saved = state::SavedState::load();
    let initial_scale = saved.scale
        .or_else(|| {
            std::env::var("NAGOMI_SCALE")
                .ok()
                .and_then(|value| value.parse::<u32>().ok())
        })
        .unwrap_or(4)
        .clamp(1, 4);
    let renderer =
        pollster::block_on(gpu::Renderer::new(instance, wgpu_surface, initial_scale));
    let mut scene = Scene::new(FishConfig::default());
    saved.apply(&mut scene);
    if let Ok(name) = std::env::var("NAGOMI_WEATHER") {
        if scene.weather.set(&name).is_ok() {
            scene.weather.snap();
        }
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
        scene,
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
                    app.scene.school.update(sim::FIXED_STEP, 0.0);
                }
                if let Ok(dir) = std::env::var("NAGOMI_VIDEO_DIR") {
                    let frames: usize = std::env::var("NAGOMI_VIDEO_FRAMES")
                        .ok()
                        .and_then(|value| value.parse().ok())
                        .unwrap_or(300);
                    for frame in 0..frames {
                        app.scene.tick(1.0 / 30.0);
                        let time = app.scene.time();
                        app.scene.build_frame(time);
                        let (ripple_data, ripple_count) = app.scene.ripple_uniforms();
                        let (fish_frame, surface_frame) = app.scene.frames();
                        let weather = *app.scene.weather.current();
                        let path = format!("{dir}/frame{frame:05}.ppm");
                        app.renderer.dump_pond(
                            &path,
                            time,
                            &ripple_data,
                            ripple_count,
                            fish_frame,
                            surface_frame,
                            &weather,
                        )?;
                    }
                    break;
                }
                let (ripple_data, ripple_count) = app.scene.ripple_uniforms();
                let time = app.started.elapsed().as_secs_f32();
                app.scene.build_frame(time);
                let (fish_frame, surface_frame) = app.scene.frames();
                let weather = *app.scene.weather.current();
                app.renderer.dump_pond(
                    &path,
                    time,
                    &ripple_data,
                    ripple_count,
                    fish_frame,
                    surface_frame,
                    &weather,
                )?;
                break;
            }
        }
        // Fixed-step simulation, independent of the wallpaper frame rate.
        app.scene.advance(Instant::now());
        if let Some((x, y)) = app.pending_call.take() {
            app.scene.school.call_to(sim::vec2(x, y));
        }

        if app.configured && last_frame.elapsed() >= scene::FRAME_INTERVAL {
            let (ripple_data, ripple_count) = app.scene.ripple_uniforms();
            let time = app.started.elapsed().as_secs_f32();
            app.scene.build_frame(time);
            let (fish_frame, surface_frame) = app.scene.frames();
            let weather = *app.scene.weather.current();
            app.renderer.draw(
                time,
                &ripple_data,
                ripple_count,
                fish_frame,
                surface_frame,
                &weather,
            );
            last_frame = Instant::now();
        }
    }

    Ok(())
}
