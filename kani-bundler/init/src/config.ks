;=========================================
; コンフィグ モード　画面作成
;=========================================
;メッセージレイヤ0を不可視に
	[layopt layer="message0" visible="false"]
;fixボタンをクリア
	[clearfix]
;キーコンフィグの無効化
	[stop_keyconfig]
;メニューボタン非表示
	[hidemenubutton]
	[iscript]
	; ラベル通過記録を有効に（kani-engineホストに転送される拡張タグで設定）
	; 初期値（TyranoScriptのデフォルト値を使用）
	tf.current_bgm_vol   = 100; // BGM音量
	tf.current_se_vol    = 100; // SE音量
	tf.current_ch_speed  = 30;  // テキスト表示速度
	tf.current_auto_speed = 2000; // オート時のテキスト表示速度
	tf.text_skip = "OFF"; // 未読スキップ
	[endscript]
	[iscript]
	/* 画像類のパス */
	tf.img_path = "image/config/";
	/* 画像類のパス（ボタン） */
	tf.btn_path_off = tf.img_path + "c_btn.gif";
	tf.btn_path_on  = tf.img_path + "c_set.png";
	// ボタン画像の幅と高さ
	tf.btn_w = 46;
	tf.btn_h = 46;
	// ボタンを表示する座標
	tf.config_x = [1040, 400, 454, 508, 562, 616, 670, 724, 778, 832, 886];
	tf.config_y_bgm  = 190; // BGMのY座標
	tf.config_y_se   = 250; // SEのY座標
	tf.config_y_ch   = 325; // テキスト速度のY座標
	tf.config_y_auto = 385; // オート速度のY座標
	// テキスト速度のサンプルテキスト
	tf.text_sample = "テストメッセージです。このスピードでテキストが表示されます。";
	[endscript]
[cm]
;コンフィグ用の背景を読み込んでトランジション
	[bg storage=&tf.img_path+"bg_config.png" time="100"]
;画面右上の「Back」ボタン
	[button fix="true" graphic=&tf.img_path+"c_btn_back.png" enterimg=&tf.img_path+"c_btn_back2.png" target="*backtitle" x="1160" y="20"]
[jump target="*config_page"]
*config_page
;------------------------------------------------------------------------------------------------------
; BGM音量
;------------------------------------------------------------------------------------------------------
	[button name="bgmvol,bgmvol_10"  fix="true" target="*vol_bgm_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[1]  y=&tf.config_y_bgm exp="tf.current_bgm_vol =  10;"]
	[button name="bgmvol,bgmvol_20"  fix="true" target="*vol_bgm_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[2]  y=&tf.config_y_bgm exp="tf.current_bgm_vol =  20;"]
	[button name="bgmvol,bgmvol_30"  fix="true" target="*vol_bgm_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[3]  y=&tf.config_y_bgm exp="tf.current_bgm_vol =  30;"]
	[button name="bgmvol,bgmvol_40"  fix="true" target="*vol_bgm_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[4]  y=&tf.config_y_bgm exp="tf.current_bgm_vol =  40;"]
	[button name="bgmvol,bgmvol_50"  fix="true" target="*vol_bgm_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[5]  y=&tf.config_y_bgm exp="tf.current_bgm_vol =  50;"]
	[button name="bgmvol,bgmvol_60"  fix="true" target="*vol_bgm_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[6]  y=&tf.config_y_bgm exp="tf.current_bgm_vol =  60;"]
	[button name="bgmvol,bgmvol_70"  fix="true" target="*vol_bgm_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[7]  y=&tf.config_y_bgm exp="tf.current_bgm_vol =  70;"]
	[button name="bgmvol,bgmvol_80"  fix="true" target="*vol_bgm_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[8]  y=&tf.config_y_bgm exp="tf.current_bgm_vol =  80;"]
	[button name="bgmvol,bgmvol_90"  fix="true" target="*vol_bgm_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[9]  y=&tf.config_y_bgm exp="tf.current_bgm_vol =  90;"]
	[button name="bgmvol,bgmvol_100" fix="true" target="*vol_bgm_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[10] y=&tf.config_y_bgm exp="tf.current_bgm_vol = 100;"]
;BGMミュート
	[button name="bgmvol,bgmvol_0" fix="true" target="*vol_bgm_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[0] y=&tf.config_y_bgm exp="tf.current_bgm_vol = 0;"]
;------------------------------------------------------------------------------------------------------
; SE音量
;------------------------------------------------------------------------------------------------------
	[button name="sevol,sevol_10"  fix="true" target="*vol_se_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[1]  y=&tf.config_y_se exp="tf.current_se_vol =  10;"]
	[button name="sevol,sevol_20"  fix="true" target="*vol_se_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[2]  y=&tf.config_y_se exp="tf.current_se_vol =  20;"]
	[button name="sevol,sevol_30"  fix="true" target="*vol_se_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[3]  y=&tf.config_y_se exp="tf.current_se_vol =  30;"]
	[button name="sevol,sevol_40"  fix="true" target="*vol_se_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[4]  y=&tf.config_y_se exp="tf.current_se_vol =  40;"]
	[button name="sevol,sevol_50"  fix="true" target="*vol_se_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[5]  y=&tf.config_y_se exp="tf.current_se_vol =  50;"]
	[button name="sevol,sevol_60"  fix="true" target="*vol_se_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[6]  y=&tf.config_y_se exp="tf.current_se_vol =  60;"]
	[button name="sevol,sevol_70"  fix="true" target="*vol_se_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[7]  y=&tf.config_y_se exp="tf.current_se_vol =  70;"]
	[button name="sevol,sevol_80"  fix="true" target="*vol_se_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[8]  y=&tf.config_y_se exp="tf.current_se_vol =  80;"]
	[button name="sevol,sevol_90"  fix="true" target="*vol_se_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[9]  y=&tf.config_y_se exp="tf.current_se_vol =  90;"]
	[button name="sevol,sevol_100" fix="true" target="*vol_se_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[10] y=&tf.config_y_se exp="tf.current_se_vol = 100;"]
;SEミュート
	[button name="sevol,sevol_0" fix="true" target="*vol_se_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[0] y=&tf.config_y_se exp="tf.current_se_vol = 0;"]
;------------------------------------------------------------------------------------------------------
; テキスト速度
;------------------------------------------------------------------------------------------------------
	[button name="ch,ch_100" fix="true" target="*ch_speed_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[1]  y=&tf.config_y_ch exp="tf.set_ch_speed = 100;"]
	[button name="ch,ch_80"  fix="true" target="*ch_speed_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[2]  y=&tf.config_y_ch exp="tf.set_ch_speed =  80;"]
	[button name="ch,ch_50"  fix="true" target="*ch_speed_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[3]  y=&tf.config_y_ch exp="tf.set_ch_speed =  50;"]
	[button name="ch,ch_40"  fix="true" target="*ch_speed_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[4]  y=&tf.config_y_ch exp="tf.set_ch_speed =  40;"]
	[button name="ch,ch_30"  fix="true" target="*ch_speed_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[5]  y=&tf.config_y_ch exp="tf.set_ch_speed =  30;"]
	[button name="ch,ch_25"  fix="true" target="*ch_speed_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[6]  y=&tf.config_y_ch exp="tf.set_ch_speed =  25;"]
	[button name="ch,ch_20"  fix="true" target="*ch_speed_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[7]  y=&tf.config_y_ch exp="tf.set_ch_speed =  20;"]
	[button name="ch,ch_11"  fix="true" target="*ch_speed_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[8]  y=&tf.config_y_ch exp="tf.set_ch_speed =  11;"]
	[button name="ch,ch_8"   fix="true" target="*ch_speed_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[9]  y=&tf.config_y_ch exp="tf.set_ch_speed =   8;"]
	[button name="ch,ch_5"   fix="true" target="*ch_speed_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[10] y=&tf.config_y_ch exp="tf.set_ch_speed =   5;"]
;------------------------------------------------------------------------------------------------------
; オート速度
;------------------------------------------------------------------------------------------------------
	[button name="auto,auto_5000" fix="true" target="*auto_speed_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[1]  y=&tf.config_y_auto exp="tf.set_auto_speed = 5000;"]
	[button name="auto,auto_4500" fix="true" target="*auto_speed_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[2]  y=&tf.config_y_auto exp="tf.set_auto_speed = 4500;"]
	[button name="auto,auto_4000" fix="true" target="*auto_speed_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[3]  y=&tf.config_y_auto exp="tf.set_auto_speed = 4000;"]
	[button name="auto,auto_3500" fix="true" target="*auto_speed_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[4]  y=&tf.config_y_auto exp="tf.set_auto_speed = 3500;"]
	[button name="auto,auto_3000" fix="true" target="*auto_speed_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[5]  y=&tf.config_y_auto exp="tf.set_auto_speed = 3000;"]
	[button name="auto,auto_2500" fix="true" target="*auto_speed_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[6]  y=&tf.config_y_auto exp="tf.set_auto_speed = 2500;"]
	[button name="auto,auto_2000" fix="true" target="*auto_speed_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[7]  y=&tf.config_y_auto exp="tf.set_auto_speed = 2000;"]
	[button name="auto,auto_1300" fix="true" target="*auto_speed_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[8]  y=&tf.config_y_auto exp="tf.set_auto_speed = 1300;"]
	[button name="auto,auto_800"  fix="true" target="*auto_speed_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[9]  y=&tf.config_y_auto exp="tf.set_auto_speed =  800;"]
	[button name="auto,auto_500"  fix="true" target="*auto_speed_change" graphic=&tf.btn_path_off width=&tf.btn_w height=&tf.btn_h x=&tf.config_x[10] y=&tf.config_y_auto exp="tf.set_auto_speed =  500;"]
;------------------------------------------------------------------------------------------------------
; 未読スキップ
;------------------------------------------------------------------------------------------------------
;未読スキップ-OFF
	[button name="unread_off" fix="true" target="*skip_off" graphic=&tf.btn_path_off width="170" height="45" x="400" y="470"]
;未読スキップ-ON
	[button name="unread_on"  fix="true" target="*skip_on"  graphic=&tf.btn_path_off width="170" height="45" x="580" y="470"]
[s]
;--------------------------------------------------------------------------------
; コンフィグモードの終了
;--------------------------------------------------------------------------------
*backtitle
[cm]
;テキスト速度のサンプル表示に使用していたメッセージレイヤを非表示に
	[layopt layer="message1" visible="false"]
;fixボタンをクリア
	[clearfix]
;キーコンフィグの有効化
	[start_keyconfig]
;コールスタックのクリア
	[clearstack]
;ゲーム復帰
	[awakegame]
;================================================================================
; ボタンクリック時の処理
;================================================================================
;--------------------------------------------------------------------------------
; BGM音量
;--------------------------------------------------------------------------------
*vol_bgm_change
[bgmopt volume=&tf.current_bgm_vol]
[return]
;--------------------------------------------------------------------------------
; SE音量
;--------------------------------------------------------------------------------
*vol_se_change
[seopt volume=&tf.current_se_vol]
[return]
;---------------------------------------------------------------------------------
; テキスト速度
;--------------------------------------------------------------------------------
*ch_speed_change
	[iscript]
	tf.current_ch_speed = tf.set_ch_speed;
	[endscript]
	[configdelay speed=&tf.set_ch_speed]
;テキスト表示速度のサンプルに使用するメッセージレイヤの設定
	[position layer="message1" left="90" top="580" width="1100" height="100" margint="2" marginl="30" page="fore" visible="true" opacity="0"]
	[layopt layer="message1" visible="true"]
	[current layer="message1"]
;サンプルテキストを表示する
	[emb exp="tf.text_sample"]
;待ち時間をテキスト速度とサンプルの文字数に対応
	[eval exp="tf.text_sample_speed = tf.set_ch_speed * tf.text_sample.len() + 700"]
	[wait time=&tf.text_sample_speed]
	[er]
	[layopt layer="message1" visible="false"]
[return]
;--------------------------------------------------------------------------------
; オート速度
;--------------------------------------------------------------------------------
*auto_speed_change
	[autoconfig speed=&tf.set_auto_speed]
[return]
;--------------------------------------------------------------------------------
; スキップ処理-OFF
;--------------------------------------------------------------------------------
*skip_off
	[iscript]
	tf.text_skip = "OFF";
	[endscript]
	[config_record_label skip="false"]
[return]
;--------------------------------------------------------------------------------
; スキップ処理-ON
;--------------------------------------------------------------------------------
*skip_on
	[iscript]
	tf.text_skip = "ON";
	[endscript]
	[config_record_label skip="true"]
[return]
