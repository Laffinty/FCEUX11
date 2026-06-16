# check_simp_trad.ps1 - v0.3.15 PR-C
# Validate that the Simplified Chinese .ts does not contain traditional
# Chinese glyphs and vice versa. This guards against accidental term
# bleed between the two translation trees.
#
# Usage:
#   powershell scripts/check_simp_trad.ps1

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$ProjectRoot = Split-Path -Parent $ScriptDir
$LangDir = Join-Path $ProjectRoot "src\drivers\Qt\lang"

# Traditional-only / Simplified-only representative glyphs.
# v0.3.15 PHASE-1 fix: rebuilt TraditionalOnly/SimplifiedOnly lists
# to remove chars that appear in BOTH lists (i.e., shared between
# scripts and thus NOT distinguishing). Previously the script
# flagged shared chars like 件/存/入/出 as "traditional", which
# gave false positives on legitimate simplified text.
#
# True distinguishing character pairs covered (sample):
#   軟/软, 體/体, 網/网, 訊/讯, 時/时, 當/当, 機/机, 電/电, 腦/脑
#   頭/头, 麵/面, 麥/麦, 齒/齿, 輪/轮, 蘭/兰, 關/关, 閉/闭, 開/开
#   ... (full list is 124 trad-only, 121 simp-only chars)

# A more rigorous check would consult Unihan; this is a heuristic
# covering the most common mixed-script terms.
$TraditionalOnly = (
    '個', '們', '備', '償', '優', '別', '創', '動', '協', '參',
    '員', '單', '園', '執', '塊', '審', '寫', '對', '庫', '從',
    '應', '擊', '據', '擴', '時', '東', '條', '標', '機', '檔',
    '欄', '權', '決', '測', '滑', '滾', '畫', '當', '確', '節',
    '籤', '糾', '約', '紛', '終', '組', '結', '給', '綠', '維',
    '網', '編', '緩', '練', '繫', '繼', '續', '習', '聯', '聲',
    '腦', '與', '蘭', '處', '螢', '觸', '訊', '設', '註', '試',
    '該', '認', '語', '說', '調', '請', '謝', '證', '譯', '議',
    '護', '讀', '變', '豬', '貓', '賠', '車', '軟', '輪', '輸',
    '轉', '運', '過', '達', '違', '選', '邊', '銷', '錄', '錯',
    '鍵', '鏡', '長', '閉', '開', '間', '關', '隱', '雙', '雞',
    '離', '電', '項', '頭', '類', '顯', '馬', '驗', '體', '鴨',
    '麥', '麵', '點', '齒'
)

$SimplifiedOnly = (
    '与', '东', '个', '习', '从', '们', '优', '体', '偿', '兰',
    '关', '写', '决', '击', '创', '别', '动', '协', '单', '参',
    '双', '变', '员', '园', '块', '声', '处', '备', '头', '审',
    '对', '屏', '库', '应', '开', '当', '录', '执', '扩', '护',
    '据', '时', '显', '机', '权', '条', '标', '栏', '注', '测',
    '滚', '猪', '猫', '电', '画', '确', '离', '签', '类', '系',
    '纠', '约', '纷', '练', '组', '终', '结', '给', '继', '续',
    '维', '绿', '缓', '编', '网', '联', '脑', '节', '触', '认',
    '议', '讯', '设', '证', '译', '试', '该', '语', '说', '请',
    '读', '调', '谢', '赔', '车', '转', '轮', '软', '输', '边',
    '达', '过', '运', '违', '选', '销', '错', '键', '镜', '长',
    '闭', '间', '隐', '面', '项', '马', '验', '鸡', '鸭', '麦',
    '齿'
)

$ExitCode = 0

function Test-FileForGlyphs($path, $forbiddenGlyphs, $label) {
    if (-not (Test-Path $path)) {
        Write-Warning "$path not found."
        return
    }
    $content = Get-Content $path -Raw -Encoding UTF8
    $hits = @()
    foreach ($g in $forbiddenGlyphs) {
        if ($content.Contains($g)) {
            $hits += $g
        }
    }
    if ($hits.Count -gt 0) {
        Write-Host "[FAIL] $label contains $($hits.Count) forbidden glyphs:"
        Write-Host ("       {0}" -f ($hits -join ', '))
        return $false
    } else {
        # Use ${label}: form to escape PowerShell's scope operator parsing
        Write-Host ("[PASS] {0}: no forbidden glyphs found." -f $label)
        return $true
    }
}

$zhCN = Join-Path $LangDir "fceux11_zh_CN.ts"
$zhTW = Join-Path $LangDir "fceux11_zh_TW.ts"

# zh_CN should NOT contain traditional glyphs
$ok1 = Test-FileForGlyphs $zhCN $TraditionalOnly "fceux11_zh_CN.ts (no traditional)"

# zh_TW should NOT contain simplified glyphs
$ok2 = Test-FileForGlyphs $zhTW $SimplifiedOnly "fceux11_zh_TW.ts (no simplified)"

if (-not $ok1 -or -not $ok2) { $ExitCode = 1 }

exit $ExitCode
