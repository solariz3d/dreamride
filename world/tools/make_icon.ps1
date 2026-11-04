# Draws Dreamride's icon (world\assets\dreamride.ico): a glowing line rising through the dark to a
# bright draw-head, mirrored faintly in a floor, the world in one picture. Writes PNG-compressed
# entries at 256, 64, 48, 32 and 16 px. Uses only System.Drawing (in Windows PowerShell 5.1).
#   powershell -ExecutionPolicy Bypass -File world\tools\make_icon.ps1
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$out = Join-Path $PSScriptRoot '..\assets\dreamride.ico'
New-Item -ItemType Directory -Force (Split-Path $out) | Out-Null

function Draw-Icon([int]$n) {
  $bmp = New-Object System.Drawing.Bitmap $n, $n, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.SmoothingMode = 'AntiAlias'; $g.InterpolationMode = 'HighQualityBicubic'; $g.PixelOffsetMode = 'HighQuality'
  $g.Clear([System.Drawing.Color]::Transparent)
  $s = $n / 256.0

  # The void: a dark rounded tile, a little lighter at the horizon.
  $r = [single](56 * $s); $m = [single](8 * $s); $w = [single]($n - 2 * $m)
  $path = New-Object System.Drawing.Drawing2D.GraphicsPath
  $path.AddArc($m, $m, $r, $r, 180, 90); $path.AddArc($m + $w - $r, $m, $r, $r, 270, 90)
  $path.AddArc($m + $w - $r, $m + $w - $r, $r, $r, 0, 90); $path.AddArc($m, $m + $w - $r, $r, $r, 90, 90)
  $path.CloseFigure()
  $bg = New-Object System.Drawing.Drawing2D.LinearGradientBrush ([System.Drawing.PointF]::new(0, $m)), ([System.Drawing.PointF]::new(0, $m + $w)), ([System.Drawing.Color]::FromArgb(255, 9, 11, 20)), ([System.Drawing.Color]::FromArgb(255, 18, 20, 36))
  $g.FillPath($bg, $path)
  $g.SetClip($path)

  # The floor line.
  $hy = [single](178 * $s)
  $g.DrawLine((New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(40, 140, 170, 255)), ([single][Math]::Max(1, 1.5 * $s))), $m, $hy, $m + $w, $hy)

  # The line: a rising, swinging curve, violet in the past to coral at the present.
  $pts = @()
  for ($i = 0; $i -le 40; $i++) {
    $u = $i / 40.0
    $x = 34 + 158 * $u
    $y = 150 - 70 * $u - 22 * [Math]::Sin($u * 3.0 * [Math]::PI) * (1 - 0.4 * $u)
    $pts += [System.Drawing.PointF]::new([single]($x * $s), [single]($y * $s))
  }
  $head = $pts[$pts.Count - 1]
  # A wide faint stroke, a narrower brighter one, then the line itself: the glow.
  foreach ($layer in @(@(26, 22), @(14, 50), @(7, 255))) {
    $a = $layer[1]
    $grad = New-Object System.Drawing.Drawing2D.LinearGradientBrush ([System.Drawing.PointF]::new([single](30 * $s), 0)), ([System.Drawing.PointF]::new([single](196 * $s), 0)), ([System.Drawing.Color]::FromArgb($a, 120, 70, 255)), ([System.Drawing.Color]::FromArgb($a, 255, 110, 120))
    $pen = New-Object System.Drawing.Pen $grad, ([single]([Math]::Max(1.2, $layer[0] * $s)))
    $pen.StartCap = 'Round'; $pen.EndCap = 'Round'; $pen.LineJoin = 'Round'
    $g.DrawCurve($pen, $pts)
  }
  # Its reflection in the floor, faint.
  $refl = @(); foreach ($p in $pts) { $refl += [System.Drawing.PointF]::new($p.X, [single](2 * $hy - $p.Y)) }
  $rpen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(55, 200, 110, 200)), ([single]([Math]::Max(1, 5 * $s)))
  $g.DrawCurve($rpen, $refl)

  # The draw-head: a bright bloom at the present.
  foreach ($k in @(@(46, 40), @(28, 90), @(14, 255))) {
    $d = [single]($k[0] * $s)
    $brush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb($k[1], 255, 196, 170))
    $g.FillEllipse($brush, $head.X - $d / 2, $head.Y - $d / 2, $d, $d)
  }
  $g.Dispose()
  return $bmp
}

$sizes = @(256, 64, 48, 32, 16)
$big = Draw-Icon 1024
$pngs = @()
foreach ($n in $sizes) {
  $b = New-Object System.Drawing.Bitmap $n, $n, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
  $g = [System.Drawing.Graphics]::FromImage($b)
  $g.InterpolationMode = 'HighQualityBicubic'; $g.PixelOffsetMode = 'HighQuality'; $g.SmoothingMode = 'AntiAlias'
  $g.DrawImage($big, 0, 0, $n, $n); $g.Dispose()
  $ms = New-Object System.IO.MemoryStream
  $b.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
  $pngs += , $ms.ToArray()
  $b.Dispose()
}

# ICO: a 6-byte header, a 16-byte entry per image, then the PNG data.
$fs = [System.IO.File]::Create($out)
$bw = New-Object System.IO.BinaryWriter $fs
$bw.Write([uint16]0); $bw.Write([uint16]1); $bw.Write([uint16]$sizes.Count)
$offset = 6 + 16 * $sizes.Count
for ($i = 0; $i -lt $sizes.Count; $i++) {
  $n = $sizes[$i]; $len = $pngs[$i].Length
  $bw.Write([byte]($(if ($n -ge 256) { 0 } else { $n }))); $bw.Write([byte]($(if ($n -ge 256) { 0 } else { $n })))
  $bw.Write([byte]0); $bw.Write([byte]0); $bw.Write([uint16]1); $bw.Write([uint16]32)
  $bw.Write([uint32]$len); $bw.Write([uint32]$offset)
  $offset += $len
}
foreach ($p in $pngs) { $bw.Write($p) }
$bw.Close()
Write-Output "icon: $out ($((Get-Item $out).Length) bytes)"
