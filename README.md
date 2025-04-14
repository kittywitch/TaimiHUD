# TaimiHUD

A cross-platform Timers addon, leveraging [Raidcore Nexus](https://raidcore.gg/Nexus).
Long-term, intends to provide Pathing and Markers too.

[Video](https://files.catbox.moe/xdno9s.mp4)

## Screenshots

![Main interface](https://github.com/user-attachments/assets/82044140-5a81-4bb1-8d2a-be468de2450e)
![Data source updating](https://github.com/user-attachments/assets/12135f4f-5ceb-44d0-a136-15ebaa07511a)
![Timers during combat](https://github.com/user-attachments/assets/bb930b54-717c-4fa7-b65e-2ec77a7c2393)

## Features (or, my To-dos) ;3

- [ ] Data
    - [ ] Fork Hero's Timers
        - [ ] Add markers alongside cardinal directions on Sabetha
    - [x] Provide a method for downloading and extracting data into the addon_dir. We do not want to redistribute the files themselves.
        - [x] https://docs.rs/tar/latest/tar/ - combine with download from GitHub
            - [x] Either use a crate to get GitHub information and check last downloaded release, or pre-obtain md5...?
- [ ] Timers
    - [x] Get DLL loaded with proper name
    - [x] Empty UI window toggled with button
    - [x] Timers file loaded using Serde
    - [ ] Timer phases
        - [ ] Add variant system, e.g. roles (1-3, 2-4, or challenge mode)
        - [x] Timer triggers
            - [x] Location
                - [x] Sphere
                - [x] Cuboid
            - [ ] Keybind
            - [x] Conditions
                - [x] Entry
                - [x] Departure
                - [x] Combat
                - [x] Left Combat
        - [x] Timer actions
            - [x] Display progress bar
        - [ ] Markers
            - [ ] Check implemented type
            - [ ] 3D rendering
        - [ ] Directions
            - [ ] Implement type
        - [ ] Sounds
            - [ ] Implement type
            - [ ] Investigate the use of a TTS library for text
    - [x] Get a timer running
        - [x] Timer state machine
    - [x] Load every timer file
    - [x] Move phasestate into timermachine
    - [ ] Persistent configuration/enablement state
    - [ ] UI work
        - [x] Main Window
            - [x] Tab bar
                - [x] Timers tab
        - [ ] Timers Window
            - [ ] Add icon to progress bars
            - [ ] Render text separately from the progress bar widget so that it no longer moves with the progress
            - [ ] Make colours for progress bar text and background more sane (and still imported from the timer data)
        - [x] Timers tab - control timers
            - [x] Reset button for timers
            - [x] Allow enabling and disabling timers
            - [ ] Timer icons
            - [x] Timer descriptions
            - [x] Separate timers into categories
        - [ ] Make mutually exclusive timer enablement for CMs, or provide the user with a prompt on map for the choice
    - [ ] Find way to include data within the DLL for icons
        - [ ] investigate https://crates.io/crates/include_dir
- [ ] Commander's Markers

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
