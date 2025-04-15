# TaimiHUD

A cross-platform Timers addon, leveraging [Raidcore Nexus](https://raidcore.gg/Nexus).
Long-term, intends to provide markers and directions too.

Project management is handled here: https://github.com/users/kittywitch/projects/1

[Video](https://files.catbox.moe/xdno9s.mp4)

## Screenshots

![Main interface](https://github.com/user-attachments/assets/82044140-5a81-4bb1-8d2a-be468de2450e)
![Data source updating](https://github.com/user-attachments/assets/12135f4f-5ceb-44d0-a136-15ebaa07511a)
![Timers during combat](https://github.com/user-attachments/assets/bb930b54-717c-4fa7-b65e-2ec77a7c2393)

## Features

* Can load .bhtimer type timers
    * Supports location and keybind triggers, where keybind triggers are partially working (see #9)
      * Handles combat state directly
    * Phases are functional
* Supports persistent enabling and disabling of timers
* Can download Hero-Timers automatically for you, has a check for update functionality

### Does not have yet:

* Markers
* Directions
* Sounds

## References

### Nexus

* https://docs.rs/nexus-rs/latest/nexus_rs/#
* https://docs.rs/arcdps-imgui/0.8.0/arcdps_imgui/

### Imgui

* https://pthom.github.io/imgui_manual_online/manual/imgui_manual.html
* https://github.com/ocornut/imgui/blob/master/imgui_widgets.cpp

### Timers

* Timers Data: https://github.com/QuitarHero/Hero-Timers
* https://github.com/Dev-Zhao/Timers_BlishHUD

### Markers

* Marker Data: https://github.com/manlaan/BlishHud-CommanderMarkers/tree/bhud-static/Manlaan.CommanderMarkers
