# shown on places like "keyboard layout variant: Default"
default = Default

## page: Welcome

# page title
welcome = Welcome to {$distro}
    .ready = Let's get your system ready.
    # ≈ "Next" btn; on click, goes to the next page
    .go = Let's Go
    # also a "Next" btn, but skips some pages
    .skipcfg = Skip Configuration

## page: Keyboard Layout
# page title
page-keyboard = Keyboard Layout
    # search field placeholder
    .search-layout = Search keyboard layout…
    # search field placeholder
    .search-variant = Search keyboard variant…

## page: Who are You?
# page title
# remember, if you can't translate things literally, think of what makes sense here as a page heading
page-whoareyou = Who are You?
    # text field placeholder
    .fullname = Full Name
    # text field placeholder
    .username = Username
    # multiline, explanation pops out if invalid username
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
page-internet = Let's Get You Online
    .desc = Connect to the Internet to get the latest and greatest.
    # btn
    .skip = I don't have Internet
    .warn = Codecs, drivers and other user programs will not be installed.
    # btn
    .open = Open Wi-Fi connection applet

# do NOT translate (for now)
page-analytics = Analytics and Crash Reporting

# do NOT translate (for now)
page-crashreport = Crash Reporting
    .desc = Allow {$org} to collect crash data to find bugs and assist you.
    .lbl1 = All data collected is anonymous and end-to-end-encrypted.
    .lbl2 = You will be given a crash ID to help support find what went wrong.
