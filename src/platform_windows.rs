//! Windows front end: a borderless window parented under Explorer's desktop
//! WorkerW, presenting the pond through wgpu.

use std::sync::{mpsc, Arc};
use std::time::Instant;

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use crate::commands;
use crate::control::{self, Request};
use crate::gpu;
use crate::scene::{self, Scene};
use crate::sim::FishConfig;
use crate::state::SavedState;

pub fn run() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if matches!(
        args.first().map(String::as_str),
        Some("help" | "--help" | "-h")
    ) {
        print!("{}", control::help());
        return;
    }
    if !args.is_empty() {
        std::process::exit(control::run_client(&args));
    }

    if std::net::TcpStream::connect(("127.0.0.1", control::PORT)).is_ok() {
        eprintln!("nagomi: wallpaper already running");
        return;
    }
    let requests = control::spawn_server();

    let event_loop = EventLoop::new().expect("event loop");
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::new(requests);
    if let Err(error) = event_loop.run_app(&mut app) {
        eprintln!("nagomi: {error}");
    }
}

struct App {
    requests: Option<mpsc::Receiver<Request>>,
    window: Option<Arc<Window>>,
    renderer: Option<gpu::Renderer>,
    scene: Scene,
    started: Instant,
    next_frame: Instant,
    scale: u32,
}

impl App {
    fn new(requests: Option<mpsc::Receiver<Request>>) -> Self {
        let saved = SavedState::load();
        let mut scene = Scene::new(FishConfig::default());
        saved.apply(&mut scene);
        Self {
            requests,
            window: None,
            renderer: None,
            scene,
            started: Instant::now(),
            next_frame: Instant::now(),
            scale: saved.scale.unwrap_or(4).clamp(1, 4),
        }
    }

    fn pump_commands(&mut self) {
        loop {
            let Some(request) = self
                .requests
                .as_ref()
                .and_then(|requests| requests.try_recv().ok())
            else {
                break;
            };
            let response = match self.renderer.as_mut() {
                Some(renderer) => commands::handle(
                    &request.text,
                    &mut self.scene,
                    renderer,
                    self.started,
                ),
                None => "nagomi: still starting\n".to_string(),
            };
            let _ = request.reply.send(response);
        }
    }

    fn render(&mut self) {
        let (Some(renderer), Some(_)) = (self.renderer.as_mut(), self.window.as_ref()) else {
            return;
        };
        self.scene.advance(Instant::now());
        let (ripple_data, ripple_count) = self.scene.ripple_uniforms();
        let time = self.started.elapsed().as_secs_f32();
        self.scene.build_frame(time);
        let (fish_frame, surface_frame) = self.scene.frames();
        let weather = self.scene.graded_weather();
        renderer.draw(
            time,
            &ripple_data,
            ripple_count,
            fish_frame,
            surface_frame,
            &weather,
        );
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let monitor_size = event_loop
            .primary_monitor()
            .or_else(|| event_loop.available_monitors().next())
            .map(|monitor| monitor.size())
            .unwrap_or(PhysicalSize::new(1920, 1080));
        let attrs = Window::default_attributes()
            .with_title("nagomi")
            .with_decorations(false)
            .with_resizable(false)
            .with_inner_size(monitor_size)
            .with_position(PhysicalPosition::new(0, 0));
        let window = match event_loop.create_window(attrs) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                eprintln!("nagomi: cannot create window: {error}");
                event_loop.exit();
                return;
            }
        };

        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        descriptor.backends = wgpu::Backends::all();
        let instance = wgpu::Instance::new(descriptor);

        let surface = match instance.create_surface(window.clone()) {
            Ok(surface) => surface,
            Err(error) => {
                eprintln!("nagomi: cannot create surface: {error}");
                event_loop.exit();
                return;
            }
        };
        let mut renderer = pollster::block_on(gpu::Renderer::new(instance, surface, self.scale));
        let size = window.inner_size();
        renderer.configure(size.width.max(1), size.height.max(1));

        attach_to_desktop(&window, size.width.max(1), size.height.max(1));

        window.request_redraw();
        self.renderer = Some(renderer);
        self.window = Some(window);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.configure(size.width.max(1), size.height.max(1));
                }
            }
            WindowEvent::RedrawRequested => self.render(),
            _ => {}
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_frame));
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.pump_commands();
        let now = Instant::now();
        if now >= self.next_frame {
            self.render();
            self.next_frame = now + scene::FRAME_INTERVAL;
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_frame));
    }
}

/// Parent the window under the WorkerW that Explorer uses for the desktop.
fn attach_to_desktop(window: &Window, width: u32, height: u32) {
    use windows::core::{w, BOOL};
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, FindWindowExW, FindWindowW, SendMessageTimeoutW, SetParent, SetWindowPos,
        SMTO_NORMAL, SWP_NOACTIVATE, SWP_NOZORDER, SWP_SHOWWINDOW,
    };

    let Ok(handle) = window.window_handle() else {
        return;
    };
    let RawWindowHandle::Win32(win32) = handle.as_raw() else {
        return;
    };
    let hwnd = HWND(win32.hwnd.get() as *mut _);

    unsafe extern "system" fn find_worker(hwnd: HWND, param: LPARAM) -> BOOL {
        let target = &mut *(param.0 as *mut HWND);
        if FindWindowExW(Some(hwnd), None, w!("SHELLDLL_DefView"), None)
            .map(|view| !view.is_invalid())
            .unwrap_or(false)
        {
            if let Ok(worker) = FindWindowExW(None, Some(hwnd), w!("WorkerW"), None) {
                if !worker.is_invalid() {
                    *target = worker;
                    return BOOL(0);
                }
            }
        }
        BOOL(1)
    }

    unsafe {
        if let Ok(progman) = FindWindowW(w!("Progman"), None) {
            let _ = SendMessageTimeoutW(
                progman,
                0x052C,
                WPARAM(0),
                LPARAM(0),
                SMTO_NORMAL,
                1000,
                None,
            );
            let _ = SendMessageTimeoutW(
                progman,
                0x052C,
                WPARAM(0),
                LPARAM(1),
                SMTO_NORMAL,
                1000,
                None,
            );
            let mut worker = HWND::default();
            let _ = EnumWindows(Some(find_worker), LPARAM(&mut worker as *mut _ as isize));
            let target = if worker.is_invalid() { progman } else { worker };
            let _ = SetParent(hwnd, Some(target));
            let _ = SetWindowPos(
                hwnd,
                None,
                0,
                0,
                width as i32,
                height as i32,
                SWP_NOZORDER | SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
        }
    }
}
