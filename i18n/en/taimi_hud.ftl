## Common

height = Height
font = Font
okay = OK
save = Save
error = Error
unknown = Unknown
update = Update
description = Description
location = Location: { $path }
data = Data
object = Object
files = Files
# as in 3D
model = Model
close = Close
name = Name
icon = Icon
path = Path
title = Title
controls = Controls
id = ID
id-arg = { id }: { $id }
map-id = Map { id }
map-id-arg = { map-id }: { $id }
author = Author
position = position
position_cap = Position
not-applicable = N/A
rt-api-required-base = RTAPI is required for
rt-api-required = { rt-api-required-base } { $reason }.
no-description = No description provided.
no-thing-arg = No { $thing } provided.
expand-all = Expand All
collapse-all = Collapse All
enable = Enable
disable = Disable
enabled = { enable }d
disabled = { disable }d
author-arg = { author }: { $author }
reset = Reset
timer = Timer
timers = { timer }s
experimental-notice = Hi! This feature is (mostly) experimental. Some things may be using and it might require more thought and effort to use than the less experimental features. My apologies to any problems you have; feel free to reach out on Discord. - Kat
## Addon

primary-window-toggle = Taimi Window Toggle
timer-window-toggle = Timer Window Toggle
primary-window-toggle-text = Show/hide taimi primary window
timer-key-trigger = Timer Key Trigger { $id }

## Config

config-tab = Config
stock-imgui-progress-bar = Stock Imgui Progress Bar
shadow = Shadow
centre-text-after-icon = Centre text after icon

## Windows

primary-window = Taimi
timer-window = Timers Window
marker-window = Markers Window

## Modals

addon-uninstall-modal-title = Uninstall { $source }?
addon-uninstall-modal-button = Uninstall
addon-uninstall-modal-description = Please be careful! This will delete the folder and anything it contains.

## Openable

open-button = Open { $kind }
open-error = { error } opening { $kind }: { $path }

## Data sources

data-sources-tab = Data Sources
checking-for-updates = Checking for updates!
check-for-updates = Check for updates
check-for-updates-tooltip = Check for updates to any data sources. We don't do this automatically to respect your choice on whether or not to make network requests.
checked-for-updates-last = Last checked for updates at: { $time }
reload-data-sources = Reload data sources
reload-data-sources-tooltip = Reload items from currently installed data sources. Useful if you have changed the files within them!

remote = Remote
update-status = Update Status
actions = Actions
version-installed = Installed version: { $version }
version-not-installed = Not installed
update-unknown = Update status unknown; check for updates?
update-not-required = Update not required; up to date!
update-available = New version available: { $version }!
update-error = { error } updating: { $error }!
attempt-update = Attempt to update anyway?
settings-unloaded = Settings have not yet loaded!

## Info tab

info-tab = Info
keybind-triggers = If you need keybind-based timer triggers, please bind the appropriate keys in the Nexus settings.
active-timer-phases = Active timer phases
timer = Timer
phase = Phase
# As in, like, "game engine" or "rendering engine" :o
engine = Engine
ecs-data = ECS { data }
object-data = { object } { data }
obkect-kind = { object } Kind
model-files = { model } Files
vertices = Vertices
textures = Textures: { $count }

## Markers tab

marker-tab = { markers }
marker = Marker
markers = { marker }s
markers-place = Place { markers }
marker-set = { marker } Set
marker-set-create = Create { marker-set }
marker-set-delete = Delete { marker-set }
scaling-factor = scaling factor
current-scaling-factor = Current { scaling-factor }: ({ $x }, { $y })
current-scaling-factor-multiple = Current { scaling-factor } as multiple of ft per continent unit: ({ $x }, { $y })
scaling-factor-reset = { reset } detected { scaling-factor }
no-file-associated = Couldn't find associated file
markers-arg = { markers }: { $count }
marker-type = { marker } Type
local-header = Local (XYZ)
map-header = Map (XY)
screen-header = Screen (XY)
marker-not-on-screen = Not on screen
select-a-marker = Please select a marker to configure!

## Markers window

set-map-id = Set Map ID to current map
current-squad-markers = current squad markers
take-squad-markers = Take from { current-squad-markers }
cannot-take-squad-markers = Cannot take from { current-squad-markers }; not in a squad.
rt-api-required-squad-markers = { rt-api-required-base } taking squad marker locations.
no-position = No position provided.
position-get = Get current { position }
set-manually = Set manually

## Timer tab

timer-tab = { timers }
source-arg = Source: { $source }
source-adhoc = Source: Ad-hoc
select-a-timer = Please select a timer to configure!

## Timer window

timer-window = { timers }
no-phases-active = No phases currently active, no timers running.
reset-timers = { reset } { timers }
