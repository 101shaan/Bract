# Bract Compiler Benchmark Suite
# Tests all complex programs and measures compilation performance

Write-Host "Bract Compiler Benchmark Suite" -ForegroundColor Green
Write-Host "===============================================" -ForegroundColor Green

$tests = @(
    @{name="Simple Test"; file="examples/simple_test.bract"; output="simple_test"},
    @{name="Fibonacci Recursive"; file="examples/fibonacci_recursive.bract"; output="fibonacci_recursive"}, 
    @{name="Matrix Operations"; file="examples/matrix_operations.bract"; output="matrix_operations"},
    @{name="Deep Recursion"; file="examples/deep_recursion.bract"; output="deep_recursion"},
    @{name="Complex Loops"; file="examples/complex_loops.bract"; output="complex_loops"},
    @{name="Mixed Features"; file="examples/mixed_features.bract"; output="mixed_features"}
)

$totalTime = 0
$successful = 0
$failed = 0

foreach ($test in $tests) {
    Write-Host "`nTesting: $($test.name)" -ForegroundColor Cyan
    Write-Host "   File: $($test.file)"
    
    $startTime = Get-Date
    
    try {
        # Compile with stats
        $result = & cargo run --bin bract_cranelift -- $test.file "$($test.output).exe" --stats 2>&1
        
        $endTime = Get-Date
        $elapsed = ($endTime - $startTime).TotalMilliseconds
        $totalTime += $elapsed
        
        if ($LASTEXITCODE -eq 0) {
            Write-Host "   SUCCESS in $([math]::Round($elapsed, 2))ms" -ForegroundColor Green
            $successful++
            
            # Try to run the executable
            if (Test-Path "$($test.output).exe") {
                try {
                    $execResult = & ".\$($test.output).exe"
                    $execCode = $LASTEXITCODE
                    Write-Host "   Execution result: $execCode" -ForegroundColor Yellow
                } catch {
                    Write-Host "   Execution failed: $($_.Exception.Message)" -ForegroundColor Yellow
                }
            }
        } else {
            Write-Host "   FAILED in $([math]::Round($elapsed, 2))ms" -ForegroundColor Red
            Write-Host "   Error: $result" -ForegroundColor Red
            $failed++
        }
    } catch {
        $endTime = Get-Date
        $elapsed = ($endTime - $startTime).TotalMilliseconds
                    Write-Host "   CRASHED in $([math]::Round($elapsed, 2))ms" -ForegroundColor Red
        Write-Host "   Exception: $($_.Exception.Message)" -ForegroundColor Red
        $failed++
    }
}

Write-Host "`nBENCHMARK RESULTS" -ForegroundColor Green
Write-Host "===============================================" -ForegroundColor Green
Write-Host "Successful: $successful" -ForegroundColor Green
Write-Host "Failed: $failed" -ForegroundColor Red
Write-Host "Total time: $([math]::Round($totalTime, 2))ms" -ForegroundColor Cyan
Write-Host "Average per test: $([math]::Round($totalTime / $tests.Count, 2))ms" -ForegroundColor Cyan

if ($successful -eq $tests.Count) {
    Write-Host "`nALL TESTS PASSED! Compiler is ROCK SOLID!" -ForegroundColor Green
} else {
    Write-Host "`nSome tests failed. Check errors above." -ForegroundColor Yellow
}