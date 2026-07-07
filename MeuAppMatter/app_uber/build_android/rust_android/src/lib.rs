use eframe::{egui, NativeOptions};

#[no_mangle]
fn android_main(app: android_activity::AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("UberRealAndroid")
    );

    log::info!("Starting Matter native Android runner...");

    let options = NativeOptions {
        android_app: Some(app),
        ..Default::default()
    };

    eframe::run_native(
        "UberRealAndroid",
        options,
        Box::new(|cc| {
            Ok(Box::new(MatterAndroidApp::new(cc)))
        }),
    ).unwrap();
}

struct MatterAndroidApp {
    status: String,
    completed_rides: u32,
    revenue: f64,
}

impl MatterAndroidApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            status: "Online - Ready".to_string(),
            completed_rides: 0,
            revenue: 0.0,
        }
    }
}

impl eframe::App for MatterAndroidApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("🌋 Matter Core Android App");
                ui.add_space(20.0);
                
                ui.group(|ui| {
                    ui.label(format!("Status: {}", self.status));
                    ui.label(format!("Revenue: ${:.2}", self.revenue));
                    ui.label(format!("Completed Rides: {}", self.completed_rides));
                });

                ui.add_space(30.0);
                
                if ui.button("Match Simulation Ride").clicked() {
                    self.status = "Driver Carlos matched!".to_string();
                    self.completed_rides += 1;
                    self.revenue += 71.0;
                }
            });
        });
    }
}
