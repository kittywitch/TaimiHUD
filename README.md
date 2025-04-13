# TaimiHUD

Features:
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
        - [ ] Main Window
            - [x] Tab bar
                - [x] Timers tab
        - [ ] Timers Window
            - [ ] Add icon to progress bars
            - [ ] Render text separately from the progress bar widget so that it no longer moves with the progress
            - [ ] Make colours for progress bar text and background more sane (and still imported from the timer data)
        - [ ] Timers tab - control timers
            - [ ] Reset button for timers
            - [ ] Allow enabling and disabling timers
            - [ ] Timer icons
            - [ ] Timer descriptions
            - [ ] Separate enabling and disabling of timers into categories
        - [ ] Make mutually exclusive timer enablement for CMs, or provide the user with a prompt on map for the choice
    - [ ] Find way to include data within the DLL for icons
        - [ ] investigate https://crates.io/crates/include_dir
    - [ ] Fork Hero's Timers
        - [ ] Add markers alongside cardinal directions on Sabetha
- [ ] Commander's Markers
- [ ] Pathing

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
