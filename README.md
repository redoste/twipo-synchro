# twipo-synchro

**twipo-synchro** is a [**ROBOTICS;NOTES ELITE Steam Patch**](https://sonome.dareno.me/projects/rne-steam.html) mod for interacting with *Twipo* (in-game Twitter clone) from an in-real-life mobile device.

## Installation and Usage

You can download the latest version from the [releases](https://github.com/redoste/twipo-synchro/releases), make sure to extract the archive before proceeding. This `README` is also present in the archive.

### Install
You first need to install the [ROBOTICS;NOTES ELITE Steam Patch](https://sonome.dareno.me/projects/rne-steam.html), the current version of twipo-synchro was tested with the version `1.1.0` of the patch.

On Windows, simply browse to the extraced archive, execute `installer-win.exe` and follow the on-screen instructions.

On Linux, you need to mark the file as executable before. Open a terminal emulator in the folder you extracted and run the following commands :
```console
chmod +x ./installer-linux
./installer-linux
```
The current version of the mod was tested with Proton Experimental build [`10106891`](https://steamdb.info/patchnotes/10106891/).

### Usage
After installation, you can start the game as usual from the patch launcher. A new command prompt will be opened after the game starts, it will show the web server logs. It is now possible to connect using another device on the same network by browsing to the web page `http://[local IP address of the computer running the game]:8080/`.

On the first start on Windows you may need to accept a firewall exception, this is required to allow the web server to listen on the network and let your mobile device connect to it.

On Linux make sure you have no `iptables` or `nftables` rules blocking the web server. Some distributions might include built-in firewalls such as `ufw`, make sure to check this if you are experiencing connection issues.

### Configuration
You can change the port the web server is listening on by editing the JSON file located in the game folder at `languagebarrier/defaultconfig.json`. Edit the string with the key `twipoSynchroListenAddress`, you can specify a different port than `8080` or select a precise address to listen on if your computer has multiple network interfaces.

### Uninstall
To uninstall the mod, simply delete the `twipo-synchro` folder in the game directory and restore the original version of LanguageBarrier by renaming `dinput8_coz.dll` back to `dinput8.dll` in the `NOTES ELITE` folder.

If you want a completly clean install of the patch, you can remove the [signatures created by the mod](./res/signatures.json) in the `languagebarrier/gamedef.json` file and the configuration option added in `languagebarrier/defaultconfig.json`.

## Support
Please **DO NOT** contact the Committee of Zero if you encounter a problem with your game after installing the mod. This is more likely my fault than theirs. You can open a [GitHub issue](https://github.com/redoste/twipo-synchro/issues/new) and describe your problem.

## Building

The build procedure can be deduced from the [GitHub Actions Workflow file](./.github/workflows/build.yml), but basically the project consists of a simple Rust project using Cargo that implements an HTTP and WebSocket server and a [LanguageBarrier](https://github.com/CommitteeOfZero/LanguageBarrier) fork that intercepts Tweeps and sends them to the web server.

Changes applied to LanguageBarrier can be analysed from [the GitHub compare view](https://github.com/CommitteeOfZero/LanguageBarrier/compare/rn-changes...redoste:LanguageBarrier:twipo-synchro).
