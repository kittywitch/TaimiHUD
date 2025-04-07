# TaimiHUD

Features:
- [ ] Timers
    - [x] Get DLL loaded with proper name
    - [x] Empty UI window toggled with button
    - [x] Timers file loaded using Serde
    - [ ] Timer phases
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
            - [ ] Display progress bar
        - [ ] Directions
        - [ ] Markers
            - [ ] 3D rendering
        - [ ] Sounds
    - [x] Get a timer running
        - [x] Timer state machine
    - [ ] Load every timer file
        - [ ] Fix issues
            - [ ] "R_W3B3 - Xera - All.bhtimer" - key must be a string at line 105 column 9
            - [ ] "R_W5B4T3 - Dhuum - Golem Spawns.bhtimer" - expected value at line 355 column 2
            - [ ] "S_B10 - Harvest Temple CM.bhtimer" - expected value at line 1560 column 7
            - [ ] "S_B10 - Simulation - 0 - Main.bhtimer" - expected value at line 132 column 7
            - [ ] "S_B10 - Simulation - 0 - Main.bhtimer" - expected value at line 132 column 7
            - [ ] "S_B10 - Simulation - 0 - Main.bhtimer" - expected value at line 132 column 7
            - [ ] "S_B10 - Simulation - Void Pools 1.bhtimer" - expected value at line 64 column 7
            - [ ] "S_B10 - Simulation - Void Pools 2.bhtimer" - expected value at line 183 column 7
            - [ ] "S_B13 - Old Lion's Court CM.bhtimer" - expected value at line 127 column 7

    - [ ] UI work
        - [ ] Allow enabling and disabling timers with categories
- [ ] Markers
- [ ] Pathing

## References

### Nexus

* https://docs.rs/nexus-rs/latest/nexus_rs/#
* https://docs.rs/arcdps-imgui/0.8.0/arcdps_imgui/

### Timers

* Timers Data: https://github.com/QuitarHero/Hero-Timers
* https://github.com/Dev-Zhao/Timers_BlishHUD

### Markers
* Marker Data: https://github.com/manlaan/BlishHud-CommanderMarkers/tree/bhud-static/Manlaan.CommanderMarkers
