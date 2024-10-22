extern crate native_windows_gui as nwg;
extern crate native_windows_derive as nwd;

use nwg::NativeUi;
use nwd::NwgUi;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::cmp::Ordering;

use nwg::{MessageButtons, MessageChoice, MessageIcons, MessageParams};

#[derive(Default, NwgUi)]
pub struct ADBAppManager {
    #[nwg_control(size: (400, 350), position: (300, 300), title: "ADB App Manager")]
    #[nwg_events(OnWindowClose: [ADBAppManager::exit])]
    window: nwg::Window,

    #[nwg_control(
        parent: window,
        text: "Check Device Connection",
        size: (180, 30),
        position: (10, 10)
    )]
    #[nwg_events(OnButtonClick: [ADBAppManager::check_device_connection])]
    check_btn: nwg::Button,

    #[nwg_control(
        parent: window,
        text: "No device connected",
        size: (380, 30),
        position: (10, 50)
    )]
    status_label: nwg::Label,

    #[nwg_control(
        parent: window,
        placeholder_text: Some("Search for an app..."),
        size: (380, 30),
        position: (10, 90)
    )]
    #[nwg_events(OnTextInput: [ADBAppManager::filter_apps])]
    search_line_edit: nwg::TextInput,

    #[nwg_control(parent: window, size: (380, 150), position: (10, 130))]
    app_listbox: nwg::ListBox<String>,

    #[nwg_control(
        parent: window,
        text: "Remove Selected App",
        size: (180, 30),
        position: (10, 290)
    )]
    #[nwg_events(OnButtonClick: [ADBAppManager::remove_app])]
    remove_btn: nwg::Button,

    // Data storage for installed apps
    all_apps: Arc<Mutex<Vec<String>>>,
}

impl ADBAppManager {
    fn exit(&self) {
        nwg::stop_thread_dispatch();
    }

    fn check_device_connection(&self) {
        let output = Command::new("adb")
            .arg("devices")
            .output()
            .expect("Failed to execute adb devices command");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let devices_output: Vec<&str> = stdout.lines().collect();

        if devices_output.len() > 1 && devices_output[1].contains("device") {
            let device_id = devices_output[1].split_whitespace().next().unwrap_or("");

            // Get the device name (model)
            let device_name_output = Command::new("adb")
                .args(&["shell", "getprop", "ro.product.model"])
                .output()
                .expect("Failed to execute adb shell getprop command");

            let device_name = String::from_utf8_lossy(&device_name_output.stdout)
                .trim()
                .to_string();

            // Update the status label
            self.status_label
                .set_text(&format!("Device Connected: {} ({})", device_name, device_id));

            // List installed apps
            self.list_installed_apps();
        } else {
            self.status_label.set_text("No device connected");
        }
    }

    fn list_installed_apps(&self) {
        let output = Command::new("adb")
            .args(&["shell", "pm", "list", "packages"])
            .output()
            .expect("Failed to execute adb shell pm list packages");

        let stdout = String::from_utf8_lossy(&output.stdout);

        let mut packages: Vec<String> = stdout
            .lines()
            .map(|line| line.replace("package:", ""))
            .collect();

        // Sort packages alphabetically
        packages.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));

        // Update the all_apps data
        {
            let mut all_apps = self.all_apps.lock().unwrap();
            *all_apps = packages.clone();
        }

        // Update the listbox
        self.update_listbox(&packages);
    }

    fn update_listbox(&self, apps: &Vec<String>) {
        self.app_listbox.clear();
        for app in apps {
            self.app_listbox.push(app.clone());
        }
    }

    fn filter_apps(&self) {
        let search_text = self.search_line_edit.text().to_lowercase();

        let all_apps = self.all_apps.lock().unwrap();
        let filtered_apps: Vec<String> = all_apps
            .iter()
            .filter(|app| app.to_lowercase().contains(&search_text))
            .cloned()
            .collect();

        self.update_listbox(&filtered_apps);
    }

    fn remove_app(&self) {
        let selected_index = self.app_listbox.selection();

        if let Some(index) = selected_index {
            let app_name = self.app_listbox.collection()[index].clone();

            let msg = format!("Are you sure you want to remove {}?", app_name);

            let params = MessageParams {
                title: "Confirm Uninstall",
                content: &msg,
                buttons: MessageButtons::YesNo,
                icons: MessageIcons::Question,
            };

            let result = nwg::modal_message(self.window.handle, &params);

            if result == MessageChoice::Yes {
                let output = Command::new("adb")
                    .args(&["shell", "pm", "uninstall", "--user", "0", &app_name])
                    .output()
                    .expect("Failed to execute adb shell pm uninstall");

                let stdout = String::from_utf8_lossy(&output.stdout);

                if stdout.contains("Success") {
                    nwg::simple_message(
                        "Success",
                        &format!("{} uninstalled successfully!", app_name),
                    );
                    self.list_installed_apps();
                } else {
                    nwg::simple_message("Failed", &format!("Failed to uninstall {}", app_name));
                }
            } else {
                nwg::simple_message(
                    "Aborted",
                    &format!("Uninstallation of {} was aborted.", app_name),
                );
            }
        }
    }
}

fn main() {
    nwg::init().expect("Failed to initialize Native Windows GUI");

    let app = ADBAppManager {
        all_apps: Arc::new(Mutex::new(vec![])),
        ..Default::default()
    };

    let _ui = ADBAppManager::build_ui(app).expect("Failed to build UI");

    nwg::dispatch_thread_events();
}
