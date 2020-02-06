#[macro_use] extern crate failure; use failure::Error;
extern crate discord_rpc_client as rpc;
extern crate time as extern_time;
extern crate serde_json as json;

use std::{ thread, time, process };

use iui::prelude::*;
use iui::controls::{ Button, Checkbox, VerticalBox, Group, Combobox, Entry, Label, Spinbox };

use rpc::Client as DiscordRPC;
use rpc::models::{ Activity };

use settingsfile::{ Settings, SupportedType, SettingsRaw, Format, Type };

#[derive(Clone)]
struct Configuration { }
impl Format for Configuration {
    fn filename(&self) -> String { "config.json".into() }
    fn folder(&self) -> String { ".discord_rpc".into() }

    fn from_str<T>(&self,buffer:&str) -> Result<SettingsRaw,Error> 
        where T : Format + Clone {
        let result : Result<SettingsRaw,json::Error> = json::de::from_str(&buffer);
        
        result.map_err(|e| format_err!("{}", e))
    }

    fn to_string<T:Sized>(&self,object:&T) -> Result<String,Error>
        where T : SupportedType + serde::ser::Serialize, {
        let result : Result<String,json::Error> = json::ser::to_string_pretty(object);

        result.map_err(|e| format_err!("{}", e))
    }
}

fn main() {
    // defining the ui
    let mut settings = Settings::new_and_load(Configuration{});
    let ui = UI::init().expect("Couldn't initialize UI library");
    let mut win = Window::new(&ui, "Discord RPC", 150, 500, WindowType::NoMenubar);
    let mut vbox = VerticalBox::new(&ui);

    // defining "state" (lower text) and "details" (higher text) controls
    let mut text_vbox = VerticalBox::new(&ui);
    let mut text_group = Group::new(&ui, "Text");
    let mut state_entry = Entry::new(&ui);
    let mut details_entry = Entry::new(&ui);
    let details_label = Label::new(&ui, &"Higher text:");
    let state_label = Label::new(&ui, &"Lower text:");
    
    // handling the "state" control
    state_entry.set_value(&ui, &settings.get_value_or("state", "This is the lower text!").to_string());
    state_entry.on_changed(&ui, |entry| {
        if entry.trim().is_empty() || entry.trim().len() < 2 {
            if settings.get_value("state").is_some() {
                settings.delete_key("state").unwrap();
            };
            return;
        }
        settings.set_value("state", &entry).unwrap();
    });

    // handling the "details" control, its pretty much the same as the "state" control
    details_entry.set_value(&ui, &settings.get_value_or("details", "This is the higher text!").to_string());
    details_entry.on_changed(&ui, |entry| {
        if entry.trim().is_empty() || entry.trim().len() < 2 {
            if settings.get_value("details").is_some() {
                settings.delete_key("details").unwrap();
            };
            return;
        }
        settings.set_value("details", &entry).unwrap();
    });

    // defining timer controls
    let mut timer_vbox = VerticalBox::new(&ui);
    let mut timer_group = Group::new(&ui, "Timer");
    let mut countdown_vbox = VerticalBox::new(&ui);
    let mut countdown_group = Group::new(&ui, "Countdown Options");
    let mut hours_spinbox = Spinbox::new(&ui, 24, 0);
    let mut minutes_spinbox = Spinbox::new(&ui, 60, 0);
    let mut seconds_spinbox = Spinbox::new(&ui, 60, 0);
    let hours_label = Label::new(&ui, "Hours:");
    let minutes_label = Label::new(&ui, "Minutes:");
    let seconds_label = Label::new(&ui, "Seconds:");

    // setting the default values for the spinboxes
    hours_spinbox.set_value(&ui, settings.get_value_or("timer.duration_h", "0").to_int().unwrap_or(0) as i64);
    minutes_spinbox.set_value(&ui, settings.get_value_or("timer.duration_m", "0").to_int().unwrap_or(0) as i64);
    seconds_spinbox.set_value(&ui, settings.get_value_or("timer.duration_s", "0").to_int().unwrap_or(0) as i64);

    // saving the new value of one of the spinboxes value got changed
    hours_spinbox.on_changed(&ui, |val| settings.set_value("timer.duration_h", &(val as i32)).unwrap());
    minutes_spinbox.on_changed(&ui, |val| settings.set_value("timer.duration_m", &(val as i32)).unwrap());
    seconds_spinbox.on_changed(&ui, |val| settings.set_value("timer.duration_s", &(val as i32)).unwrap());

    // making a combobox for the timer type to decide if its going to be normal or a countdown
    let mut timer_type = Combobox::new(&ui);
    let timer_label = Label::new(&ui, "Timer type:");
    let empty_timer_label = Label::new(&ui, "");

    // adding the options "Normal" and "Countdown" to the Combobox (Drop list) 
    timer_type.append(&ui, "Normal");
    timer_type.append(&ui, "Countdown");

    timer_type.set_selected(&ui, match &*settings.get_value_or("timer.type", "normal").to_string() {
        "normal" => 206158430208,
        "countdown" => 206158430209,
        _ => 206158430208
    });

    // saving the new option when the Combobox (Drop list) value gets changed
    timer_type.on_selected(&ui, |value| {
        match value {
            206158430208 => settings.set_value("timer.type", "normal").unwrap(),
            206158430209 => settings.set_value("timer.type", "countdown").unwrap(),
            _ => settings.set_value("timer.type", "normal").unwrap()
        }
    });

    // checking if the timer should be enabled or not
    let mut timer_check = Checkbox::new(&ui, "Enable the timer");
    timer_check.set_checked(&ui, settings.get_value_or("timer.enabled", &false).to_switch().unwrap());
    timer_check.on_toggled(&ui, |checked| settings.set_value("timer.enabled", &checked).unwrap());

    let mut start_button = Button::new(&ui, "Start the activity!");
    let mut start_button_clicked = false;
    start_button.on_clicked(&ui, |btn| {

        // a check if the button was clicked before or not
        if !start_button_clicked {
            // if it was not clicked before we set the text to "Running the activity..."
            btn.set_text(&ui, "Running the activity...");
            // and set the "start_button_clicked" to true so it doesn't run again
            start_button_clicked = true;
        } else {
            // if the "start_button_clicked" was true then we return so we don't spawn any more threads
            return;
        }

        // save the settings so any changes will be loaded in the next thread
        settings.save().unwrap();

        // spawning a new thread to run the infinity loop withn, instead of our main thread because running it in the main thread will result in freezing the program
        thread::spawn(move || {
            // getting client id of the options, maybe later there will be an option to change this so i will just keep it like that for now
            let mut settings = Settings::new_and_load(Configuration{});
            let client_id = settings.get_value_or("client_id", &"656753180901638144").to_string();

            // the client requires the id to be of type u64 so i parsed it into that
            let mut drpc = DiscordRPC::new(client_id.parse::<u64>().unwrap());
    
            let time_elapsed = extern_time::get_time().sec as u64;
            let mut time_hours = settings.get_value("timer.duration_h").unwrap_or(Type::Int(0)).to_int().expect("\"timer.duration_h\" was not a number");
            let mut time_minutes = settings.get_value("timer.duration_m").unwrap_or(Type::Int(0)).to_int().expect("\"timer.duration_m\" was not a number");
            let mut time_seconds = settings.get_value("timer.duration_s").unwrap_or(Type::Int(0)).to_int().expect("\"timer.duration_s\" was not a number");

            // add a check if the timer exceed 24 hours, and if so we make it 24 hours, 59 minutes and 59 seconds to prevent discord from starting the timer from 00:00:00
            if time_hours == 24 || time_hours == 23 && (time_seconds > 59 && time_minutes > 59) {
                time_hours = 23; time_minutes = 59; time_seconds = 59;
            }

            let countdown = time_elapsed + ((time_hours * 3600) + (time_minutes * 60) + time_seconds) as u64;
            
            drpc.start();
			settings.save().expect("An error happened when saving the settings");
            loop {
                let mut activity = Activity::new().assets(|asset| asset.large_image("large_image"));
            
                // if "details" is Some/not None we set the details property to it
                // details is the higher text
                if let Some(details) = settings.get_value("details") {
                    activity = activity.details(&details.to_string());
                    println!("Set the details to: {}", &details.to_string())
                } else {
                    println!("No details property has been provided, skipped it");
                }
    
                // doing the same thing for state
                // State is the lower text
                if let Some(state) = settings.get_value("state") {
                    activity = activity.state(&state.to_string());
                    println!("Set the state to: {}", &state.to_string())
                } else {
                    println!("No state property has been provided, skipped it");
                }
            
                // adding a check if there is no timer object at all
                if let None = settings.get_value("timer") {
                    println!("No \"timer\" object was added")
                }
    
                // managing timer reading
                if let Some(enabled) = settings.get_value("timer.enabled") {
                    if !enabled.to_switch().unwrap() {
                        println!("Timer is disabled, no timer will be set");
                    } else {
                        match settings.get_value("timer.type") {
                            Some(timer_type) => {
                                if timer_type.to_string() == "normal" {
                                    activity = activity.timestamps(|timer| timer.start(time_elapsed));
                                } else if timer_type.to_string() == "countdown" {
                                    activity = activity.timestamps(|timer| timer.end(countdown));
                                } else {
                                    println!("invalid timer type, defaulted to \"normal\" timer type");
                                    activity = activity.timestamps(|timer| timer.start(time_elapsed));
                                }
                            },
                            None => println!("Timer was enabled but was not able to find a type, please provide a \"type\" property under the \"timer\" object")
                        }
                    }
                } else {
                    println!("No enabled property for the timer was set, skipping timer")
                }

                drpc.set_activity(|_| activity).unwrap();
				
                // load the settings again so if something changed the program will have track of it
                settings.load().unwrap();
                
                // wait 10 seconds before iterating again
                thread::sleep(time::Duration::from_secs(10));
            };
        });
    });

    // add the "details" and the "state" controls to the ui
    text_vbox.append(&ui, details_label, LayoutStrategy::Compact);
    text_vbox.append(&ui, details_entry, LayoutStrategy::Compact);

    text_vbox.append(&ui, state_label, LayoutStrategy::Compact);
    text_vbox.append(&ui, state_entry, LayoutStrategy::Compact);

    text_group.set_child(&ui, text_vbox);
    vbox.append(&ui, text_group, LayoutStrategy::Compact);

    // add the timer controls to the ui
    timer_vbox.append(&ui, timer_check, LayoutStrategy::Compact);
    timer_vbox.append(&ui, empty_timer_label, LayoutStrategy::Compact);
    timer_vbox.append(&ui, timer_label, LayoutStrategy::Compact);
    timer_vbox.append(&ui, timer_type, LayoutStrategy::Compact);
    // timer countdown
    countdown_vbox.append(&ui, hours_label, LayoutStrategy::Compact);
    countdown_vbox.append(&ui, hours_spinbox, LayoutStrategy::Compact);

    countdown_vbox.append(&ui, minutes_label, LayoutStrategy::Compact);
    countdown_vbox.append(&ui, minutes_spinbox, LayoutStrategy::Compact);

    countdown_vbox.append(&ui, seconds_label, LayoutStrategy::Compact);
    countdown_vbox.append(&ui, seconds_spinbox, LayoutStrategy::Compact);

    countdown_group.set_child(&ui, countdown_vbox);
    timer_vbox.append(&ui, countdown_group, LayoutStrategy::Compact);

    timer_group.set_child(&ui, timer_vbox);
    vbox.append(&ui, timer_group, LayoutStrategy::Compact);

    vbox.append(&ui, start_button, LayoutStrategy::Compact);
    win.set_child(&ui, vbox);

    // handle the window closing
    win.on_closing(&ui, |_win| {
        // save the settings on close so any unsaved changes gets saved
        settings.save().unwrap();

        // exit the process with exit code 0 
        process::exit(0);
    });

    win.show(&ui);
    ui.main();
}