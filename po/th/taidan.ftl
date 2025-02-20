# shown on places like "keyboard layout variant: Default"
default =
# btn (previous page)
prev = ก่อนหน้า
# btn (next page)
next = ถัดไป

# shown on the progress bar
steps =
    .dnfdownloadupdate = กำลังดาวน์โหลดอัปเดตระบบ…
    .dnfinstallupdate = กำลังติดตั้งอัปเดตระบบ…
    .script = 
    .dnfdownloadapps = กำลังดาวน์โหลดโปรแกรม…
    .dnfinstallapps = กำลังติดตั้งโปรแกรม…
    .driverscodecs = กำลังติดตั้งอัปเดตระบบ…

imelangs =
    .ch =
    .ja =
    .ko =
    .vi =
    .in =
    .th =

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
    .display = {$lang_name} ({$native_lang_name})

## page: Welcome

# page title
page-welcome = ยินดีต้อนรับสู่ {$distro}
    .ready = เรามาเตรียมพร้อมระบบกันดีกว่า
    # ≈ "Next" btn; on click, goes to the next page
    .go = ไปกันเถอะ
    # also a "Next" btn, but skips some pages
    .skipcfg = ข้ามการตั้งค่า

## page: Keyboard Layout
# page title
page-keyboard =
    # search field placeholder
    .search-layout =
    # search field placeholder
    .search-variant =

## page: Who are You?
# page title
# remember, if you can't translate things literally, think of what makes sense here as a page heading
page-whoareyou = คุณเป็นใคร?
    # text field placeholder
    .fullname = ชื่อจริง
    # text field placeholder
    .username = ชื่อผู้ใช้
    # multiline, explanation pops out if invalid username
    .error = ชื่อผู้ใช้
        - ต้องเริ่มด้วยอักษรละตินตัวพิมพ์เล็ก
        - ใส่ได้แค่อักษรละติน, เลขฮินดูอารบิก, อันเดอร์สกอร์ (<tt>_</tt>) หรือยัติภาค(<tt>-</tt>) เท่านั้น

## page: Create a Password
# page title
# the `.pw` attribute is a text field placeholder
# the `.rp` attribute is yet another text field placeholder
page-password = สร้างรหัสผ่าน
    .pw = รหัสผ่าน
    .rp = ใส่รหัสผ่านอีกครั้ง

## page: Internet
# page title
page-internet = มาต่ออินเตอร์เน็ตกันเถอะ
    .desc =
    # btn
    .skip = ฉันไม่มีอินเตอร์เน็ต
    .warn =
    # btn
    .open =

# do NOT translate (for now)
page-analytics =

# do NOT translate (for now)
page-crashreport =
    .desc =
    .lbl1 =
    .lbl2 =
# do NOT translate (for now)
# btn-switch
switch-crashreport =
    .desc =

# do NOT translate
page-location =
    .desc =

## page: Codecs and Drivers
page-codecs =
    .desc1 =
    .desc2 =
    # we need to make this another attribute because we need to tag this as a link
    .wiki =
switch-codecs =
    .desc =

## page: Input Method
# page title
page-inputmethod =
    .desc1 =
    .desc2 =
    .desc3 =
    .ime =
    .rime =
    .wiki =
    # search field placeholder
    .search-lang =
    # search field placeholder
    .search-ims =

## page: Nightlight
# page title
page-nightlight =
    .lbl1 = ปรับสีจอภาพด้วยโทนสีอุ่นในเวลากลางคืนเพื่อลดอาการปวดตา
    .lbl2 =

switch-nightlight = Night Light

## page: Choose Your Theme
# page title
page-theme = Choose Your Theme
    .desc = ตั้งธีมให้ตรงใจคุณ
            คุณยังกลับมาที่การตั้งค่าเพื่อเปลี่ยนตรงนี้ทีหลังได้
    .note = บางแอพพลิเคชันอาจไม่ปรับสีให้ตามธีม
    # Light Theme
    .light = สว่าง
    # Dark Theme
    .dark = มืด

## page: Browser Selection
page-browser = Browser Selection

## page: Categories
## this page shows different categories in the app catalogue (e.g. Productivity, Gaming, Media Prod., etc.)
page-categories = คุณใช้เครื่องนี้ทำอะไร?
    # btn
    .confirm =
    # btn; shown on new windows for closing
    .done =

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
    .desc =
    .loading =

## page: Finish
page-finish =
    .desc =
    # btn; closes the entire application
    .done =

## page: Error
page-error =
    .desc =
    # btn; closes the entire application
    .done =
    # btn
    .retry =
