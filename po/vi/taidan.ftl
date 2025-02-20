# shown on places like "keyboard layout variant: Default"
default = Mặc định
# btn (previous page)
prev = Trước
# btn (next page)
next = Sau

# shown on the progress bar
steps =
    .dnfdownloadupdate = Đang tải xuống các cập nhật hệ thống…
    .dnfinstallupdate = Đang cài đặt các cập nhật hệ thống…
    .script =
    .dnfdownloadapps = Đang tải xuống các ứng dụng…
    .dnfinstallapps = Đang cài đặt các ứng dụng…
    .driverscodecs = Đang cài đặt các trình điều khiển bổ sung…

imelangs =
    .ch = Tiếng Trung
    .ja = Tiếng Nhật
    .ko = Tiếng Hàn
    .vi = Tiếng Việt
    .in = Tiếng Ấn Độ
    .th = Tiếng Thái

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
page-welcome = Chào mừng đến với {$distro}
    .ready = Cùng chuẩn bị hệ thống mới của bạn.
    # ≈ "Next" btn; on click, goes to the next page
    .go = Bắt đầu nào
    # also a "Next" btn, but skips some pages
    .skipcfg = Bỏ qua bước cấu hình

## page: Keyboard Layout
# page title
page-keyboard = Bố cục bàn phím
    # search field placeholder
    .search-layout = Tìm kiếm bố cục bàn phím…
    # search field placeholder
    .search-variant = Tìm kiếm biến thể bàn phím…

## page: Who are You?
# page title
# remember, if you can't translate things literally, think of what makes sense here as a page heading
page-whoareyou = Giới thiệu bạn là ai?
    # text field placeholder
    .fullname = Tên đầy đủ
    # text field placeholder
    .username = Tên người dùng
    # multiline, explanation pops out if invalid username
    .error = Tên người dùng
        - phải bắt đầu bằng chữ thường
        - chỉ được chứa chữ cái và số, dấu gạch dưới (<tt>_</tt>) hoặc dấu gạch ngang (<tt>-</tt>)

## page: Create a Password
# page title
# the `.pw` attribute is a text field placeholder
# the `.rp` attribute is yet another text field placeholder
page-password = Hãy đặt mật khẩu
    .pw = Mật khẩu
    .rp = Nhập lại mật khẩu

## page: Internet
# page title
page-internet = Hãy đưa bạn lên mạng
    .desc = Kết nối mạng để có được điều tuyệt nhất và mới nhất.
    # btn
    .skip = Tôi không có mạng
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
page-codecs = Codec và trình điều khiển
    .desc1 = Cài đặt codec và trình điều khiển thiết bị độc quyền cho thiết bị của bạn.
    .desc2 = Tham khảo {$wiki} nếu bạn không có kết nối Internet.
    # we need to make this another attribute because we need to tag this as a link
    .wiki = wiki
switch-codecs = Cài đặt Codec và trình điều khiển
    .desc = Nhấn tiếp để bỏ qua việc cài đặt

## page: Input Method
# page title
page-inputmethod = Phương thức nhập
    .desc1 = Bạn có thể <b>tùy chọn</b>cài đặt thêm {$ime}. Điều này giúp cho bạn có thể nhập ở một số ngôn ngữ nhất định Thay đổi này sẽ có hiệu lực sau khi bạn đăng nhập vào tài khoản của mình.
    .desc2 = Nhiều tùy chọn phương thức nhập tiếng Trung có thể khả dụng hơn với công cụ {$rime}, nhưng điều này yêu cầu cấu hình nâng cao và do đó không được khuyến nghị cho người mới bắt đầu.
    .desc3 = Bạn có thể tìm hiểu thêm thông tin ở {$wiki}.
    .ime = trình điều khiểu phương thức nhập (IME)
    .rime = Rime
    .wiki = wiki
    # search field placeholder
    .search-lang = Tìm kiếm ngôn ngữ…
    # search field placeholder
    .search-ims = Tìm kiếm IMs/IMEs…

## page: Nightlight
# page title
page-nightlight = Ánh sáng đêm
    .lbl1 = Phủ màn hình bằng tông màu ấm vào ban đêm để giảm mỏi mắt.
    .lbl2 = Ánh sáng đêm không được chứng minh là hỗ trợ việc mất ngủ.

switch-nightlight = {page-nightlight}

## page: Choose Your Theme
# page title
page-theme = Chọn chủ đề bạn muốn
    .desc = Biến hệ thống này thành của riêng bạn với một chút màu sắc.
            Bạn có thể thay đổi tùy chọn này sau trong phần cài đặt.
    .note = Một số ứng dụng sẽ không tuân theo tùy chọn này.
    # Light Theme
    .light = Sáng
    # Dark Theme
    .dark = Tối

## page: Browser Selection
page-browser = Chọn trình duyệt

## page: Categories
## this page shows different categories in the app catalogue (e.g. Productivity, Gaming, Media Prod., etc.)
page-categories = Bạn sẽ dùng thiết bị này như thế nào?
    # btn
    .confirm = Xác nhận và cài đặt hệ thống
    # btn; shown on new windows for closing
    .done = Xong

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
page-installing = Dang cài đặt các ứng dụng
    .desc = Sẽ không lâu lắm đâu!
    .loading = Đang tải…

## page: Finish
page-finish = Hệ thống của bạn đã sẵn sàng
    .desc = Tận hưởng hệ thống mới của bạn!
    # btn; closes the entire application
    .done = {page-categories.done}

## page: Error
page-error = Lỗi
    .desc = Rất tiếc, đã có lỗi xảy ra.
    # btn; closes the entire application
    .done = {page-categories.done}
    # btn
    .retry = Thử lại
