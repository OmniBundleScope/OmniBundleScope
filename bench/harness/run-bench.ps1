param(
  [Parameter(Mandatory = $true)][string]$Exe,
  [string[]]$CliArgs,
  [string]$Label = 'run'
)
# Peak-RSS sampling wrapper (benchmarks). Node cannot read a child process's RSS
# cheaply, so the shell does it: sample WorkingSet64 every 100 ms until exit.
$t0 = Get-Date
$p = Start-Process $Exe -ArgumentList $CliArgs -PassThru -NoNewWindow `
    -RedirectStandardOutput "$env:TEMP/ob-$Label-out.txt" `
    -RedirectStandardError "$env:TEMP/ob-$Label-err.txt"
$peak = 0
while (-not $p.HasExited) {
  Start-Sleep -Milliseconds 100
  $p.Refresh()
  if ($p.WorkingSet64 -gt $peak) { $peak = $p.WorkingSet64 }
}
$p.WaitForExit()
$wall = [int]((Get-Date) - $t0).TotalMilliseconds
[pscustomobject]@{
  label   = $Label
  exit    = $p.ExitCode
  wallMs  = $wall
  peakMB  = [int]($peak / 1MB)
} | ConvertTo-Json -Compress
Get-Content "$env:TEMP/ob-$Label-err.txt" -Tail 2 -ErrorAction SilentlyContinue
