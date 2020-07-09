/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use gleam::gl;
use std::env;
use std::path::PathBuf;
use webrender;
use winit;
use winit::platform::run_return::EventLoopExtRunReturn;
use webrender::{DebugFlags, ShaderPrecacheFlags};
use webrender::api::*;
use webrender::render_api::*;
use webrender::api::units::*;

struct Notifier {
    events_proxy: winit::event_loop::EventLoopProxy<()>,
}

impl Notifier {
    fn new(events_proxy: winit::event_loop::EventLoopProxy<()>) -> Notifier {
        Notifier { events_proxy }
    }
}

impl RenderNotifier for Notifier {
    fn clone(&self) -> Box<dyn RenderNotifier> {
        Box::new(Notifier {
            events_proxy: self.events_proxy.clone(),
        })
    }

    fn wake_up(&self, _composite_needed: bool) {
        #[cfg(not(target_os = "android"))]
        let _ = self.events_proxy.send_event(());
    }

    fn new_frame_ready(&self,
                       _: DocumentId,
                       _scrolled: bool,
                       composite_needed: bool,
                       _: FramePublishId) {
        self.wake_up(composite_needed);
    }
}

pub trait HandyDandyRectBuilder {
    fn to(&self, x2: i32, y2: i32) -> LayoutRect;
    fn by(&self, w: i32, h: i32) -> LayoutRect;
}
// Allows doing `(x, y).to(x2, y2)` or `(x, y).by(width, height)` with i32
// values to build a f32 LayoutRect
impl HandyDandyRectBuilder for (i32, i32) {
    fn to(&self, x2: i32, y2: i32) -> LayoutRect {
        LayoutRect::from_origin_and_size(
            LayoutPoint::new(self.0 as f32, self.1 as f32),
            LayoutSize::new((x2 - self.0) as f32, (y2 - self.1) as f32),
        )
    }

    fn by(&self, w: i32, h: i32) -> LayoutRect {
        LayoutRect::from_origin_and_size(
            LayoutPoint::new(self.0 as f32, self.1 as f32),
            LayoutSize::new(w as f32, h as f32),
        )
    }
}

pub trait Example {
    const TITLE: &'static str = "WebRender Sample App";
    const PRECACHE_SHADER_FLAGS: ShaderPrecacheFlags = ShaderPrecacheFlags::EMPTY;
    const WIDTH: u32 = 1920;
    const HEIGHT: u32 = 1080;

    fn render(
        &mut self,
        api: &mut RenderApi,
        builder: &mut DisplayListBuilder,
        txn: &mut Transaction,
        device_size: DeviceIntSize,
        pipeline_id: PipelineId,
        document_id: DocumentId,
    );
    fn on_event(
        &mut self,
        _: winit::event::WindowEvent,
        _: &winit::window::Window,
        _: &mut RenderApi,
        _: DocumentId,
    ) -> bool {
        false
    }
    fn get_image_handler(
        &mut self,
        _gl: &dyn gl::Gl,
    ) -> Option<Box<dyn ExternalImageHandler>> {
        None
    }
    fn draw_custom(&mut self, _gl: &dyn gl::Gl) {
    }
}

pub fn main_wrapper<E: Example>(
    example: &mut E,
    options: Option<webrender::WebRenderOptions>,
) {
    env_logger::init();

    #[cfg(target_os = "macos")]
    {
        use core_foundation::{self as cf, base::TCFType};
        let i = cf::bundle::CFBundle::main_bundle().info_dictionary();
        let mut i = unsafe { i.to_mutable() };
        i.set(
            cf::string::CFString::new("NSSupportsAutomaticGraphicsSwitching"),
            cf::boolean::CFBoolean::true_value().into_CFType(),
        );
    }

    let args: Vec<String> = env::args().collect();
    let res_path = if args.len() > 1 {
        Some(PathBuf::from(&args[1]))
    } else {
        None
    };

    let mut events_loop = winit::event_loop::EventLoop::new();
    let window_builder = winit::window::WindowBuilder::new()
        .with_title(E::TITLE)
        // .with_multitouch()
        .with_inner_size(winit::dpi::LogicalSize::new(E::WIDTH as f64, E::HEIGHT as f64));
    let window = window_builder.build(&events_loop).unwrap();

    let connection = surfman::Connection::from_winit_window(&window).unwrap();
    let widget = connection.create_native_widget_from_winit_window(&window).unwrap();
    let adapter = connection.create_adapter().unwrap();
    let mut device = connection.create_device(&adapter).unwrap();
    let (major, minor) = match device.gl_api() {
        surfman::GLApi::GL => (3, 2),
        surfman::GLApi::GLES => (3, 0),
    };
    let context_descriptor = device.create_context_descriptor(&surfman::ContextAttributes {
        version: surfman::GLVersion {
            major,
            minor,
        },
        flags: surfman::ContextAttributeFlags::ALPHA |
        surfman::ContextAttributeFlags::DEPTH |
        surfman::ContextAttributeFlags::STENCIL,
    }).unwrap();
    let mut context = device.create_context(&context_descriptor, None).unwrap();
    device.make_context_current(&context).unwrap();

    let gl = match device.gl_api() {
        surfman::GLApi::GL => unsafe {
            gl::GlFns::load_with(
                |symbol| device.get_proc_address(&context, symbol) as *const _
            )
        },
        surfman::GLApi::GLES => unsafe {
            gl::GlesFns::load_with(
                |symbol| device.get_proc_address(&context, symbol) as *const _
            )
        },
    };
    let gl = gl::ErrorCheckingGl::wrap(gl);

    println!("OpenGL version {}", gl.get_string(gl::VERSION));
    println!("Shader resource path: {:?}", res_path);
    let device_pixel_ratio = window.scale_factor() as f32;
    println!("Device pixel ratio: {}", device_pixel_ratio);

    let surface = device.create_surface(
        &context,
        surfman::SurfaceAccess::GPUOnly,
        surfman::SurfaceType::Widget { native_widget: widget },
    ).unwrap();
    device.bind_surface_to_context(&mut context, surface).unwrap();

    println!("Loading shaders...");
    let mut debug_flags = DebugFlags::ECHO_DRIVER_MESSAGES | DebugFlags::TEXTURE_CACHE_DBG;
    let opts = webrender::WebRenderOptions {
        resource_override_path: res_path,
        precache_flags: E::PRECACHE_SHADER_FLAGS,
        clear_color: ColorF::new(0.3, 0.0, 0.0, 1.0),
        debug_flags,
        //allow_texture_swizzling: false,
        ..options.unwrap_or(webrender::WebRenderOptions::default())
    };

    let device_size = {
        let size = window
            .inner_size();
        DeviceIntSize::new(size.width as i32, size.height as i32)
    };
    let notifier = Box::new(Notifier::new(events_loop.create_proxy()));
    let (mut renderer, sender) = webrender::create_webrender_instance(
        gl.clone(),
        notifier,
        opts,
        None,
    ).unwrap();
    let mut api = sender.create_api();
    let document_id = api.add_document(device_size);

    let external = example.get_image_handler(&*gl);

    if let Some(external_image_handler) = external {
        renderer.set_external_image_handler(external_image_handler);
    }

    let epoch = Epoch(0);
    let pipeline_id = PipelineId(0, 0);
    let mut builder = DisplayListBuilder::new(pipeline_id);
    let mut txn = Transaction::new();
    builder.begin();

    example.render(
        &mut api,
        &mut builder,
        &mut txn,
        device_size,
        pipeline_id,
        document_id,
    );
    txn.set_display_list(
        epoch,
        builder.end(),
    );
    txn.set_root_pipeline(pipeline_id);
    txn.generate_frame(0, RenderReasons::empty());
    api.send_transaction(document_id, txn);

    println!("Entering event loop");
    events_loop.run_return(|global_event, _elwt, control_flow| {
        let mut txn = Transaction::new();
        let mut custom_event = true;

        let old_flags = debug_flags;
        let win_event = match global_event {
            winit::event::Event::WindowEvent { event, .. } => event,
            _ => return,
        };
        match win_event {
            winit::event::WindowEvent::CloseRequested => {
                *control_flow = winit::event_loop::ControlFlow::Exit;
                return;
            }
            winit::event::WindowEvent::AxisMotion { .. } |
            winit::event::WindowEvent::CursorMoved { .. } => {
                custom_event = example.on_event(
                    win_event,
                    &window,
                    &mut api,
                    document_id,
                );
                // skip high-frequency events from triggering a frame draw.
                if !custom_event {
                    return;
                }
            },
            winit::event::WindowEvent::KeyboardInput {
                input: winit::event::KeyboardInput {
                    state: winit::event::ElementState::Pressed,
                    virtual_keycode: Some(key),
                    ..
                },
                ..
            } => match key {
                winit::event::VirtualKeyCode::Escape => {
                    *control_flow = winit::event_loop::ControlFlow::Exit;
                    return;
                }
                winit::event::VirtualKeyCode::P => debug_flags.toggle(DebugFlags::PROFILER_DBG),
                winit::event::VirtualKeyCode::O => debug_flags.toggle(DebugFlags::RENDER_TARGET_DBG),
                winit::event::VirtualKeyCode::I => debug_flags.toggle(DebugFlags::TEXTURE_CACHE_DBG),
                winit::event::VirtualKeyCode::T => debug_flags.toggle(DebugFlags::PICTURE_CACHING_DBG),
                winit::event::VirtualKeyCode::Q => debug_flags.toggle(
                    DebugFlags::GPU_TIME_QUERIES | DebugFlags::GPU_SAMPLE_QUERIES
                ),
                winit::event::VirtualKeyCode::G => debug_flags.toggle(DebugFlags::GPU_CACHE_DBG),
                winit::event::VirtualKeyCode::M => api.notify_memory_pressure(),
                winit::event::VirtualKeyCode::C => {
                    let path: PathBuf = "../captures/example".into();
                    //TODO: switch between SCENE/FRAME capture types
                    // based on "shift" modifier, when `glutin` is updated.
                    let bits = CaptureBits::all();
                    api.save_capture(path, bits);
                },
                _ => {
                    custom_event = example.on_event(
                        win_event,
                        &window,
                        &mut api,
                        document_id,
                    )
                },
            },
            other => custom_event = example.on_event(
                other,
                &window,
                &mut api,
                document_id,
            ),
        };

        if debug_flags != old_flags {
            api.send_debug_cmd(DebugCommand::SetFlags(debug_flags));
        }

        if custom_event {
            let mut builder = DisplayListBuilder::new(pipeline_id);
            builder.begin();

            example.render(
                &mut api,
                &mut builder,
                &mut txn,
                device_size,
                pipeline_id,
                document_id,
            );
            txn.set_display_list(
                epoch,
                builder.end(),
            );
            txn.generate_frame(0, RenderReasons::empty());
        }
        api.send_transaction(document_id, txn);

        let framebuffer_object = device
            .context_surface_info(&context)
            .unwrap()
            .unwrap()
            .framebuffer_object;
        gl.bind_framebuffer(gl::FRAMEBUFFER, framebuffer_object);
        assert_eq!(gl.check_frame_buffer_status(gleam::gl::FRAMEBUFFER), gl::FRAMEBUFFER_COMPLETE);

        renderer.update();
        renderer.render(device_size, 0).unwrap();
        let _ = renderer.flush_pipeline_info();
        example.draw_custom(&*gl);

        let mut surface = device.unbind_surface_from_context(&mut context).unwrap().unwrap();
        device.present_surface(&context, &mut surface).unwrap();
        device.bind_surface_to_context(&mut context, surface).unwrap();

        *control_flow = winit::event_loop::ControlFlow::Wait;
    });

    renderer.deinit();

    device.destroy_context(&mut context).unwrap();
}
