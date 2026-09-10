$ErrorActionPreference = 'Continue'
Write-Output '--- Get-NetAdapter ---'
Get-NetAdapter -Name 'LightTunnelAdapter' | Format-List Name, InterfaceDescription, ifIndex, Status
Write-Output '--- Get-NetAdapterStatistics ---'
Get-NetAdapterStatistics -Name 'LightTunnelAdapter' | Format-List Name, ReceivedBytes, SentBytes
Write-Output '--- Get-NetAdapterStatistics all ---'
Get-NetAdapterStatistics | Select-Object Name, ReceivedBytes, SentBytes | Format-Table -AutoSize
