pub trait Application {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue, imgui: &mut imgui::Context) -> Self;
    fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {}
    fn render(&mut self, frame: &wgpu::SwapChainFrame, encoder: &mut wgpu::CommandEncoder) {}
    fn build_ui(&mut self, ui: &imgui::Ui) {}
    fn resize(&mut self, width: u32, height: u32) {}
    fn is_exit(&self) -> bool {
        false
    }
    fn handle_event(&mut self, event: &winit::event::Event<()>) {}
    fn clear_color(&self) -> wgpu::Color {
        wgpu::Color::BLACK
    }
}
