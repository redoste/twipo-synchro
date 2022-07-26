/* WARNING : The installer just crash on the first error
 * I did this choice to make sure we don't destroy too much the game in case
 * something goes wrong. To make sure it remains at least playable, we execute
 * the installation steps from the less likely to break to the most likely
 * (e.g. : Add signatures before copying the patched LB DLL)
 */

use std::fs;
use std::io::{self, BufRead, Write};
use std::path::Path;

const EXPECTED_VERSION: &str = "1.0.8";
const CONFIG_KEY: &str = "twipoSynchroListenAddress";
const CONFIG_VALUE: &str = "0.0.0.0:8080";
const VERSION_STRING: &[u8] = b"0.1.0\n";

fn flush() {
    std::io::stdout().flush().unwrap();
}

fn read() -> String {
    // rust when you don't want to catch errors
    io::stdin().lock().lines().next().unwrap().unwrap()
}

fn pause() {
    if cfg!(windows) {
        read();
    }
}

fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn save_json(path: &Path, value: &serde_json::Value) {
    let mut file = fs::File::create(path).unwrap();
    let value_json = serde_json::to_string_pretty(value).unwrap();
    file.write_all(value_json.as_bytes()).unwrap();
}

fn main() {
    let installer_exe_path = std::env::current_exe().unwrap();
    let installer_path = installer_exe_path.parent().unwrap();
    let installer_dinput_path = installer_path.join("NOTES ELITE").join("dinput8.dll");
    let installer_server_path = installer_path
        .join("twipo-synchro")
        .join("twipo-synchro.exe");
    for path in [&installer_dinput_path, &installer_server_path].iter() {
        if !path.is_file() {
            println!(
                "Unable to find {}, make sure you extracted the installation archive.",
                path.to_str().unwrap()
            );
            pause();
            return;
        }
    }

    print!(
        r#"twipo-synchro installer
=======================

Licenses :
==========
twipo-synchro/LICENSE:               twipo-synchro license
twipo-synchro/charset.LICENSE.md:    twipo-synchro RN;E charset license
twipo-synchro/thirdparty.LICENSE.md: twipo-synchro thirdparty licenses
NOTES ELITE/LICENSE:                 LanguageBarrier license
NOTES ELITE/THIRDPARTY.LB.txt:       LanguageBarrier thirdparty licenses

Did you read and accept the terms and conditions [y/N] : "#
    );
    flush();

    let license_ans = read();
    if license_ans.trim().to_lowercase() != "y" {
        println!("You need to accept the licenses to continue");
        pause();
        return;
    }

    let default_folder = "TODO".to_string();
    print!(
        r#"
Game Directory :
================

Please provide the location of your ROBOTICS;NOTES ELITE installation.
This is folder that Steam's point to when you select the "Browse local file"
option in the properties window in your library.

"#
    );

    if cfg!(windows) {
        println!("You can drag-and-drop the folder in the command line window.");
    }
    println!("Press enter without providing any folder if the detected one is correct.");

    print!("\n[{}] : ", default_folder);
    flush();

    let folder_ans = read();
    let folder_str = match folder_ans.trim() {
        "" => default_folder,
        f => f.trim().to_string(),
    };

    let game_path = Path::new(&folder_str);
    if !game_path.is_absolute() {
        println!("Please enter an absolute path");
        pause();
        return;
    }

    let lb_path = game_path.join("languagebarrier");
    let patchdef_path = lb_path.join("patchdef.json");
    let gamedef_path = lb_path.join("gamedef.json");
    let defaultconfig_path = lb_path.join("defaultconfig.json");

    let dll_path = game_path.join("NOTES ELITE");
    let dinput_path = dll_path.join("dinput8.dll");
    let dinput_backup_path = dll_path.join("dinput8_coz.dll");
    for path in [
        &patchdef_path,
        &gamedef_path,
        &defaultconfig_path,
        &dinput_path,
    ]
    .iter()
    {
        if !path.is_file() {
            println!("Unable to find {}, make sure you installed the Committee of Zero ROBOTICS;NOTES ELITE Steam Patch.", path.to_str().unwrap());
            pause();
            return;
        }
    }

    {
        let patchdef = read_json(&patchdef_path);
        let patch_version = patchdef["patchVersion"].as_str().unwrap();
        if patch_version != EXPECTED_VERSION {
            println!(
                "This mod was only tested with CoZ patch version {} but version {} is installed",
                EXPECTED_VERSION, patch_version
            );
            print!("Continue ? [y/N] : ");
            flush();
            let continue_ans = read();
            if continue_ans.trim().to_lowercase() != "y" {
                return;
            }
        }
    }

    println!("STEP 1 : Adding twipo-synchro signatures...");
    {
        let twipo_sig_json =
            serde_json::from_str::<serde_json::Value>(include_str!("../res/signatures.json"))
                .unwrap();
        let twipo_sig_object = twipo_sig_json.as_object().unwrap();
        let mut gamedef = read_json(&gamedef_path);
        let signatures = gamedef["signatures"]["game"].as_object_mut().unwrap();
        for (key, value) in twipo_sig_object.iter() {
            if signatures.contains_key(key) {
                println!("WARN : Signature {} already found, replacing", key);
            }
            signatures.insert(key.clone(), value.clone());
        }
        save_json(&gamedef_path, &gamedef);
    }

    println!("STEP 2 : Adding twipo-synchro config...");
    {
        let mut config_json = read_json(&defaultconfig_path);
        let config_object = config_json.as_object_mut().unwrap();
        if config_object.contains_key(CONFIG_KEY) {
            println!("WARN : Config key {} already found, skipping", CONFIG_KEY);
        } else {
            config_object.insert(
                CONFIG_KEY.to_string(),
                serde_json::Value::String(CONFIG_VALUE.to_string()),
            );
        }
        save_json(&defaultconfig_path, &config_json);
    }

    println!("STEP 3 : Copying twipo-synchro server...");
    {
        let twipo_synchro_path = game_path.join("twipo-synchro");
        if !twipo_synchro_path.is_dir() {
            fs::create_dir(&twipo_synchro_path).unwrap();
        }

        let server_path = twipo_synchro_path.join("twipo-synchro.exe");
        if server_path.is_file() {
            println!("WARN : twipo-synchro server already present, overwriting");
        }
        fs::copy(installer_server_path, server_path).unwrap();

        let version_path = twipo_synchro_path.join("version.txt");
        let mut version_file = fs::File::create(version_path).unwrap();
        version_file.write_all(VERSION_STRING).unwrap();
    }

    println!(
        "STEP 4 : Backing up old LanguageBarrier and copying twipo-synchro LanguageBarrier..."
    );
    {
        if dinput_backup_path.is_file() {
            println!("WARN : LanguageBarrier backup already present, skiping");
        } else {
            fs::copy(&dinput_path, dinput_backup_path).unwrap();
        }
        fs::copy(installer_dinput_path, dinput_path).unwrap();
    }

    println!("The instalation of twipo-synchro was successful !");
    pause();
}
