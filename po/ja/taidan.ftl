# shown on places like "keyboard layout variant: Default"
default = デフォルト
# btn (previous page)
prev = 前
# btn (next page)
next = 次

# shown on the progress bar
steps =
    .dnfdownloadupdate = システムアップデートダウンロード中…
    .dnfinstallupdate = システムアップデートインストール中…
    .script = ディストリビューションスクリプト実行中…
    .dnfdownloadapps = ユーザーアプリダウンロード中…
    .dnfinstallapps = ユーザーアプリインストール中…
    .driverscodecs = システムアップデートインストール中…

imelangs =
    .ch = 中国語
    .ja = 日本語
    .ko = 韓国語
    .vi = ベトナム語
    .in = インド語群
    .th = タイ語
    .display = {$lang_name}（{$native_lang_name}）

## page: Welcome

# page title
page-welcome = {$distro} へようこそ
    .ready = システムをセットアップしましょう。
    .go = 次
    .skipcfg = スキップ

## page: Keyboard Layout
# page title
page-keyboard = キーボード配列
    .search-layout = キー配列を検索…
    .search-variant = キーバリエーションを検索…

## page: Who are You?
# page title
# remember, if you can't translate things literally, think of what makes sense here as a page heading
page-whoareyou = ユーザー登録
    .fullname = 名前
    .username = ユーザー名
    .error = ユーザー名のフォーマットについて
        - 英小文字で始まる
        - 英文字・数字・<tt>_</tt>・<tt>-</tt>だけで組む

## page: Create a Password
# page title
# the `.pw` attribute is a text field placeholder
# the `.rp` attribute is yet another text field placeholder
page-password = パスワード作成
    .pw = パスワード
    .rp = 確認

## page: Internet
# page title
page-internet = インターネット接続
    .desc = インターネットに接続すると、最新ソフトをダウンロードできるようになります。
    .skip = スキップ
    .warn = コーデック・デバイスドライバ・ユーザーアプリをインストールするにはネット接続が必要です。
    .open = Wi-Fi接続アプリを起動

## page: Codecs and Drivers
page-codecs = コーデックとデバイスドライバ
    .desc1 = 専売特許のコーデック・デバイスドライバをインストールしますか？
    .desc2 = ネット接続が必要なので、接続のない場合は、{$wiki}を読んでください。
    .wiki = ウィキの記事
switch-codecs = コーデック・デバイスドライバをインストール
    .desc = 不必要である方は、「次」のボタンを押してください

## page: Input Method
# page title
page-inputmethod = 入力方法
    .desc1 = {$ime}の追加次第で、英語以外の入力が可能になりますが、設定はログインした後だけ有効になります。
    .desc2 = <b>日本語はMozcを選択してください。</b>

        特定の中国語入力方法は{$rime}で利用可能かもしれませんが、セットアップが上級者向けのため、初心者におすすめしません。
    .desc3 = 詳細は{$wiki}にご覧ください。
    .ime = IME
    .rime = Rime（中州韻輸入法引擎）
    .wiki = ウィキ
    .search-lang = 言語検索…
    .search-ims = 入力方法検索…

## page: Nightlight
# page title
page-nightlight = 夜間モード
    .lbl1 = 画面の色合いを暖かくすることによって、目への負担を軽減することができます。
    .lbl2 =

switch-nightlight = {page-nightlight}

## page: Choose Your Theme
# page title
page-theme = テーマ選択
    .desc = 色の綴でシステムをカスタマイズしましょう。
            後でシステム設定に変えることもできます。
    .note = このオプションは従われない場合もあります。
    .light = ライト
    .dark = ダーク

## page: Browser Selection
page-browser = ブラウザ選択

## page: Categories
## this page shows different categories in the app catalogue (e.g. Productivity, Gaming, Media Prod., etc.)
page-categories = ユーザーアプリ
    .confirm = 続きにセットアップ
    .done = 完了

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
page-installing = アプリをインストール中
    .desc = しばらくお待ちください。
    .loading = ローディング…

## page: Finish
page-finish = 完成
    .desc = 新しいシステムをお楽しみくださいませ。
    .done = 完了

## page: Error
page-error = エラー
    .desc = 申し訳ございませんが、エラーが発生しました。
    .done = 閉じる
    .retry = リトライ
