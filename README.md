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
            - [ ] "R_W3B3 - Xera - All.bhtimer" - key must be a string at line 105 column 9 - https://github.com/QuitarHero/Hero-Timers/blob/main/timers/R_W3B3%20-%20Xera%20-%20All.bhtimer#L105
            - [ ] "R_W5B4T3 - Dhuum - Golem Spawns.bhtimer" - expected value at line 355 column 2 - https://github.com/QuitarHero/Hero-Timers/blob/main/timers/R_W5B4T3%20-%20Dhuum%20-%20Golem%20Spawns.bhtimer#L355
            - [ ] "S_B10 - Harvest Temple CM.bhtimer" - expected value at line 1560 column 7 - https://github.com/QuitarHero/Hero-Timers/blob/main/timers/S_B10%20-%20Harvest%20Temple%20CM.bhtimer#L1560 - this one has duplicate "markers" keys!
            - [ ] "S_B10 - Simulation - 0 - Main.bhtimer" - expected value at line 132 column 7 - https://github.com/QuitarHero/Hero-Timers/blob/main/timers/S_B10%20-%20Simulation%20-%200.%20Main.bhtimer#L132
            - [ ] "S_B10 - Simulation - Void Pools 1.bhtimer" - expected value at line 64 column 7 - https://github.com/QuitarHero/Hero-Timers/blob/main/timers/S_B10%20-%20Simulation%20-%20Void%20Pools%201.bhtimer#L64
            - [ ] "S_B10 - Simulation - Void Pools 2.bhtimer" - expected value at line 183 column 7 - https://github.com/QuitarHero/Hero-Timers/blob/main/timers/S_B10%20-%20Simulation%20-%20Void%20Pools%202.bhtimer#L183
            - [ ] "S_B13 - Old Lion's Court CM.bhtimer" - expected value at line 127 column 7 - https://github.com/QuitarHero/Hero-Timers/blob/main/timers/S_B11%20-%20Old%20Lion's%20Court%20CM.bhtimer#L127

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
