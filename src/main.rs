#[macro_use] extern crate failure; use failure::Error;
extern crate discord_rpc_client as rpc;
extern crate time as extern_time;
extern crate serde_json as json;

use std::{ thread, time, process };

use iui::prelude::*;
use iui::controls::{ Button, Checkbox, VerticalBox, Group, Combobox, Entry, Label };

use rpc::Client as DiscordRPC;
use rpc::models::{ Activity };

use settingsfile::{ Settings, SupportedType, SettingsRaw, Format };

#[derive(Clone)]
struct Configuration { }
impl Format for Configuration {
    fn filename(&self) -> String { "config.json".into() }
    fn folder(&self) -> String { ".discord_rpc".into() }

    fn from_str<T>(&self,buffer:&str) -> Result<SettingsRaw,Error> 
        where T : Format + Clone {
        let result : Result<SettingsRaw,json::Error> = json::de::from_str(&buffer);
        
        match result {
            Ok(result) => Ok(result),
            Err(error) => Err(format_err!("{}",error)),
        }
    }

    fn to_string<T:Sized>(&self,object:&T) -> Result<String,Error>
        where T : SupportedType + serde::ser::Serialize, {
        let result : Result<String,json::Error> = json::ser::to_string(object);

        match result {
            Ok(result) => Ok(result),
            Err(error) => Err(format_err!("{}",error)),
        }
    }
}

fn main() {
    // defining required variables for the ui
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
        settings.set_value("state", &entry).unwrap();
    });

    // handling the "details" control, its pretty much the same as the "state" control
    details_entry.set_value(&ui, &settings.get_value_or("details", "This is the higher text!").to_string());
    details_entry.on_changed(&ui, |entry| {
        settings.set_value("details", &entry).unwrap();
    });

    // defining timer controls
    let mut timer_vbox = VerticalBox::new(&ui);
    let mut timer_group = Group::new(&ui, "Timer");
    let mut timer_countdown_group = Group::new(&ui, "Countdown Options");

    // making a combobox for the timer type to decide if its going to be normal or a countdown
    let mut timer_type = Combobox::new(&ui);
    timer_type.append(&ui, "Normal");
    timer_type.append(&ui, "Countdown");
    timer_type.set_selected(&ui, match &*settings.get_value_or("timer.type", "normal").to_string() {
        "normal" => 206158430208,
        "countdown" => 206158430209,
        _ => 206158430208
    });
    timer_type.on_selected(&ui, |value| {
        match value {
            206158430208 => {
                timer_countdown_group.hide(&ui);
                settings.set_value("timer.type", "normal").unwrap();
            },
            206158430209 => {
                timer_countdown_group.show(&ui);
                settings.set_value("timer.type", "countdown").unwrap();
            },
            _ => settings.set_value("timer.type", "normal").unwrap()
        }
    });

    // checking if the timer should be enabled or not
    let mut timer_check = Checkbox::new(&ui, "Enable the timer");
    timer_check.set_checked(&ui, settings.get_value_or("timer.enabled", &false).to_switch().unwrap());
    timer_check.on_toggled(&ui, |checked| {
        settings.set_value("timer.enabled", &checked).unwrap();
        if !checked {
            // timer_type.hide(&ui);
        } else {
            // timer_type.show(&ui);
        }
    });

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
            let time_multiplier = match settings.get_value("timer.duration_type") {
                Some(value) => {
                    match value.to_string().to_lowercase().as_ref() {
                        "hours"   | "houre"  | "hor" | "h" => 3600,
                        "minutes" | "minute" | "min" | "m" => 60,
                        "seconds" | "second" | "sec" | "s" => 1,
                        _ => {
                            println!("The entered duration type is invalid, defaulting to seconds");
                            1
                        }
                    }
                },
                None => {
                    println!("You didn't enter a duration type, defaulting to seconds");
                    1
                }
            };

            let timer_duration = match settings.get_value("timer.duration_time") {
                Some(value) => value.to_int().unwrap(),
                None => {
                    println!("Please enter a duration time");
                    0
                }
            };

            let countdown = time_elapsed + (timer_duration * time_multiplier) as u64;
            
            drpc.start();
			
            loop {
                let mut activity = Activity::new().assets(|asset| asset.large_image("large_image").large_text("Creator: ZackGabri#7771"));
            
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

    text_vbox.append(&ui, details_label, LayoutStrategy::Compact);
    text_vbox.append(&ui, details_entry, LayoutStrategy::Compact);

    text_vbox.append(&ui, state_label, LayoutStrategy::Compact);
    text_vbox.append(&ui, state_entry, LayoutStrategy::Compact);

    text_group.set_child(&ui, text_vbox);
    vbox.append(&ui, text_group, LayoutStrategy::Compact);

    timer_vbox.append(&ui, timer_check, LayoutStrategy::Compact);
    timer_vbox.append(&ui, timer_type, LayoutStrategy::Compact);
    timer_vbox.append(&ui, timer_countdown_group, LayoutStrategy::Compact);
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