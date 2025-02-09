# Service Configuration
$serviceName = "AWinSysHlpSrv6"
$exePath = "C:\Program Files\MicroWin\WinSysHlp\WinSysHlp.exe"
$timeoutMinutes = 10
$logPath = "C:\Program Files\MicroWin\WinSysHlp\service.log"
$displayName = "WinSysHelperService"
$description = "Runs and periodically restarts the application every $timeoutMinutes minutes"

# Function to write to the log and event log
function Write-Log {
    param([string]$Message, [System.Diagnostics.EventLogEntryType]$EntryType = "Information")
    $timestamp = Get-Date -Format 'yyyy-MM-dd HH:mm:ss'
    "$timestamp - $Message" | Out-File -FilePath $logPath -Append
    Write-EventLog -LogName 'Application' -Source $serviceName -EventId 1000 -EntryType $EntryType -Message $Message
}

# Create log directory if it doesn't exist
New-Item -ItemType Directory -Force -Path (Split-Path $logPath)

# Create event source if it doesn't exist
if (-not [System.Diagnostics.EventLog]::SourceExists($serviceName)) {
    New-EventLog -LogName Application -Source $serviceName
}

# Stop and remove existing service (with retry logic)
$existingService = Get-Service -Name $serviceName -ErrorAction SilentlyContinue
if ($existingService) {
    Write-Log "Existing service found. Stopping..."
    try {
      Stop-Service -Name $serviceName -Force -ErrorAction Stop
      Write-Log "Service stopped."
    }
    catch {
      Write-Log "Failed to stop service: $($_.Exception.Message)" -EntryType Error
    }

    Write-Log "Removing service (with retry)..."
    $retries = 5  # Number of retries
    for ($i = 0; $i -lt $retries; $i++) {
        try {
            sc delete $serviceName
            Write-Log "Service removed."
            break  # Exit the loop if successful
        } catch {
            Write-Log "Failed to remove service (attempt $($i+1)/$retries): $($_.Exception.Message)" -EntryType Error
            Start-Sleep -Seconds 2 # Wait before retrying
        }
    }
    if ($i -eq $retries) {
        Write-Log "Failed to remove service after multiple retries." -EntryType Error
        exit 1
    }
    Start-Sleep -Seconds 5  # Give Windows time to release the service name
}


# Create the service wrapper script
$scriptContent = @"
# ... (Your existing script content, slightly modified below) ...
"@

# ... (rest of your script content)

try {
    # ... (Your existing script to save the wrapper script and set permissions)

    # Create new service
    $params = @{
        Name = $serviceName
        BinaryPathName = "powershell.exe -ExecutionPolicy Bypass -NoProfile -File `"$scriptPath`""
        DisplayName = $displayName
        StartupType = "Automatic"
        Description = $description
    }

    New-Service @params

# Set service recovery options (with retry logic)
$retries = 5
for ($i = 0; $i -lt $retries; $i++) {
  try {
    Start-Process -FilePath sc -ArgumentList "$serviceName failure reset=86400 actions=restart/60000/restart/60000/restart/60000" -Wait
    Write-Log "Service recovery options set successfully."
    break
  } catch {
    Write-Log "Failed to set recovery options (attempt $($i+1)/$retries): $($_.Exception.Message)" -EntryType Error
    Start-Sleep -Seconds 2
  }
}

if ($i -eq $retries) {
  Write-Log "Failed to set service recovery options after multiple retries." -EntryType Error
  exit 1
}

    Write-Log "Service '$serviceName' created successfully."
    Write-Log "Check the log file at $logPath after starting the service."

} catch {
    Write-Log "Failed to create service: $($_.Exception.Message)" -EntryType Error
    exit 1
}