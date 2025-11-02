# shown on places like "keyboard layout variant: Default"
default = デフォルト
# btn (previous page)
prev = 前
# btn (next page)
next = 次
# shown on the progress bar
steps-dnfdownloadupdate = システムアップデートをダウンロード中…
steps-dnfinstallupdate = システムアップデートをインストール中…
steps-script = スクリプト実行中…
steps-dnfdownloadapps = アプリをダウンロード中…
steps-dnfinstallapps = アプリをインストール中…
steps-driverscodecs = ドライバーとコーデックをインストール中…
imelangs-ch = 中国語
imelangs-ja = 日本語
imelangs-ko = 韓国語
imelangs-vi = ベトナム語
imelangs-in = インド語群
imelangs-th = タイ語
imelangs-display = { $lang_name }（{ $native_lang_name }）

## page: Welcome

# page title
page-welcome = { $distro } へようこそ
page-welcome-ready = デバイスをセットアップしましょう。
page-welcome-go = 次
page-welcome-skipcfg = スキップ

## page: Keyboard Layout

# page title
page-keyboard = キーボード配列
page-keyboard-search-layout = キー配列を検索…
page-keyboard-search-variant = キーバリエーションを検索…

## page: Who are You?

# page title
# remember, if you can't translate things literally, think of what makes sense here as a page heading
page-whoareyou = ユーザー登録
page-whoareyou-fullname = 名前
page-whoareyou-username = ユーザー名
page-whoareyou-error = ユーザー名は、英小文字で始まり、英小文字・数字・<tt>_</tt>・<tt>-</tt>だけで組むことが必要です。

## page: Create a Password

# page title
# the `.pw` attribute is a text field placeholder
# the `.rp` attribute is yet another text field placeholder
page-password = パスワード作成
page-password-pw = パスワード
page-password-rp = 確認

## page: Internet

# page title
page-internet = インターネット接続
page-internet-desc = インターネットに接続すると、最新ソフトをダウンロードできるようになります。
page-internet-skip = スキップ
page-internet-warn = コーデック・デバイスドライバ・アプリをインストールするにはネット接続が必要です。
page-internet-open = Wi-Fi接続

## page: Codecs and Drivers

page-codecs = コーデックとデバイスドライバ
page-codecs-desc1 = コーデック・デバイスドライバをインストールします。
page-codecs-desc2 = ネット接続が必要なので、接続のない場合は、{ $wiki }をご覧ください。
page-codecs-wiki = ウィキの記事
switch-codecs = コーデック・デバイスドライバをインストール
switch-codecs-desc = オンの場合はコーデック・デバイスドライバをインストールします

## page: Input Method

# page title
page-inputmethod = 入力方法
page-inputmethod-wiki = ウィキ
page-inputmethod-search-lang = 言語検索…
page-inputmethod-search-ims = 入力方法検索…

## page: Nightlight

# page title
page-nightlight = 夜間モード
switch-nightlight = 夜間モード

## page: Choose Your Theme

# page title
page-theme = テーマ選択
page-theme-desc =
    色の綴でシステムをカスタマイズしましょう。
    後でシステム設定に変えることもできます。
page-theme-note = このオプションは従われない場合もあります。
page-theme-light = ライト
page-theme-dark = ダーク

## page: Browser Selection

page-browser = ブラウザ選択

## page: Categories
## this page shows different categories in the app catalogue (e.g. Productivity, Gaming, Media Prod., etc.)

page-categories = ユーザーアプリ
page-categories-confirm = 続きにセットアップ
page-categories-done = 完了
# category names
# you should use a selector; e.g.
# { $cat ->
#   [Productivity] -> projdsalkaj;cjzkpqi
#   [...]
# }
# the list of categories is available at 
# https://github.com/Ultramarine-Linux/taidan/tree/master/catalogue
# open each yml file, check the first line for the name
categories = { $cat }

## page: Installing

page-installing = アプリをインストール中
page-installing-desc = しばらくお待ちください。
page-installing-loading = ローディング…

## page: Finish

page-finish = 準備はよろしいでしょうか？
page-finish-desc = 新しいシステムをお楽しみくださいませ！
page-finish-done = 完了

## page: Error

page-error = エラー
page-error-desc = 問題が発生しました。
page-error-done = 閉じる
page-error-retry = リトライ
skip = スキップ
page-language = 言語
page-language-search-lang = 言語検索…
page-internet-ok = ネットに接続しました！「次」を押してください。
page-internet-portal = ウェブブラウザでログイン
page-tweaks = システム微調整
page-inputmethod-desc =
    非ラテン文字を用いる言語は<b>入力方法</b>が必要となる場合がございます。
    「入力方法を追加」を押し、インストールすることが可能です。
    詳細は{ $wiki }をご参考ください。
page-inputmethod-add = 入力方法を追加
page-nightlight-desc =
    夜間に画面を暖かい色調にして目の疲れを軽減します。
    ナイトライトが寝付きの悪さを改善する効果は証明されていません。
flatpakening-name = Flatpakの代わりにRPMをインストール
flatpakening-desc = Flatpakは安全性が高い一方、RPMはスペースを節約できます。
page-analytics = 分析とクラッシュレポート
page-crashreport = クラッシュレポート
page-crashreport-desc = { $org }が不具合の特定やサポートのためにクラッシュデータを収集することを許可します。
page-crashreport-lbl1 = 収集されるすべてのデータは匿名化され、エンドツーエンドで暗号化されています。
page-crashreport-lbl2 = サポートが問題の原因を特定できるように、クラッシュ ID が付与されます。
switch-crashreport = クラッシュデータを送信
switch-crashreport-desc = オフのままにするには次へを押してください
page-installing-flatpak = { $n }つのFlatpakをインストールしています…
