<#
Runs a command, and reports wall time plus the child's own peak working set.

Why PowerShell and not Node: Windows exposes no cheap per-child RSS to Node, and
the memory numbers in docs/en/01-evidence.md are claims, so they have to come
from the OS rather than from a parent-process approximation. A Process object
retains PeakWorkingSet64 after exit, which is the high-water mark the memory
targets in docs/contracts/bench-spec.md §6 need.

  powershell -NoProfile -File measure.ps1 -Binary <exe> -Target <input> `
      -Extra '--mode static --report out.html' -Runs 3

Targets Windows PowerShell 5.1: `ArgumentList` is .NET Core only, so arguments
are passed as one quoted string (as the native API has always required).
#>
param(
  [Parameter(Mandatory = $true)][string]$Binary,
  [Parameter(Mandatory = $true)][string]$Target,
  [string]$Extra = '',
  [int]$Runs = 1
)

# Native quoting: wrap anything with whitespace, and escape the two characters
# the CRT treats specially inside a quoted argument.
function Quote-Native([string]$a) {
  if ($a -notmatch '[\s"]') { return $a }
  return '"' + ($a -replace '(\\*)"', '$1$1\"' -replace '(\\+)$', '$1$1') + '"'
}

$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $Binary
$psi.Arguments = ((Quote-Native $Target) + ' ' + $Extra).Trim()
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError = $true
$psi.UseShellExecute = $false

$walls = @()
$peaks = @()
$privates = @()
$lastOut = ''
$lastErr = ''

for ($i = 0; $i -lt $Runs; $i++) {
  $proc = [System.Diagnostics.Process]::Start($psi)
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  $stdout = $proc.StandardOutput.ReadToEndAsync()
  $stderr = $proc.StandardError.ReadToEndAsync()

  # Sample the child's memory while it runs. `PeakWorkingSet64` reads 0 after the
  # process exits under Windows PowerShell 5.1, so the high-water mark is not
  # available here; these are the max of the samples and are reported as such.
  #
  # Two numbers, because they answer different questions and only quoting one is
  # how a benchmark becomes a lie:
  #   * working set = resident pages, *including* file-backed pages the OS keeps
  #     cached. For a tool that streams a 1 GB stats file that inflates with the
  #     page cache and moved between 411 MB and 1008 MB for the same run.
  #   * private bytes = heap + stacks the process actually asked for. This is what
  #     the 200/400/500 MB targets are about, and it is what the Rust/WBA/SME
  #     numbers in docs/en/01-evidence.md are comparable to.
  $peakWs = 0
  $peakPrivate = 0
  while (-not $proc.HasExited) {
    try {
      $proc.Refresh()
      $ws = [math]::Round($proc.WorkingSet64 / 1MB, 1)
      if ($ws -gt $peakWs) { $peakWs = $ws }
      $priv = [math]::Round($proc.PrivateMemorySize64 / 1MB, 1)
      if ($priv -gt $peakPrivate) { $peakPrivate = $priv }
    } catch { }
    Start-Sleep -Milliseconds 25
  }
  $sw.Stop()
  $lastOut = $stdout.Result
  $lastErr = $stderr.Result
  $walls += $sw.ElapsedMilliseconds
  $peaks += $peakWs
  $privates += $peakPrivate
  $proc.Dispose()
}

$mid = [int][math]::Floor($Runs / 2)
$sortedW = @($walls | Sort-Object)
$sortedP = @($peaks | Sort-Object)
$sortedPriv = @($privates | Sort-Object)

$out = [ordered]@{
  binary             = $Binary
  target             = $Target
  args               = $psi.Arguments
  runs               = $Runs
  wall_ms            = @($walls)
  wall_ms_median     = $sortedW[$mid]
  peak_rss_mb        = @($peaks)
  peak_rss_median_mb = $sortedP[$mid]
  peak_private_mb        = @($privates)
  peak_private_median_mb = $sortedPriv[$mid]
  rss_source         = 'sampled max every 25 ms; working set includes file-backed pages, private bytes do not. Not a kernel high-water mark.'
  stdout_tail        = (($lastOut -split "`r?`n" | Where-Object { $_.Trim() } | Select-Object -First 3) -join ' | ')
  stderr_tail        = (($lastErr -split "`r?`n" | Where-Object { $_.Trim() } | Select-Object -First 3) -join ' | ')
}
$out | ConvertTo-Json -Depth 4 -Compress
