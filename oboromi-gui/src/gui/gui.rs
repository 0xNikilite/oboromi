use eframe::egui::{ScrollArea, CentralPanel};
use oboromi_core::tests::run::run_tests;

/// Main GUI application with test execution capabilities
pub struct GUI {
    pub logs: Vec<String>,
    pub test_thread: Option<std::thread::JoinHandle<Vec<String>>>,
}

impl Default for GUI {
    fn default() -> Self {
        Self { 
            logs: vec![],
            test_thread: None,
        }
    }
}

impl eframe::App for GUI {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.heading("oboromi");
            
            if let Some(handle) = self.test_thread.take() {
                self.logs = handle.join().unwrap();
            }
            
            if self.test_thread.is_none() {
                if ui.button("🧪 Run Dynarmic Tests").clicked() {
                    let ctx = ctx.clone();
                    self.test_thread = Some(std::thread::spawn(move || {
                        ctx.request_repaint();
                        run_tests()
                    }));
                    self.logs = vec![
                        "Warming up JIT compiler (no timeout)...".to_string(),
                        "Running tests...".to_string()
                    ];
                }
            } else {
                ui.label("⏳ Compiling and testing...");
            }

            ui.separator();
            ui.label("Test Results:");
            ScrollArea::vertical().show(ui, |ui| {
                for line in &self.logs {
                    ui.label(line);
                }
            });
        });
    }
}