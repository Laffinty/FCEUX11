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
# A more rigorous check would consult Unihan; this is a heuristic
# covering the most common mixed-script terms.
$TraditionalOnly = @(
    '檔', '軟', '體', '網', '訊', '時', '當', '機', '電', '腦',
    '頭', '麵', '麥', '齒', '輪', '蘭', '關', '閉', '開', '聲',
    '畫', '個', '們', '東', '車', '馬', '雞', '鴨', '貓', '豬',
    '備', '綠', '邊', '長', '間', '員', '園', '節', '練', '習',
    '讀', '寫', '語', '說', '請', '謝', '對', '錯', '離', '給',
    '從', '過', '達', '運', '選', '轉', '變', '應', '該', '處',
    '創', '設', '決', '點', '組', '維', '護', '聯', '繫', '結',
    '終', '繼', '續', '測', '試', '驗', '證', '審', '確', '認',
    '類', '別', '條', '項', '標', '籤', '顯', '示', '單', '雙',
    '點', '擊', '滾', '動', '欄', '目', '塊', '版', '權', '聲',
    '輸', '出', '入', '編', '譯', '執', '行', '處', '理', '法',
    '參', '與', '據', '庫', '緩', '存', '登', '錄', '註', '銷',
    '顯', '示', '隱', '藏', '設', '定', '調', '整', '優', '化',
    '軟', '鍵', '滑', '鼠', '觸', '碰', '螢', '幕', '鏡', '頭',
    '插', '件', '擴', '展', '通', '訊', '協', '定', '協', '議',
    '執', '照', '版', '權', '糾', '紛', '賠', '償', '違', '約'
)

$SimplifiedOnly = @(
    '软', '体', '网', '讯', '时', '当', '机', '电', '脑',
    '头', '面', '麦', '齿', '轮', '兰', '关', '闭', '开', '声',
    '画', '个', '们', '东', '车', '马', '鸡', '鸭', '猫', '猪',
    '备', '绿', '边', '长', '间', '员', '园', '节', '练', '习',
    '读', '写', '语', '说', '请', '谢', '对', '错', '离', '给',
    '从', '过', '达', '运', '选', '转', '变', '应', '该', '处',
    '创', '设', '决', '组', '维', '护', '联', '系', '结',
    '终', '继', '续', '测', '试', '验', '证', '审', '确', '认',
    '类', '别', '条', '项', '标', '签', '显', '示', '单', '双',
    '击', '滚', '动', '栏', '目', '块', '版', '权', '声',
    '输', '出', '入', '编', '译', '执', '行', '理', '法',
    '参', '与', '据', '库', '缓', '存', '登', '录', '注', '销',
    '隐', '藏', '设', '定', '调', '整', '优', '化',
    '键', '鼠', '触', '碰', '屏', '幕', '镜', '头',
    '插', '件', '扩', '展', '通', '协', '议',
    '执', '照', '纠', '纷', '赔', '偿', '违', '约'
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
        Write-Host "       $($hits -join ', ')"
        return $false
    } else {
        Write-Host "[PASS] $label: no forbidden glyphs found."
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
