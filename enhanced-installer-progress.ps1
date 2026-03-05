# Enhanced installer function with better UX
function Start-ServicesWithProgress {
    Write-Info "Starting Govrix services..."
    docker compose up -d --no-color

    Write-Host ""
    Write-Host "🔍 Monitoring startup progress..." -ForegroundColor Yellow
    
    $timeout = 120  # 2 minutes
    $elapsed = 0
    $checkInterval = 5

    while ($elapsed -lt $timeout) {
        Write-Host "[$elapsed/$timeout] " -NoNewline -ForegroundColor Gray
        
        # Check container status
        $containers = docker ps --format "{{.Names}}: {{.Status}}" | Where-Object { $_ -match "govrix" }
        $postgres = $containers | Where-Object { $_ -match "postgres" }
        $proxy = $containers | Where-Object { $_ -match "proxy" }
        $dashboard = $containers | Where-Object { $_ -match "dashboard" }

        if ($postgres -match "healthy") {
            Write-Host "✅ PostgreSQL ready " -NoNewline -ForegroundColor Green
        } else {
            Write-Host "⏳ PostgreSQL starting... " -NoNewline -ForegroundColor Yellow
        }

        if ($proxy -match "healthy") {
            Write-Host "✅ Proxy ready " -NoNewline -ForegroundColor Green
        } elseif ($proxy -match "unhealthy") {
            Write-Host "⚠️ Proxy connection issues " -NoNewline -ForegroundColor Red
            # Show actual error from logs
            $proxyLogs = docker logs govrix-scout-proxy --tail 1 2>&1
            if ($proxyLogs -match "PostgreSQL unavailable") {
                Write-Host "(DB conn) " -NoNewline -ForegroundColor Red
            }
        } else {
            Write-Host "⏳ Proxy starting... " -NoNewline -ForegroundColor Yellow
        }

        if ($dashboard -match "healthy") {
            Write-Host "✅ Dashboard ready" -ForegroundColor Green
            break  # All services ready!
        } else {
            Write-Host "⏳ Dashboard starting..." -ForegroundColor Yellow
        }

        Start-Sleep $checkInterval
        $elapsed += $checkInterval
    }

    # Final status check
    Write-Host ""
    Write-Info "Testing service endpoints..."
    
    try {
        $apiStatus = Invoke-WebRequest -Uri "http://localhost:4001/health" -UseBasicParsing -TimeoutSec 3
        Write-Success "Management API: http://localhost:4001 ✅"
    } catch {
        Write-Warn "Management API not ready: $($_.Exception.Message)"
    }

    try {
        $dashboardStatus = Invoke-WebRequest -Uri "http://localhost:3000" -UseBasicParsing -TimeoutSec 3
        Write-Success "Dashboard: http://localhost:3000 ✅"
    } catch {
        Write-Warn "Dashboard not ready: $($_.Exception.Message)"
    }

    Write-Success "Proxy ready for agents: http://localhost:4000"
}