use winit::dpi::LogicalSize;
use winit::event::Event;
use winit::event::WindowEvent;
use winit::event_loop::ControlFlow;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

use clipboard::{ClipboardContext, ClipboardProvider};
use imgui::{ClipboardBackend, ImStr, ImString};

use crate::rendering_context::RenderingContext;
use crate::application::Application;

struct ClipboardSupport(ClipboardContext);

impl ClipboardSupport {
    pub fn new() -> Option<Self> {
        ClipboardContext::new().ok().map(|ctx| Self(ctx))
    }
}

impl ClipboardBackend for ClipboardSupport {
    fn get(&mut self) -> Option<ImString> {
        self.0.get_contents().ok().map(|text| text.into())
    }
    fn set(&mut self, text: &ImStr) {
        let _ = self.0.set_contents(text.to_str().to_owned());
    }
}

pub struct Context<App> {
    window: winit::window::Window,
    rendering_context: RenderingContext,
    imgui: imgui::Context,
    imgui_platform: imgui_winit_support::WinitPlatform,
    imgui_renderer: imgui_wgpu_rs::Renderer,
    app: App,
}

impl<App> Context<App>
where
    App: 'static + Application,
{
    pub fn run_application(mut self, event_loop: EventLoop<()>) {
        event_loop.run(move |event, _, control_flow| {
            if !self.handle_event(&event) {
                *control_flow = ControlFlow::Exit;
            }
        });
    }
    fn handle_event(&mut self, event: &winit::event::Event<'_, ()>) -> bool {
        self.app.handle_event(event);
        if self.app.is_exit() {
            return false;
        }
        self.imgui_platform
            .handle_event(self.imgui.io_mut(), &self.window, event);
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    return false;
                }
                WindowEvent::Resized(size) => {
                    self.rendering_context
                        .resize_request(size.width, size.height);
                }
                _ => {}
            },
            Event::RedrawEventsCleared => {
                self.imgui_platform
                    .prepare_frame(self.imgui.io_mut(), &self.window)
                    .expect("failed to prepare frame.");

                self.app.update(
                    &self.rendering_context.device,
                    &self.rendering_context.queue,
                );
                let ui = self.imgui.frame();
                self.app.build_ui(&ui);

                let frame = self.rendering_context.get_current_frame();
                self.imgui_platform.prepare_render(&ui, &self.window);
                let mut encoder = self
                    .rendering_context
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                {
                    let _clear_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                            attachment: &frame.output.view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(self.app.clear_color()),
                                store: true,
                            },
                        }],
                        depth_stencil_attachment: None,
                    });
                }

                self.app.render(&frame, &mut encoder);
                {
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                            attachment: &frame.output.view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: true,
                            },
                        }],
                        depth_stencil_attachment: None,
                    });
                    self.imgui_renderer.render(
                        &self.rendering_context.queue,
                        &mut rpass,
                        ui.render(),
                    );
                }
                self.rendering_context.queue.submit(Some(encoder.finish()));
            }
            _ => {}
        }
        return true;
    }

    pub fn new<T: Into<String>, E>(
        event_loop: &winit::event_loop::EventLoop<E>,
        title: T,
        width: u32,
        height: u32,
        font_data:&[u8],
    ) -> Self {
        let window = WindowBuilder::new()
            .with_inner_size(LogicalSize { width, height })
            .with_title(title)
            .build(event_loop)
            .unwrap();
        let rendering_context =
            futures::executor::block_on(RenderingContext::setup(&window, wgpu::Features::empty()));

        let mut imgui = imgui::Context::create();
        if let Some(backend) = ClipboardSupport::new() {
            imgui.set_clipboard_backend(Box::new(backend));
        }
        let mut imgui_platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
        imgui_platform.attach_window(
            imgui.io_mut(),
            &window,
            imgui_winit_support::HiDpiMode::Rounded,
        );
        imgui.fonts().add_font(&[imgui::FontSource::TtfData {
            data: font_data,
            size_pixels: 15.0,
            config: Some(imgui::FontConfig {
                rasterizer_multiply: 1.75,
                glyph_ranges: imgui::FontGlyphRanges::japanese(),
                ..imgui::FontConfig::default()
            }),
        }]);
        let imgui_renderer = imgui_wgpu_rs::Renderer::new(
            &mut imgui,
            &rendering_context.device,
            &rendering_context.queue,
            rendering_context.sc_desc.format,
        );
        let app = App::new(
            &rendering_context.device,
            &rendering_context.queue,
            &mut imgui,
        );
        Self {
            window,
            rendering_context,
            imgui,
            imgui_platform,
            imgui_renderer,
            app,
        }
    }
}
