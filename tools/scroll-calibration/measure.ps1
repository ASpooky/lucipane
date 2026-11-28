# Measure pixels per wheel notch in one window (see README.md).
# Usage: measure.ps1 -Title <part> -Inset top,right,bottom,left -Dir down|up -Series 1,3,5
param([string]$Title, [string]$Inset = "130,30,10,10", [string]$Dir = "down", [string]$Series = "1,1,1,3,5")
$ins = [int[]]($Inset -split ","); $counts = [int[]]($Series -split ",")
Add-Type -Path "$PSScriptRoot\Scroll.cs" -ReferencedAssemblies System.Drawing
$h = [Scroll]::FindWindow($Title)
if ($h -eq [IntPtr]::Zero) { throw "no window titled *$Title*" }
$r = New-Object Scroll+RECT; [void][Scroll]::GetWindowRect($h, [ref]$r)
$region = New-Object System.Drawing.Rectangle ($r.Left + $ins[3]), ($r.Top + $ins[0]), ($r.Right - $r.Left - $ins[1] - $ins[3]), ($r.Bottom - $r.Top - $ins[0] - $ins[2])
$cx = $region.Left + [int]($region.Width / 2); $cy = $region.Top + [int]($region.Height / 2)
"window '$([Scroll]::Title($h))' rect=($($r.Left),$($r.Top))-($($r.Right),$($r.Bottom)) region=$region"
$sign = if ($Dir -eq "down") { -1 } else { 1 }
foreach ($n in $counts) {
  $score = 0.0; $settle = 0
  $px = [Scroll]::Measure($region, $cx, $cy, $sign * $n, [ref]$score, [ref]$settle)
  "{0} notch(es): {1,5} px  per notch {2,6:N1}  match={3:N2}  settled in {4} ms" -f $n, [Math]::Abs($px), ([Math]::Abs($px) / $n), $score, $settle
}
