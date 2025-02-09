$exePath = "C:\Program Files\MicroWin\WinSysHlp\WinSysHlp.exe"  # Make sure this is correct
try {
  Start-Process -FilePath $exePath -Wait
  Write-Host "WinSysHlp.exe started successfully (outside of service)."
} catch {
  Write-Host "Error starting WinSysHlp.exe: $($_.Exception.Message)"
}