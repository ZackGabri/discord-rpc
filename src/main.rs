#[macro_use] extern crate failure; use failure::Error;
extern crate discord_rpc_client as rpc;
extern crate time as extern_time;
extern crate serde_json as json;
use std::{ thread, time };

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
        
        println!("from_str result: {:?}",result);

        match result {
            Ok(result) => Ok(result),
            Err(error) => Err(format_err!("{}",error)),
        }
    }

    fn to_string<T:Sized>(&self,object:&T) -> Result<String,Error>
        where T : SupportedType + serde::ser::Serialize, {
        let result : Result<String,json::Error> = json::ser::to_string(object);
        
        println!("to_string result: {:?}",result);

        match result {
            Ok(result) => Ok(result),
            Err(error) => Err(format_err!("{}",error)),
        }
    }
}

fn main() {

    let mut settings = Settings::new_and_load(Configuration{});


    // getting client id of the options, maybe later there will be an option to change this so i will just keep it like that for now
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

    // using an infinity loop so the activity doesn't reset right away
    loop {
        let mut activity = Activity::new();
        
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

        drpc.set_activity(|_| activity).expect("Ooops, couldn't set the activity );");

        // load the settings again so if something changed the program will have track of it
        settings.load().unwrap();

        // wait 15 seconds before iterating again
        thread::sleep(time::Duration::from_secs(15));
    };
}