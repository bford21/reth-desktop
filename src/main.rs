use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    
    eframe::run_native(
        "Reth Desktop",
        options,
        Box::new(|_cc| Box::<MyApp>::default()),
    )
}

struct MyApp {
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Reth Desktop");
            
            ui.separator();
            
            if ui.button("Click me!").clicked() {
                // Button doesn't do anything yet
            }
        });
    }
}