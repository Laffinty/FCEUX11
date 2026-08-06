#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# hotfix4 i18n gap-fill: translate all `unfinished` entries in the 11
# non-English .ts files. Machine-translation style, reviewed to avoid
# religious / folk-taboo wording; untranslatable technical terms
# (FCC, FilePos, NTSC/PAL/Dendy, zoom factors) intentionally stay English
# per project policy ("确实无法翻译词和句的宁愿用英语代替").
# Usage: python scripts/_hotfix4_fill_translations.py
import re, sys, io, os

os.chdir(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "src", "drivers", "Qt", "lang"))
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8")

# Language order for the translation tuples below.
LANGS = ["zh_CN", "zh_TW", "ja", "ko", "es", "fr", "de", "vi", "th", "hi", "ar"]

# key: source text with XML entities UNescaped and whitespace collapsed.
# value: tuple of 11 translations in LANGS order.
T = {
"AVI RIFF Viewer": ("AVI RIFF 查看器","AVI RIFF 檢視器","AVI RIFF ビューア","AVI RIFF 뷰어","Visor AVI RIFF","Visionneuse AVI RIFF","AVI-RIFF-Viewer","Trình xem AVI RIFF","ตัวดู AVI RIFF","AVI RIFF व्यूअर","عارض AVI RIFF"),
"Block": ("块","區塊","ブロック","블록","Bloque","Bloc","Block","Khối","บล็อก","ब्लॉक","كتلة"),
"FCC": ("FCC",)*11,
"Size": ("大小","大小","サイズ","크기","Tamaño","Taille","Größe","Kích thước","ขนาด","आकार","الحجم"),
"FilePos": ("FilePos",)*11,
"PRG Logged Code:": ("PRG 已记录代码:","PRG 已記錄程式碼:","PRG 記録済みコード:","PRG 기록된 코드:","Código registrado PRG:","Code PRG enregistré :","PRG-Code protokolliert:","Mã PRG đã ghi:","โค้ด PRG ที่บันทึก:","PRG लॉग कोड:","الشيفرة المسجلة PRG:"),
"PRG Logged Data:": ("PRG 已记录数据:","PRG 已記錄資料:","PRG 記録済みデータ:","PRG 기록된 데이터:","Datos registrados PRG:","Données PRG enregistrées :","PRG-Daten protokolliert:","Dữ liệu PRG đã ghi:","ข้อมูล PRG ที่บันทึก:","PRG लॉग डेटा:","البيانات المسجلة PRG:"),
"PRG Unmapped:": ("PRG 未映射:","PRG 未映射:","PRG 未マップ:","PRG 매핑되지 않음:","PRG no mapeado:","PRG non mappé :","PRG nicht gemappt:","PRG chưa ánh xạ:","PRG ที่ไม่ได้แมป:","PRG अनमैप्ड:","PRG غير معيّن:"),
"CHR Logged Code:": ("CHR 已记录代码:","CHR 已記錄程式碼:","CHR 記録済みコード:","CHR 기록된 코드:","Código registrado CHR:","Code CHR enregistré :","CHR-Code protokolliert:","Mã CHR đã ghi:","โค้ด CHR ที่บันทึก:","CHR लॉग कोड:","الشيفرة المسجلة CHR:"),
"CHR Logged Data:": ("CHR 已记录数据:","CHR 已記錄資料:","CHR 記録済みデータ:","CHR 기록된 데이터:","Datos registrados CHR:","Données CHR enregistrées :","CHR-Daten protokolliert:","Dữ liệu CHR đã ghi:","ข้อมูล CHR ที่บันทึก:","CHR लॉग डेटा:","البيانات المسجلة CHR:"),
"CHR Unmapped:": ("CHR 未映射:","CHR 未映射:","CHR 未マップ:","CHR 매핑되지 않음:","CHR no mapeado:","CHR non mappé :","CHR nicht gemappt:","CHR chưa ánh xạ:","CHR ที่ไม่ได้แมป:","CHR अनमैप्ड:","CHR غير معيّن:"),
"Auto Save CDL": ("自动保存 CDL","自動儲存 CDL","CDL を自動保存","CDL 자동 저장","Guardar CDL automáticamente","Enregistrer CDL auto.","CDL automatisch speichern","Tự động lưu CDL","บันทึก CDL อัตโนมัติ","CDL स्वतः सहेजें","حفظ CDL تلقائيًا"),
"Auto Load CDL": ("自动载入 CDL","自動載入 CDL","CDL を自動読込","CDL 자동 불러오기","Cargar CDL automáticamente","Charger CDL auto.","CDL automatisch laden","Tự động tải CDL","โหลด CDL อัตโนมัติ","CDL स्वतः लोड करें","تحميل CDL تلقائيًا"),
"Auto Resume Logging": ("自动恢复日志记录","自動恢復記錄","ログ記録を自動再開","로그 기록 자동 재개","Reanudar registro automáticamente","Reprendre le journal auto.","Protokollierung auto. fortsetzen","Tự động tiếp tục ghi nhật ký","กลับมาบันทึกอัตโนมัติ","लॉगिंग स्वतः फिर शुरू","استئناف التسجيل تلقائيًا"),
"6502 Debugger": ("6502 调试器","6502 偵錯器","6502 デバッガ","6502 디버거","Depurador 6502","Débogueur 6502","6502-Debugger","Trình gỡ lỗi 6502","ดีบักเกอร์ 6502","6502 डीबगर","مصحح أخطاء 6502"),
"Family Keyboard": ("家庭键盘","家庭鍵盤","ファミリーキーボード","패밀리 키보드","Teclado Family","Clavier Family","Family-Tastatur","Bàn phím Family","คีย์บอร์ดแฟมิลี","फैमिली कीबोर्ड","لوحة مفاتيح فاميلي"),
"Press a key to map...": ("请按要映射的按键...","請按要映射的按鍵...","割り当てるキーを押してください...","매핑할 키를 누르세요...","Pulse una tecla para asignar...","Appuyez sur une touche à mapper...","Taste zum Belegen drücken...","Nhấn phím cần gán...","กดปุ่มที่ต้องการผูก...","मैप करने के लिए कोई कुंजी दबाएँ...","اضغط مفتاحًا لتعيينه..."),
"Frame Timing Statistics": ("帧时序统计","幀時序統計","フレームタイミング統計","프레임 타이밍 통계","Estadísticas de tiempo de fotograma","Statistiques de timing d'image","Frame-Timing-Statistik","Thống kê thời gian khung hình","สถิติไทม์มิงเฟรม","फ्रेम टाइमिंग आंकड़े","إحصاءات توقيت الإطار"),
"Game Genie Encoder/Decoder Tool": ("Game Genie 编码/解码工具","Game Genie 編碼/解碼工具","Game Genie エンコード/デコードツール","Game Genie 인코더/디코더 도구","Herramienta codificador/decodificador Game Genie","Outil encodeur/décodeur Game Genie","Game-Genie-Kodierer/Dekodierer","Công cụ mã hóa/giải mã Game Genie","เครื่องมือเข้ารหัส/ถอดรหัส Game Genie","Game Genie एन्कोडर/डिकोडर टूल","أداة ترميز/فك ترميز Game Genie"),
"Add Cheat": ("添加金手指","新增金手指","チートを追加","치트 추가","Añadir truco","Ajouter un code","Cheat hinzufügen","Thêm cheat","เพิ่มโกง","चीट जोड़ें","إضافة كود غش"),
"Search Results": ("搜索结果","搜尋結果","検索結果","검색 결과","Resultados de búsqueda","Résultats de recherche","Suchergebnisse","Kết quả tìm kiếm","ผลการค้นหา","खोज परिणाम","نتائج البحث"),
"Known Value": ("已知值","已知值","既知の値","알려진 값","Valor conocido","Valeur connue","Bekannter Wert","Giá trị đã biết","ค่าที่ทราบ","ज्ञात मान","القيمة المعروفة"),
"Not Equal Search": ("不相等搜索","不相等搜尋","等しくない値を検索","같지 않은 값 검색","Búsqueda de no igual","Recherche différent","Suche ungleich","Tìm giá trị khác","ค้นหาค่าที่ไม่เท่ากัน","असमान खोज","بحث عن غير مساوٍ"),
"Greater Than Search": ("大于搜索","大於搜尋","より大きい値を検索","큰 값 검색","Búsqueda de mayor que","Recherche supérieur à","Suche größer als","Tìm giá trị lớn hơn","ค้นหาค่าที่มากกว่า","बड़ा मान खोज","بحث عن أكبر من"),
"Less Than Search": ("小于搜索","小於搜尋","より小さい値を検索","작은 값 검색","Búsqueda de menor que","Recherche inférieur à","Suche kleiner als","Tìm giá trị nhỏ hơn","ค้นหาค่าที่น้อยกว่า","छोटा मान खोज","بحث عن أصغر من"),
"Auto Load/Save Cheats": ("自动载入/保存金手指","自動載入/儲存金手指","チートを自動読込/保存","치트 자동 불러오기/저장","Cargar/guardar trucos auto.","Charger/enregistrer codes auto.","Cheats auto. laden/speichern","Tự động tải/lưu cheat","โหลด/บันทึกโกงอัตโนมัติ","चीट स्वतः लोड/सहेजें","تحميل/حفظ أكواد الغش تلقائيًا"),
"Pause While Active": ("激活时暂停","啟用時暫停","アクティブ時に一時停止","활성 시 일시 중지","Pausar mientras está activo","Pause si actif","Pausieren wenn aktiv","Tạm dừng khi đang mở","หยุดชั่วคราวเมื่อเปิดใช้","सक्रिय रहने पर रोकें","إيقاف مؤقت أثناء النشاط"),
"Hotkey Configuration": ("热键配置","熱鍵設定","ホットキー設定","단축키 설정","Configuración de teclas rápidas","Configuration des raccourcis","Hotkey-Konfiguration","Cấu hình phím nóng","ตั้งค่าฮอตคีย์","हॉटकी कॉन्फ़िगरेशन","إعدادات مفاتيح الاختصار"),
"Set Hot Key": ("设置热键","設定熱鍵","ホットキーを設定","단축키 지정","Establecer tecla rápida","Définir le raccourci","Hotkey festlegen","Đặt phím nóng","ตั้งฮอตคีย์","हॉटकी सेट करें","تعيين مفتاح الاختصار"),
"Hotkey Select": ("热键选择","熱鍵選擇","ホットキー選択","단축키 선택","Seleccionar tecla rápida","Sélection du raccourci","Hotkey auswählen","Chọn phím nóng","เลือกฮอตคีย์","हॉटकी चुनें","اختيار مفتاح الاختصار"),
"Input Configuration": ("输入配置","輸入設定","入力設定","입력 설정","Configuración de entrada","Configuration des entrées","Eingabekonfiguration","Cấu hình đầu vào","ตั้งค่าอินพุต","इनपुट कॉन्फ़िगरेशन","إعدادات الإدخال"),
"OK": ("确定","確定","OK","확인","Aceptar","OK","OK","OK","ตกลง","ठीक","موافق"),
"Reset Defaults": ("恢复默认设置","還原預設值","デフォルトに戻す","기본값으로 재설정","Restablecer valores","Réinitialiser","Standardwerte wiederherstellen","Khôi phục mặc định","คืนค่าเริ่มต้น","डिफ़ॉल्ट रीसेट करें","استعادة الإعدادات الافتراضية"),
"Video": ("视频","視訊","ビデオ","비디오","Vídeo","Vidéo","Video","Video","วิดีโอ","वीडियो","الفيديو"),
"Audio": ("音频","音訊","オーディオ","오디오","Audio","Audio","Audio","Âm thanh","เสียง","ऑडियो","الصوت"),
"The Lua script running has been running a long time. It may have gone crazy. Kill it? (I won't ask again if you say No)": (
 "Lua 脚本已运行很长时间，\n可能已失去控制。要终止它吗？（若选择“否”，以后将不再询问）\n",
 "Lua 腳本已執行很長時間，\n可能已失去控制。要終止它嗎？（若選擇「否」，以後將不再詢問）\n",
 "Lua スクリプトが長時間実行されています。\n暴走している可能性があります。終了しますか？（「いいえ」を選ぶと今後は確認しません）\n",
 "Lua 스크립트가 오래 실행 중입니다.\n제어를 벗어났을 수 있습니다. 종료하시겠습니까? (아니요를 선택하면 다시 묻지 않습니다)\n",
 "El script Lua lleva mucho tiempo ejecutándose.\nPuede que se haya descontrolado. ¿Terminarlo? (Si dice No, no se volverá a preguntar)\n",
 "Le script Lua tourne depuis longtemps.\nIl est peut-être parti en boucle. Le terminer ? (Si vous répondez Non, on ne vous le redemandera plus)\n",
 "Das Lua-Skript läuft schon sehr lange.\nEs könnte fehlgeschlagen sein. Beenden? (Bei „Nein“ wird nicht erneut gefragt)\n",
 "Script Lua đã chạy rất lâu.\nCó thể đã bị lỗi vòng lặp. Kết thúc nó? (Nếu chọn Không, sẽ không hỏi lại nữa)\n",
 "สคริปต์ Lua ทำงานมานานแล้ว\nอาจทำงานผิดปกติ ต้องการหยุดมันไหม? (ถ้าตอบ ไม่ จะไม่ถามอีก)\n",
 "Lua स्क्रिप्ट काफी समय से चल रही है।\nहो सकता है वह अनियंत्रित हो गई हो। इसे रोकें? (यदि आप नहीं कहते हैं तो फिर नहीं पूछा जाएगा)\n",
 "يعمل برنامج Lua النصي منذ وقت طويل.\nربما خرج عن السيطرة. هل تريد إيقافه؟ (إذا اخترت لا فلن يُسأل مجددًا)\n"),
"Movie Options": ("录像选项","錄影選項","ムービーオプション","묵비 옵션","Opciones de película","Options de film","Filmoptionen","Tùy chọn phim","ตัวเลือกมูฟวี่","मूवी विकल्प","خيارات التسجيل"),
"Movie Play": ("播放录像","播放錄影","ムービー再生","묵비 재생","Reproducir película","Lire le film","Film abspielen","Phát phim","เล่นมูฟวี่","मूवी चलाएँ","تشغيل التسجيل"),

"Message Log Viewer": ("消息日志查看器","訊息記錄檢視器","メッセージログビューア","메시지 로그 뷰어","Visor de registro de mensajes","Visionneuse du journal","Meldungsprotokoll-Anzeige","Trình xem nhật ký thông báo","ตัวดูบันทึกข้อความ","संदेश लॉग व्यूअर","عارض سجل الرسائل"),
"Palette Editor": ("调色板编辑器","調色盤編輯器","パレットエディタ","팔레트 편집기","Editor de paleta","Éditeur de palette","Paletten-Editor","Trình chỉnh bảng màu","ตัวแก้ไขพาเลต","पैलेट संपादक","محرر لوحة الألوان"),
"RAM Search": ("RAM 搜索","RAM 搜尋","RAM 検索","RAM 검색","Búsqueda RAM","Recherche RAM","RAM-Suche","Tìm kiếm RAM","ค้นหา RAM","RAM खोज","بحث RAM"),
"RAM Watch": ("RAM 监视","RAM 監視","RAM ウォッチ","RAM 감시","Vigilancia RAM","Surveillance RAM","RAM-Überwachung","Theo dõi RAM","เฝ้าดู RAM","RAM निगरानी","مراقبة RAM"),
"State Recorder Config": ("状态录制配置","狀態錄製設定","ステートレコーダー設定","상태 레코더 설정","Config. de grabador de estados","Config. de l'enregistreur d'états","State-Recorder-Konfiguration","Cấu hình ghi trạng thái","ตั้งค่าตัวบันทึกสถานะ","स्टेट रिकॉर्डर कॉन्फ़िग","إعدادات مسجل الحالة"),
"Is Array": ("是数组","是陣列","配列である","배열임","Es array","Est un tableau","Ist Array","Là mảng","เป็นอาร์เรย์","ऐरे है","هو مصفوفة"),
"Overwrite Name": ("覆盖名称","覆蓋名稱","名前を上書き","이름 덮어쓰기","Sobrescribir nombre","Écraser le nom","Name überschreiben","Ghi đè tên","เขียนทับชื่อ","नाम अधिलेखित करें","استبدال الاسم"),
"Overwrite Comment": ("覆盖注释","覆蓋註解","コメントを上書き","주석 덮어쓰기","Sobrescribir comentario","Écraser le commentaire","Kommentar überschreiben","Ghi đè chú thích","เขียนทับความคิดเห็น","टिप्पणी अधिलेखित करें","استبدال التعليق"),
"Comment Head Only": ("仅注释头部","僅註解開頭","コメント先頭のみ","주석 머리만","Solo cabecera del comentario","Début du commentaire seul","Nur Kommentarkopf","Chỉ đầu chú thích","เฉพาะส่วนหัวความคิดเห็น","केवल टिप्पणी शीर्ष","رأس التعليق فقط"),
"Timing Configuration": ("时序配置","時序設定","タイミング設定","타이밍 설정","Configuración de tiempo","Configuration du timing","Timing-Konfiguration","Cấu hình thời gian","ตั้งค่าไทม์มิง","टाइमिंग कॉन्फ़िगरेशन","إعدادات التوقيت"),
"Auto Update": ("自动更新","自動更新","自動更新","자동 업데이트","Actualización automática","Mise à jour auto.","Automatische Aktualisierung","Tự động cập nhật","อัปเดตอัตโนมัติ","स्वतः अपडेट","تحديث تلقائي"),
"Log Registers": ("记录寄存器","記錄暫存器","レジスタを記録","레지스터 기록","Registrar registros","Journaliser les registres","Register protokollieren","Ghi thanh ghi","บันทึกรีจิสเตอร์","रजिस्टर लॉग करें","تسجيل السجلات"),
"Log Frame Counter": ("记录帧计数器","記錄幀計數器","フレームカウンタを記録","프레임 카운터 기록","Registrar contador de fotogramas","Journaliser le compteur d'images","Frame-Zähler protokollieren","Ghi bộ đếm khung","บันทึกตัวนับเฟรม","फ्रेम काउंटर लॉग करें","تسجيل عداد الإطارات"),
"Log Emu Messages": ("记录模拟器消息","記錄模擬器訊息","エミュレータメッセージを記録","에뮬레이터 메시지 기록","Registrar mensajes del emulador","Journaliser les messages de l'émulateur","Emulator-Meldungen protokollieren","Ghi thông báo giả lập","บันทึกข้อความอีมู","एमुलेटर संदेश लॉग करें","تسجيل رسائل المحاكي"),
"Log Status Flags": ("记录状态标志","記錄狀態旗標","ステータスフラグを記録","상태 플래그 기록","Registrar flags de estado","Journaliser les flags d'état","Status-Flags protokollieren","Ghi cờ trạng thái","บันทึกแฟลกสถานะ","स्टेटस फ़्लैग लॉग करें","تسجيل أعلام الحالة"),
"Log Cycle Count": ("记录周期计数","記錄週期計數","サイクル数を記録","사이클 수 기록","Registrar recuento de ciclos","Journaliser le nombre de cycles","Zykluszähler protokollieren","Ghi số chu kỳ","บันทึกจำนวนรอบ","साइकिल संख्या लॉग करें","تسجيل عدد الدورات"),
"Use Stack Pointer": ("使用堆栈指针","使用堆疊指標","スタックポインタを使用","스택 포인터 사용","Usar puntero de pila","Utiliser le pointeur de pile","Stack-Pointer verwenden","Dùng con trỏ ngăn xếp","ใช้สแตกพอยเตอร์","स्टैक पॉइंटर उपयोग करें","استخدام مؤشر المكدس"),
"Disasm Left": ("左侧反汇编","左側反組譯","左側を逆アセンブル","왼쪽 디스어셈블","Desensamblado izquierdo","Désassemblage à gauche","Disassembly links","Dịch ngược bên trái","ดิสแอสเซมบลีซ้าย","बाईं ओर डिसअसेंबली","تفكيك يسار"),
"Log Instruction Count": ("记录指令计数","記錄指令計數","命令数を記録","명령어 수 기록","Registrar recuento de instrucciones","Journaliser le nombre d'instructions","Befehlszähler protokollieren","Ghi số lệnh","บันทึกจำนวนคำสั่ง","निर्देश संख्या लॉग करें","تسجيل عدد التعليمات"),
"Log New Mapped Code": ("记录新映射的代码","記錄新映射的程式碼","新規マップコードを記録","새로 매핑된 코드 기록","Registrar código mapeado nuevo","Journaliser le nouveau code mappé","Neuen gemappten Code protokollieren","Ghi mã mới được ánh xạ","บันทึกโค้ดที่แมปใหม่","नया मैप्ड कोड लॉग करें","تسجيل الشيفرة المعيّنة الجديدة"),
"Log New Mapped Data": ("记录新映射的数据","記錄新映射的資料","新規マップデータを記録","새로 매핑된 데이터 기록","Registrar datos mapeados nuevos","Journaliser les nouvelles données mappées","Neue gemappte Daten protokollieren","Ghi dữ liệu mới được ánh xạ","บันทึกข้อมูลที่แมปใหม่","नया मैप्ड डेटा लॉग करें","تسجيل البيانات المعيّنة الجديدة"),
"Clear": ("清除","清除","クリア","지우기","Limpiar","Effacer","Löschen","Xóa","ล้าง","साफ़ करें","مسح"),
" = ": (" = ",)*11,
"&NTSC": ("&NTSC",)*11,
"&PAL": ("&PAL",)*11,
"&Dendy": ("&Dendy",)*11,
"&Default": ("默认(&D)","預設(&D)","デフォルト(&D)","기본값(&D)","&Predeterminado","Par &défaut","&Standard","Mặc &định","ค่าเริ่มต้น(&D)","डिफ़ॉल्ट (&D)","الافتراضي (&D)"),
"Fill $&FF": ("填充 $FF(&F)","填充 $FF(&F)","$FF で埋める(&F)","$FF로 채우기(&F)","Rellenar $&FF","Remplir $&FF","$&FF füllen","Điền $&FF","เติม $FF(&F)","$FF भरें (&F)","تعبئة $FF (&F)"),
"Fill $&00": ("填充 $00(&0)","填充 $00(&0)","$00 で埋める(&0)","$00로 채우기(&0)","Rellenar $&00","Remplir $&00","$&00 füllen","Điền $&00","เติม $00(&0)","$00 भरें (&0)","تعبئة $00 (&0)"),
"&Random": ("随机(&R)","隨機(&R)","ランダム(&R)","무작위(&R)","&Aleatorio","A&léatoire","&Zufällig","&Ngẫu nhiên","สุ่ม(&R)","यादृच्छिक (&R)","عشوائي (&R)"),
"Slot &%1": ("存档槽 %1(&%1)","存檔槽 %1(&%1)","スロット %1(&%1)","슬롯 %1(&%1)","Ranura &%1","Emplacement &%1","Slot &%1","Khe &%1","ช่องเซฟ %1(&%1)","स्लॉट %1 (&%1)","خانة %1 (&%1)"),
"Japanese": ("日语","日語","日本語","일본어","Japonés","Japonais","Japanisch","Tiếng Nhật","ญี่ปุ่น","जापानी","اليابانية"),
"Korean": ("韩语","韓語","韓国語","한국어","Coreano","Coréen","Koreanisch","Tiếng Hàn","เกาหลี","कोरियाई","الكورية"),
"Spanish": ("西班牙语","西班牙語","スペイン語","스페인어","Español","Espagnol","Spanisch","Tiếng Tây Ban Nha","สเปน","स्पेनी","الإسبانية"),
"French": ("法语","法語","フランス語","프랑스어","Francés","Français","Französisch","Tiếng Pháp","ฝรั่งเศส","फ्रेंच","الفرنسية"),
"German": ("德语","德語","ドイツ語","독일어","Alemán","Allemand","Deutsch","Tiếng Đức","เยอรมัน","जर्मन","الألمانية"),
"Vietnamese": ("越南语","越南語","ベトナム語","베트남어","Vietnamita","Vietnamien","Vietnamesisch","Tiếng Việt","เวียดนาม","वियतनामी","الفيتنامية"),
"Thai": ("泰语","泰語","タイ語","태국어","Tailandés","Thaï","Thailändisch","Tiếng Thái","ไทย","थाई","التايلاندية"),
"Hindi (beta)": ("印地语（beta）","印地語（beta）","ヒンディー語 (beta)","힌디어 (beta)","Hindi (beta)","Hindi (bêta)","Hindi (Beta)","Hindi (beta)","ฮินดี (beta)","हिंदी (बीटा)","الهندية (بيتا)"),
"Arabic (beta)": ("阿拉伯语（beta）","阿拉伯語（beta）","アラビア語 (beta)","아랍어 (beta)","Árabe (beta)","Arabe (bêta)","Arabisch (Beta)","Tiếng Ả Rập (beta)","อาหรับ (beta)","अरबी (बीटा)","العربية (بيتا)"),
"&%1x": ("&%1x",)*11,
"%1 On, %2 Off": ("%1 开，%2 关","%1 開，%2 關","%1 オン、%2 オフ","%1 켜짐, %2 꺼짐","%1 act., %2 des.","%1 marche, %2 arrêt","%1 an, %2 aus","%1 bật, %2 tắt","%1 เปิด, %2 ปิด","%1 चालू, %2 बंद","%1 تشغيل، %2 إيقاف"),
"&Documentation": ("文档(&D)","文件(&D)","ドキュメント(&D)","문서(&D)","&Documentación","&Documentation","&Dokumentation","&Tài liệu","เอกสาร(&D)","दस्तावेज़ (&D)","الوثائق (&D)"),
"&Online": ("在线(&O)","線上(&O)","オンライン(&O)","온라인(&O)","En &línea","En &ligne","&Online","&Trực tuyến","ออนไลน์(&O)","ऑनलाइन (&O)","عبر الإنترنت (&O)"),
"Open online documentation in browser": ("在浏览器中打开在线文档","在瀏覽器中開啟線上文件","ブラウザでオンラインドキュメントを開く","브라우저에서 온라인 문서 열기","Abrir documentación en línea en el navegador","Ouvrir la documentation en ligne dans le navigateur","Online-Dokumentation im Browser öffnen","Mở tài liệu trực tuyến trong trình duyệt","เปิดเอกสารออนไลน์ในเบราว์เซอร์","ब्राउज़र में ऑनलाइन दस्तावेज़ खोलें","فتح الوثائق عبر الإنترنت في المتصفح"),
"&Local": ("本地(&L)","本機(&L)","ローカル(&L)","로컬(&L)","&Local","&Local","&Lokal","&Cục bộ","ภายในเครื่อง(&L)","स्थानीय (&L)","محلي (&L)"),
"Open bundled offline documentation": ("打开内置离线文档","開啟內建離線文件","同梱のオフライン文書を開く","번들 오프라인 문서 열기","Abrir documentación sin conexión incluida","Ouvrir la documentation hors ligne fournie","Mitgelieferte Offline-Dokumentation öffnen","Mở tài liệu ngoại tuyến đính kèm","เปิดเอกสารออฟไลน์ที่แนบมา","बंडल किया ऑफ़लाइन दस्तावेज़ खोलें","فتح الوثائق المضمّنة دون اتصال"),
"&Clear Recent ROM List": ("清除最近 ROM 列表(&C)","清除最近 ROM 清單(&C)","最近の ROM リストをクリア(&C)","최근 ROM 목록 지우기(&C)","&Borrar lista de ROM recientes","Effa&cer la liste des ROM récentes","&Liste der letzten ROMs löschen","&Xóa danh sách ROM gần đây","ล้างรายการ ROM ล่าสุด(&C)","हालिया ROM सूची साफ़ करें (&C)","مسح قائمة ROM الأخيرة (&C)"),
"Pick Palette Color": ("选择调色板颜色","選擇調色盤顏色","パレットカラーを選択","팔레트 색상 선택","Elegir color de paleta","Choisir la couleur de la palette","Palettenfarbe wählen","Chọn màu bảng màu","เลือกสีพาเลต","पैलेट रंग चुनें","اختيار لون اللوحة"),
"NES Header Editor": ("NES 头编辑器","NES 標頭編輯器","NES ヘッダエディタ","NES 헤더 편집기","Editor de cabecera NES","Éditeur d'en-tête NES","NES-Header-Editor","Trình chỉnh header NES","ตัวแก้ไขเฮดเดอร์ NES","NES हेडर संपादक","محرر ترويسة NES"),
"Save": ("保存","儲存","保存","저장","Guardar","Enregistrer","Speichern","Lưu","บันทึก","सहेजें","حفظ"),
"Load": ("载入","載入","読込","불러오기","Cargar","Charger","Laden","Tải","โหลด","लोड करें","تحميل"),
"Record": ("录制","錄製","録画","녹화","Grabar","Enregistrer","Aufnehmen","Ghi","บันทึกวิดีโอ","रिकॉर्ड","تسجيل"),
}

def unesc(s):
    return s.replace("&amp;","&").replace("&apos;","'").replace("&quot;",'"').replace("&lt;","<").replace("&gt;",">")
def esc(s):
    return s.replace("&","&amp;").replace("<","&lt;").replace(">","&gt;")
def norm(s):
    return re.sub(r"\s+"," ",s).strip()

Tnorm = {norm(k): v for k, v in T.items()}
missing = set(Tnorm)

msg_re = re.compile(r"<message>.*?</message>", re.S)
src_re = re.compile(r"<source>(.*?)</source>", re.S)
tr_unf_re = re.compile(r"<translation type=\"unfinished\"\s*(/>|>\s*</translation>)", re.S)

for i, lang in enumerate(LANGS):
    fn = f"fceux11_{lang}.ts"
    xml = open(fn, encoding="utf-8").read()
    filled = 0
    def repl(m):
        block = m.group(0)
        if 'type="unfinished"' not in block:
            return block
        sm = src_re.search(block)
        if not sm:
            return block
        key = norm(unesc(sm.group(1)))
        tr = Tnorm.get(key)
        if tr is None:
            return block
        newtr = "<translation>" + esc(tr[i]) + "</translation>"
        return tr_unf_re.sub(lambda _: newtr, block, count=1)
    # count fills
    def repl_count(m):
        global filled
        out = repl(m)
        if out != m.group(0):
            filled += 1
        return out
    xml2 = msg_re.sub(repl_count, xml)
    if xml2 != xml:
        open(fn, "w", encoding="utf-8", newline="").write(xml2)
    remaining = xml2.count('type="unfinished"')
    print(f"{lang}: filled={filled} remaining_unfinished={remaining}")

# report dict entries never matched (typos guard)
xml = open("fceux11_zh_CN.ts", encoding="utf-8").read()
used = set()
for m in msg_re.finditer(xml):
    sm = src_re.search(m.group(0))
    if sm:
        used.add(norm(unesc(sm.group(1))))
unused = [k for k in Tnorm if k not in used]
print("UNUSED_DICT_KEYS:", unused if unused else "none")
