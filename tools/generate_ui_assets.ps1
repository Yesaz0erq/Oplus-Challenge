$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Drawing

$outDir = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\assets\ui"))
if (-not (Test-Path $outDir)) {
    New-Item -ItemType Directory -Path $outDir | Out-Null
}

function New-Color([int]$a, [int]$r, [int]$g, [int]$b) {
    [System.Drawing.Color]::FromArgb($a, $r, $g, $b)
}

function Mix-Color($a, $b, [double]$t) {
    $clamped = [Math]::Max(0.0, [Math]::Min(1.0, $t))
    New-Color `
        ([int]($a.A + (($b.A - $a.A) * $clamped))) `
        ([int]($a.R + (($b.R - $a.R) * $clamped))) `
        ([int]($a.G + (($b.G - $a.G) * $clamped))) `
        ([int]($a.B + (($b.B - $a.B) * $clamped)))
}

function New-Canvas([int]$width, [int]$height) {
    $bmp = New-Object System.Drawing.Bitmap $width, $height, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $gfx = [System.Drawing.Graphics]::FromImage($bmp)
    $gfx.Clear([System.Drawing.Color]::Transparent)
    $gfx.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
    $gfx.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::NearestNeighbor
    $gfx.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::Half
    $gfx.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::None
    @{
        Bitmap = $bmp
        Graphics = $gfx
    }
}

function Save-Canvas($canvas, [string]$name) {
    $path = Join-Path $outDir $name
    try {
        $canvas.Bitmap.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    } finally {
        $canvas.Graphics.Dispose()
        $canvas.Bitmap.Dispose()
    }
}

function Fill-Rect($gfx, $color, [int]$x, [int]$y, [int]$w, [int]$h) {
    $brush = New-Object System.Drawing.SolidBrush $color
    try {
        $gfx.FillRectangle($brush, $x, $y, $w, $h)
    } finally {
        $brush.Dispose()
    }
}

function Set-PixelSafe($bmp, [int]$x, [int]$y, $color) {
    if ($x -lt 0 -or $y -lt 0 -or $x -ge $bmp.Width -or $y -ge $bmp.Height) {
        return
    }
    $bmp.SetPixel($x, $y, $color)
}

function Draw-Star($bmp, [int]$cx, [int]$cy, [int]$size, $core, $glow) {
    for ($dy = -$size; $dy -le $size; $dy++) {
        Set-PixelSafe $bmp $cx ($cy + $dy) $glow
    }
    for ($dx = -$size; $dx -le $size; $dx++) {
        Set-PixelSafe $bmp ($cx + $dx) $cy $glow
    }
    Set-PixelSafe $bmp $cx $cy $core
    Set-PixelSafe $bmp ($cx - 1) ($cy - 1) $glow
    Set-PixelSafe $bmp ($cx + 1) ($cy - 1) $glow
    Set-PixelSafe $bmp ($cx - 1) ($cy + 1) $glow
    Set-PixelSafe $bmp ($cx + 1) ($cy + 1) $glow
}

function Add-Starfield($bmp, [int]$marginX, [int]$marginY, [int]$count, [int]$seed) {
    $coreA = New-Color 255 246 247 255
    $coreB = New-Color 255 202 216 255
    $glowA = New-Color 160 163 194 255
    $glowB = New-Color 150 222 198 255

    for ($i = 0; $i -lt $count; $i++) {
        $x = $marginX + (($i * 29 + $seed * 7) % [Math]::Max(1, ($bmp.Width - ($marginX * 2))))
        $y = $marginY + (($i * 17 + $seed * 11) % [Math]::Max(1, ($bmp.Height - ($marginY * 2))))
        $size = if ((($i + $seed) % 5) -eq 0) { 2 } else { 1 }
        $core = if ((($i + $seed) % 3) -eq 0) { $coreB } else { $coreA }
        $glow = if ((($i + $seed) % 4) -eq 0) { $glowB } else { $glowA }
        Draw-Star $bmp $x $y $size $core $glow
    }
}

function Fill-VerticalGradient($bmp, [int]$x, [int]$y, [int]$w, [int]$h, $top, $bottom) {
    for ($iy = 0; $iy -lt $h; $iy++) {
        $t = if ($h -le 1) { 0.0 } else { $iy / ($h - 1.0) }
        $color = Mix-Color $top $bottom $t
        for ($ix = 0; $ix -lt $w; $ix++) {
            Set-PixelSafe $bmp ($x + $ix) ($y + $iy) $color
        }
    }
}

function Add-DustBand($bmp, [int]$x, [int]$y, [int]$w, [int]$h) {
    $top = New-Color 210 108 98 132
    $bottom = New-Color 225 84 71 92
    Fill-VerticalGradient $bmp $x $y $w $h $top $bottom

    for ($ix = 0; $ix -lt $w; $ix++) {
        $ridge = (($ix * 7) % 5)
        Set-PixelSafe $bmp ($x + $ix) ($y + $ridge) (New-Color 230 182 170 194)
        Set-PixelSafe $bmp ($x + $ix) ($y + $h - 1) (New-Color 220 56 47 67)
    }
}

function Draw-MetalFrame($bmp, $gfx, [int]$x, [int]$y, [int]$w, [int]$h) {
    $shadow = New-Color 255 30 34 58
    $outline = New-Color 255 62 73 110
    $body = New-Color 255 120 133 172
    $highlight = New-Color 255 210 221 244
    $innerShadow = New-Color 255 55 61 91

    Fill-Rect $gfx $shadow $x $y $w $h
    Fill-Rect $gfx $outline ($x + 1) ($y + 1) ($w - 2) ($h - 2)
    Fill-Rect $gfx $body ($x + 2) ($y + 2) ($w - 4) ($h - 4)

    for ($ix = $x + 3; $ix -lt ($x + $w - 3); $ix++) {
        Set-PixelSafe $bmp $ix ($y + 3) $highlight
        Set-PixelSafe $bmp $ix ($y + $h - 4) $innerShadow
    }
    for ($iy = $y + 3; $iy -lt ($y + $h - 3); $iy++) {
        Set-PixelSafe $bmp ($x + 3) $iy $highlight
        Set-PixelSafe $bmp ($x + $w - 4) $iy $innerShadow
    }
}

function New-PanelTexture([string]$name, [int]$width, [int]$height) {
    $canvas = New-Canvas $width $height
    $bmp = $canvas.Bitmap
    $gfx = $canvas.Graphics

    Draw-MetalFrame $bmp $gfx 0 0 $width $height
    Fill-VerticalGradient `
        $bmp `
        5 `
        5 `
        ($width - 10) `
        ($height - 10) `
        (New-Color 225 13 18 52) `
        (New-Color 225 42 46 82)

    Fill-VerticalGradient `
        $bmp `
        8 `
        8 `
        ($width - 16) `
        ($height - 16) `
        (New-Color 205 10 16 44) `
        (New-Color 205 56 61 96)

    Add-Starfield $bmp 12 12 18 3
    Add-DustBand $bmp 8 ($height - 28) ($width - 16) 20

    Save-Canvas $canvas $name
}

function New-ButtonTexture([string]$name, [int]$width, [int]$height, [bool]$compact) {
    $canvas = New-Canvas $width $height
    $bmp = $canvas.Bitmap
    $gfx = $canvas.Graphics

    Draw-MetalFrame $bmp $gfx 0 0 $width $height
    Fill-VerticalGradient `
        $bmp `
        5 `
        5 `
        ($width - 10) `
        ($height - 10) `
        (New-Color 255 36 46 92) `
        (New-Color 255 79 88 136)

    if ($compact) {
        Add-Starfield $bmp 10 8 4 5
    } else {
        Add-Starfield $bmp 12 8 8 7
    }

    $bandColor = New-Color 220 173 184 214
    for ($ix = 10; $ix -lt ($width - 10); $ix++) {
        Set-PixelSafe $bmp $ix ([int]($height * 0.42)) $bandColor
    }

    Save-Canvas $canvas $name
}

function New-SlotTexture([string]$name, [int]$width, [int]$height, [bool]$hud) {
    $canvas = New-Canvas $width $height
    $bmp = $canvas.Bitmap
    $gfx = $canvas.Graphics

    Draw-MetalFrame $bmp $gfx 0 0 $width $height
    Fill-VerticalGradient `
        $bmp `
        5 `
        5 `
        ($width - 10) `
        ($height - 10) `
        (New-Color 210 20 28 60) `
        (New-Color 220 61 69 104)

    if ($hud) {
        Fill-VerticalGradient `
            $bmp `
            8 `
            8 `
            ($width - 16) `
            16 `
            (New-Color 235 85 97 136) `
            (New-Color 230 53 60 93)

        $crest = New-Color 230 200 212 240
        for ($ix = 12; $ix -lt ($width - 12); $ix++) {
            Set-PixelSafe $bmp $ix 12 $crest
            Set-PixelSafe $bmp $ix ($height - 18) (New-Color 215 160 147 176)
        }

        Add-Starfield $bmp 10 28 7 11
        Add-DustBand $bmp 8 ($height - 18) ($width - 16) 10
    } else {
        Add-Starfield $bmp 9 9 8 13
        Add-DustBand $bmp 6 ($height - 16) ($width - 12) 10
    }

    Save-Canvas $canvas $name
}

New-PanelTexture "panel_window.png" 128 128
New-ButtonTexture "button_large.png" 128 32 $false
New-ButtonTexture "button_small.png" 96 24 $true
New-SlotTexture "slot_frame.png" 48 48 $false
New-SlotTexture "hud_slot.png" 72 104 $true
