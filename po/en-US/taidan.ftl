# shown on places like "keyboard layout variant: Default"
default = Default
# btn (previous page)
prev = Previous
# btn (next page)
next = Next
# btn (skip anything)
skip = Skip
steps-dnfdownloadupdate = Downloading System Update…
steps-dnfinstallupdate = Installing System Update…
steps-script = Running Scriptlet…
steps-dnfdownloadapps = Downloading Apps…
steps-dnfinstallapps = Installing Apps…
steps-driverscodecs = Installing Drivers and Codecs…
imelangs-ch = Chinese
imelangs-ja = Japanese
imelangs-ko = Korean
imelangs-vi = Vietnamese
imelangs-in = Indic
imelangs-th = Thai
# .display:
# This is intentional and you are required to fill in this field.
# $lang_name is the language name you probably have just translated (see
# the nearby strings above this). $native_lang_name is the language name
# in its corresponding language. For example, when displaying in English:
#
# Chinese (中文)
# Japanese (日本語)
# Korean (한국어)
# …
#
# In other languages, you might need to change the order around and maybe
# use different brackets, etc.
imelangs-display = { $lang_name } ({ $native_lang_name })

## [Language]

page-language = Language
page-language-search-lang = Search Language/Locale…

## [Welcome]

page-welcome = Welcome to { $distro }
# page title
page-welcome-ready = Let's get your device ready.
page-welcome-go = Let's Go
# .skipcfg: also a "Next" btn, but skips some pages
page-welcome-skipcfg = Skip Configuration

## [Keyboard Layout]

# page title
page-keyboard = Keyboard Layout
# .search*: search field placeholder
page-keyboard-search-layout = Search keyboard layouts…
page-keyboard-search-variant = Search keyboard variants…

## [Username]

# page title
# remember, if you can't translate things literally, think of what makes sense here as a page heading
page-whoareyou = Who are You?
# .fullname: text field placeholder
page-whoareyou-fullname = Full Name
# .username: text field placeholder
page-whoareyou-username = Username
# .error: multiline, explanation pops out if invalid username
page-whoareyou-error = Username must start with lowercase letters and must only contain lowercase letters, numbers, underscores (<tt>_</tt>), or hyphens (<tt>-</tt>).

## [Password]

# page title
page-password = Create a Password
# text field placeholder for password
page-password-pw = Password
# yet another text field placeholder for repeating the password.
# depending on the language you can just put the word equivalent to "repeat" here
page-password-rp = Repeat Password

## [Internet]

# page title
page-internet = Let's Get Online
page-internet-desc = Connect to the Internet to get the latest and greatest.
page-internet-skip = I don't have Internet
page-internet-warn = Codecs, drivers and some apps won't be installed without internet.
page-internet-open = Connect to a WiFi Network
page-internet-ok = You're connected! Press next to continue.
page-internet-portal = Login with Web Browser

## [Analytics and Crash Reporting]
## this page is currently not in use, we recommend skipping

page-analytics = Analytics and Crash Reporting
page-crashreport = Crash Reporting
page-crashreport-desc = Allow { $org } to collect crash data to find bugs and assist you.
page-crashreport-lbl1 = All data collected is anonymous and end-to-end-encrypted.
page-crashreport-lbl2 = You will be given a crash ID to help support find what went wrong.
switch-crashreport = Send Crash Data
switch-crashreport-desc = Press next to keep off

## [Tweaks]
page-tweaks = System Tweaks

## [Codecs and Drivers]

page-codecs = Codecs and Drivers
page-codecs-desc1 = Install common media codecs and drivers for your device.
page-codecs-desc2 =
    Consult the { $wiki } if you don't have an Internet connection.
page-codecs-wiki = wiki
switch-codecs = Install Codecs and Drivers
switch-codecs-desc = Turning this on installs proprietary codecs and drivers

## [Input Method]

# page title
page-inputmethod = Additional Input Methods
# languages that do require IMEs should change the second line to:
# Press "Add Input Method" to continue.
page-inputmethod-desc =
    Languages with non-Latin characters may require an extra <b>input method editor (IME)</b>.
    You can skip this page if you do not need to type in one of these languages.
    More information is available on { $wiki }.
page-inputmethod-add = Add Input Method
page-inputmethod-wiki = the wiki
page-inputmethod-search-lang = Search language…
page-inputmethod-search-ims = Search IMs/IMEs…

## [Night Light]

# page title
page-nightlight = Night Light
page-nightlight-desc =
    Tint the display with a warm tone at night to reduce eyestrain.
    Night Light is not proven to help with difficulty falling sleep.
switch-nightlight = Night Light

## [Choose Your Theme]

# page title
page-theme = Choose Your Theme
page-theme-desc =
    Make this system your own with a splash of colour.
    You can change this option later in settings.
page-theme-note = Some apps may not respect this preference.
page-theme-light = Light
page-theme-dark = Dark

## [Browser Selection]

page-browser = Browser Selection

## page: Categories
## this page shows different categories in the app catalogue (e.g. Productivity, Gaming, Media Prod., etc.)

# page title
page-categories = What Do You Use This Device For?
page-categories-confirm = Confirm and Setup System
page-categories-done = Done
# category names
# you should use a selector; e.g.
# { $cat ->
#   [Productivity] -> projdsalkaj;cjzkpqi
#   […]
# }
# the list of categories is available at
# https://github.com/Ultramarine-Linux/taidan/tree/master/catalogue
# open each yml file, check the first line for the name
categories = { $cat }

## [Installing]

page-installing = Installing Your Apps
page-installing-desc = This won't take long!
page-installing-loading = Loading…
page-installing-flatpak = Installing {$n} Flatpaks…

## [Finish]

page-finish = Ready to Do Your Thing?
page-finish-desc = We hope you enjoy Ultramarine!
page-finish-done = Done

## [Error]

page-error = Error
page-error-desc = We hit a snag.
page-error-done = Exit
page-error-retry = Retry

## [Tweaks in Ultramarine]

flatpakening-name = Use RPMs instead of Flatpaks
flatpakening-desc = Flatpaks provide better security by default, RPMs will save some space.
