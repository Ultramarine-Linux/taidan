# shown on places like "keyboard layout variant: Default"
default = Default
# btn (previous page)
prev = Previous
# btn (next page)
next = Next

# shown on the progress bar
steps =
    .dnfdownloadupdate = Downloading System Update…
    .dnfinstallupdate = Install System Update…
    .script = Running Distribution scriptlet…
    .dnfdownloadapps = Downloading User Programs…
    .dnfinstallapps = Installing User Programs…
    .driverscodecs = Installing additional drivers…

# .display:
# This is intentional and you are required to fill in this field.
# $lang_name is the language name you probably have just translated (see
# the nearby strings above this). $native_lang_name is the language name
# in its corresponding language. For example, when displaying in English:
#
# Chinese (中文)
# Japanese (日本語)
# Korean (한국어)
# ...
#
# In other languages, you might need to change the order around and maybe
# use different brackets, etc.
imelangs =
    .ch = Chinese
    .ja = Japanese
    .ko = Korean
    .vi = Vietnamese
    .in = Indic
    .th = Thai
    .display = {$lang_name} ({$native_lang_name})

## page: Welcome

# page title
# .ready: ≈ "Next" btn; on click, goes to the next page
# .skipcfg: also a "Next" btn, but skips some pages
page-welcome = Welcome to {$distro}
    .ready = Let's get your system ready.
    .go = Let's Go
    .skipcfg = Skip Configuration

## page: Keyboard Layout
# page title
# .search*: search field placeholder
page-keyboard = Keyboard Layout
    .search-layout = Search keyboard layout…
    .search-variant = Search keyboard variant…

## page: Who are You?
# page title
# remember, if you can't translate things literally, think of what makes sense here as a page heading
# .fullname: text field placeholder
# .username: text field placeholder
# .error: multiline, explanation pops out if invalid username
page-whoareyou = Who are You?
    .fullname = Full Name
    .username = Username
    .error = Username must start with lowercase letters and contain only alphanumericals, underscore (<tt>_</tt>) or dash (<tt>-</tt>)

## page: Create a Password
# page title
# the `.pw` attribute is a text field placeholder
# the `.rp` attribute is yet another text field placeholder
page-password = Create a Password
    .pw = Password
    .rp = Repeat Password

## page: Internet
# page title
# .skip: btn
# .open: btn
page-internet = Let's Get You Online
    .desc = Connect to the Internet to get the latest and greatest.
    .skip = I don't have Internet
    .warn = Codecs, drivers and other user programs will not be installed.
    .open = Open Wi-Fi connection applet

# do NOT translate (for now)
page-analytics = Analytics and Crash Reporting

# do NOT translate (for now)
page-crashreport = Crash Reporting
    .desc = Allow {$org} to collect crash data to find bugs and assist you.
    .lbl1 = All data collected is anonymous and end-to-end-encrypted.
    .lbl2 = You will be given a crash ID to help support find what went wrong.
# do NOT translate (for now)
# btn-switch
switch-crashreport = Send Crash Data
    .desc = Press next to keep off

# do NOT translate
page-location = Location Services
    .desc = Allow apps to request your approximate location with Mozilla Location Services.

## page: Codecs and Drivers
page-codecs = Codecs and Drivers
    .desc1 = Install proprietary media codecs and drivers for your device.
    .desc2 = Consult the {$wiki} if you don't have an Internet connection.
    # we need to make this another attribute because we need to tag this as a link
    .wiki = wiki
switch-codecs = Install Codecs and Drivers
    .desc = Press next to skip installation

## page: Input Method
# page title
# .search*: search field placeholder
page-inputmethod = Input Method
    .desc1 = You may <b>optionally</b> choose to add an {$ime}. This allows you to type in other specific languages. This change will take effect after you login inot your user account.
    .desc2 = More Chinese input method options may be available with the {$rime} engine, but this requires advanced configuration and is therefore not recommended to beginners.
    .desc3 = You may fine out more information on {$wiki}.
    .ime = input method editor (IME)
    .rime = Rime
    .wiki = the wiki
    .search-lang = Search language…
    .search-ims = Search IMs/IMEs…

## page: Nightlight
# page title
page-nightlight = Night Light
    .lbl1 = Tint the display with a warm tone at night to reduce eyestrain.
    .lbl2 = Night Light is not proven to help with difficulty falling sleep.

switch-nightlight = Night Light

## page: Choose Your Theme
# page title
# .light: Light Theme
# .dark: Dark Theme
page-theme = Choose Your Theme
    .desc = Make this system your own with a splash of colour.
            You can change this option later in settings.
    .note = Some apps may not respect this preference.
    .light = Light
    .dark = Dark

## page: Browser Selection
page-browser = Browser Selection

## page: Categories
## this page shows different categories in the app catalogue (e.g. Productivity, Gaming, Media Prod., etc.)
# .confirm: btn
# .done: btn; shown on new windows for closing
page-categories = What Do You Use This Device For?
    .confirm = Confirm and Setup System
    .done = Done

# category names
# you should use a selector; e.g.
# { $cat ->
#   [Productivity] -> projdsalkaj;cjzkpqi
#   [...]
# }
# the list of categories is available at 
# https://github.com/Ultramarine-Linux/taidan/tree/master/catalogue
# open each yml file, check the first line for the name
categories = {$cat}

## page: Installing
page-installing = Installing Your Apps
    .desc = This won't take long!
    .loading = Loading…

## page: Finish
# .done: btn; closes the entire application
page-finish = Your System is Ready
    .desc = We hope you enjoy your fresh system!
    .done = Done

## page: Error
# .done: btn; closes the entire application
# .retry: btn
page-error = Error
    .desc = We are sorry, but there is an unexpected problem.
    .done = Done
    .retry = Retry
