*start
[eval exp="f.karma = 0; f.courage = 0; f.kindness = 0; f.ending = 0;"]
#ナレーション
あなたは気づいたら見知らぬ白い空間に立っていた。
@l
どうやら死んでしまったらしい。
@l
#女神
あら、お疲れ様でした。私は転生を管理する女神です。
@l
あなたの次の人生を決めるために、13の質問に答えてください。正直に答えることをお勧めしますよ。
@l

; ── 質問 1 ──────────────────────────────────────────────────────────────────

*q1
#女神
質問1: 見知らぬ人が財布を落としました。あなたはどうしますか？
@l
@link target=*q1_a
こっそり中身だけいただく。
@link target=*q1_b
一緒に持ち主を探してあげる。
@link target=*q1_c
すぐに警察へ届ける。
@endlink

*q1_a
[eval exp="f.karma = f.karma + 3; f.courage = f.courage + 1;"]
@jump target=*q2

*q1_b
[eval exp="f.karma = f.karma + 1; f.courage = f.courage + 3; f.kindness = f.kindness + 1;"]
@jump target=*q2

*q1_c
[eval exp="f.courage = f.courage + 1; f.kindness = f.kindness + 3;"]
@jump target=*q2

; ── 質問 2 ──────────────────────────────────────────────────────────────────

*q2
#女神
質問2: 試験前日、友人があなたに「答えを見せてほしい」と頼んできました。
@l
@link target=*q2_a
見せる代わりに報酬を要求する。
@link target=*q2_b
断り、一緒に勉強すると提案する。
@link target=*q2_c
黙って断る。
@endlink

*q2_a
[eval exp="f.karma = f.karma + 3; f.courage = f.courage + 1;"]
@jump target=*q3

*q2_b
[eval exp="f.karma = f.karma + 1; f.courage = f.courage + 3; f.kindness = f.kindness + 1;"]
@jump target=*q3

*q2_c
[eval exp="f.courage = f.courage + 1; f.kindness = f.kindness + 3;"]
@jump target=*q3

; ── 質問 3 ──────────────────────────────────────────────────────────────────

*q3
#女神
質問3: 街中で老人が倒れています。周りの人は誰も動きません。
@l
@link target=*q3_a
スマホで動画を撮って面白がる。
@link target=*q3_b
人目を気にせず即座に助け起こす。
@link target=*q3_c
通行人に声をかけて助けを求める。
@endlink

*q3_a
[eval exp="f.karma = f.karma + 3; f.courage = f.courage + 1;"]
@jump target=*q4

*q3_b
[eval exp="f.karma = f.karma + 1; f.courage = f.courage + 3; f.kindness = f.kindness + 1;"]
@jump target=*q4

*q3_c
[eval exp="f.courage = f.courage + 1; f.kindness = f.kindness + 3;"]
@jump target=*q4

; ── 質問 4 ──────────────────────────────────────────────────────────────────

*q4
#女神
質問4: 職場で自分のミスを誰かのせいにできる状況になりました。
@l
@link target=*q4_a
ためらわず部下を犯人にして報告する。
@link target=*q4_b
自ら名乗り出て責任を取る。
@link target=*q4_c
こっそり自分で修正して誰にも言わない。
@endlink

*q4_a
[eval exp="f.karma = f.karma + 3; f.courage = f.courage + 1;"]
@jump target=*q5

*q4_b
[eval exp="f.karma = f.karma + 1; f.courage = f.courage + 3; f.kindness = f.kindness + 1;"]
@jump target=*q5

*q4_c
[eval exp="f.courage = f.courage + 1; f.kindness = f.kindness + 3;"]
@jump target=*q5

; ── 質問 5 ──────────────────────────────────────────────────────────────────

*q5
#女神
質問5: 宝くじで一億円が当たりました。
@l
@link target=*q5_a
全額を自分の欲のために使い果たす。
@link target=*q5_b
半分を挑戦的な事業に投資し、残りを寄付する。
@link target=*q5_c
全額を慈善団体に寄付する。
@endlink

*q5_a
[eval exp="f.karma = f.karma + 3; f.courage = f.courage + 1;"]
@jump target=*q6

*q5_b
[eval exp="f.karma = f.karma + 1; f.courage = f.courage + 3; f.kindness = f.kindness + 1;"]
@jump target=*q6

*q5_c
[eval exp="f.courage = f.courage + 1; f.kindness = f.kindness + 3;"]
@jump target=*q6

; ── 質問 6 ──────────────────────────────────────────────────────────────────

*q6
#女神
質問6: 深夜の無人野菜販売所でお釣りが多めに戻ってきました。
@l
@link target=*q6_a
黙って多い分もいただく。
@link target=*q6_b
少し多めにお金を置いていく。
@link target=*q6_c
正確な額に直して戻す。
@endlink

*q6_a
[eval exp="f.karma = f.karma + 3; f.courage = f.courage + 1;"]
@jump target=*q7

*q6_b
[eval exp="f.karma = f.karma + 1; f.courage = f.courage + 3; f.kindness = f.kindness + 1;"]
@jump target=*q7

*q6_c
[eval exp="f.courage = f.courage + 1; f.kindness = f.kindness + 3;"]
@jump target=*q7

; ── 質問 7 ──────────────────────────────────────────────────────────────────

*q7
#女神
質問7: 親友の重大な秘密を偶然知ってしまいました。
@l
@link target=*q7_a
交渉材料として利用する。
@link target=*q7_b
親友に「知ってしまった」と打ち明ける。
@link target=*q7_c
永遠に胸の内に秘め、墓まで持っていく。
@endlink

*q7_a
[eval exp="f.karma = f.karma + 3; f.courage = f.courage + 1;"]
@jump target=*q8

*q7_b
[eval exp="f.karma = f.karma + 1; f.courage = f.courage + 3; f.kindness = f.kindness + 1;"]
@jump target=*q8

*q7_c
[eval exp="f.courage = f.courage + 1; f.kindness = f.kindness + 3;"]
@jump target=*q8

; ── 質問 8 ──────────────────────────────────────────────────────────────────

*q8
#女神
質問8: 道に迷った外国人旅行者があなたに道を聞いてきました。
@l
@link target=*q8_a
嘘の道を教えて楽しむ。
@link target=*q8_b
目的地まで直接案内する。
@link target=*q8_c
地図アプリを開いて一緒に確認する。
@endlink

*q8_a
[eval exp="f.karma = f.karma + 3; f.courage = f.courage + 1;"]
@jump target=*q9

*q8_b
[eval exp="f.karma = f.karma + 1; f.courage = f.courage + 3; f.kindness = f.kindness + 1;"]
@jump target=*q9

*q8_c
[eval exp="f.courage = f.courage + 1; f.kindness = f.kindness + 3;"]
@jump target=*q9

; ── 質問 9 ──────────────────────────────────────────────────────────────────

*q9
#女神
質問9: あなたのチームが重要なプレゼンで大失敗しました。
@l
@link target=*q9_a
すべてチームメンバーのせいにして上司に報告する。
@link target=*q9_b
自分が全責任を取ると宣言する。
@link target=*q9_c
チーム全員で謝罪に行く。
@endlink

*q9_a
[eval exp="f.karma = f.karma + 3; f.courage = f.courage + 1;"]
@jump target=*q10

*q9_b
[eval exp="f.karma = f.karma + 1; f.courage = f.courage + 3; f.kindness = f.kindness + 1;"]
@jump target=*q10

*q9_c
[eval exp="f.courage = f.courage + 1; f.kindness = f.kindness + 3;"]
@jump target=*q10

; ── 質問 10 ─────────────────────────────────────────────────────────────────

*q10
#女神
質問10: 隣人が明らかに違法なことをしているのを目撃しました。
@l
@link target=*q10_a
証拠を握って脅迫する。
@link target=*q10_b
直接注意しに乗り込む。
@link target=*q10_c
匿名で当局に通報する。
@endlink

*q10_a
[eval exp="f.karma = f.karma + 3; f.courage = f.courage + 1;"]
@jump target=*q11

*q10_b
[eval exp="f.karma = f.karma + 1; f.courage = f.courage + 3; f.kindness = f.kindness + 1;"]
@jump target=*q11

*q10_c
[eval exp="f.courage = f.courage + 1; f.kindness = f.kindness + 3;"]
@jump target=*q11

; ── 質問 11 ─────────────────────────────────────────────────────────────────

*q11
#女神
質問11: あなたの将来の夢は何ですか？
@l
@link target=*q11_a
誰よりも早く金持ちになること。
@link target=*q11_b
誰も成し遂げたことのない偉業を達成すること。
@link target=*q11_c
みんなが幸せに生きられる世界を作ること。
@endlink

*q11_a
[eval exp="f.karma = f.karma + 3; f.courage = f.courage + 1;"]
@jump target=*q12

*q11_b
[eval exp="f.karma = f.karma + 1; f.courage = f.courage + 3; f.kindness = f.kindness + 1;"]
@jump target=*q12

*q11_c
[eval exp="f.courage = f.courage + 1; f.kindness = f.kindness + 3;"]
@jump target=*q12

; ── 質問 12 ─────────────────────────────────────────────────────────────────

*q12
#女神
質問12: やりがいは抜群だが給料がかなり低い仕事に転職するチャンスが来ました。
@l
@link target=*q12_a
絶対に断る。お金が全てだ。
@link target=*q12_b
思い切って転職する。情熱こそすべて。
@link target=*q12_c
今の職場で社会貢献できる方法を探す。
@endlink

*q12_a
[eval exp="f.karma = f.karma + 3; f.courage = f.courage + 1;"]
@jump target=*q13

*q12_b
[eval exp="f.karma = f.karma + 1; f.courage = f.courage + 3; f.kindness = f.kindness + 1;"]
@jump target=*q13

*q12_c
[eval exp="f.courage = f.courage + 1; f.kindness = f.kindness + 3;"]
@jump target=*q13

; ── 質問 13 ─────────────────────────────────────────────────────────────────

*q13
#女神
最後の質問です。あなたにとっての「幸福」とは何ですか？
@l
@link target=*q13_a
自分だけが豊かであること。
@link target=*q13_b
恐れを捨てて自分の道を歩むこと。
@link target=*q13_c
周りの人々の笑顔。
@endlink

*q13_a
[eval exp="f.karma = f.karma + 3; f.courage = f.courage + 1;"]
@jump target=*angel_section

*q13_b
[eval exp="f.karma = f.karma + 1; f.courage = f.courage + 3; f.kindness = f.kindness + 1;"]
@jump target=*angel_section

*q13_c
[eval exp="f.courage = f.courage + 1; f.kindness = f.kindness + 3;"]
@jump target=*angel_section

; ── 天使の登場とメタファー ────────────────────────────────────────────────────

*angel_section
#女神
ありがとうございました。あとは天使さんにお任せしますね。
@l
#天使
やあ、よく来たね。
@l
川は必ず海に注ぐ。種は必ず季節が来れば芽を吹く。あなたの魂も、その本性に従った形へと咲き誇るでしょう。
@l
#Me
あの……それはどういう意味ですか？
@l
#天使
つまり……次の人生では、自分らしく生きることができるということです。
@l
さあ、新しい扉を開けてください。
@l

; ── 運命の判定（Rhaiで算出）──────────────────────────────────────────────────

[eval exp="if f.karma <= 8 && f.courage >= 20 && f.kindness >= 20 { f.ending = 3; } else if f.karma >= 20 && f.courage >= 20 && f.kindness >= 9 { f.ending = 2; } else if f.karma >= 20 { f.ending = 4; } else if f.courage >= 20 { f.ending = 5; } else { f.ending = 1; }"]

; ── エンディングナレーション ─────────────────────────────────────────────────

[if exp="f.ending == 1"]
#ナレーション
エンディング：普通人
@l
あなたは穏やかで平凡な普通人として転生しました。波風立たず、それでいて温かな日々があなたを待っています。
@l
[endif]

[if exp="f.ending == 2"]
#ナレーション
エンディング：怪盗
@l
あなたは義賊として名高い怪盗に転生しました。知恵と勇気と少しの良心で、伝説の始まりを告げる鐘が鳴り響きます。
@l
[endif]

[if exp="f.ending == 3"]
#ナレーション
エンディング：偉大なるプログラマー
@l
あなたは偉大なるプログラマーとして転生しました。世界を変えるコードを書く使命があなたを待っています。謙虚に、勇敢に、優しく。
@l
[endif]

[if exp="f.ending == 4"]
#ナレーション
エンディング：ならず者
@l
あなたはならず者として転生しました。欲望のままに生きる波乱万丈な人生。誰もあなたを止めることはできない。
@l
[endif]

[if exp="f.ending == 5"]
#ナレーション
エンディング：エクストリームスポーツ選手
@l
あなたはエクストリームスポーツ選手として転生しました。限界に挑み続ける日々が、あなたに生の歓喜をもたらすでしょう。
@l
[endif]