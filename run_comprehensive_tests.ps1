#!/usr/bin/env pwsh
# BRACT COMPREHENSIVE TEST EXECUTION PROTOCOL
# Systematic validation of all native compilation capabilities

Write-Host "🔥 BRACT NATIVE COMPILATION TEST SUITE 🔥" -ForegroundColor Red
Write-Host "=========================================" -ForegroundColor Red
Write-Host ""

$tests = @(
    @{name="Simple Expression Return"; file="test_01_simple_expression.bract"; expected=42},
    @{name="Explicit Return"; file="test_02_explicit_return.bract"; expected=99},
    @{name="Unit Return"; file="test_03_unit_return.bract"; expected=$null},
    @{name="Arithmetic Operations"; file="test_04_arithmetic.bract"; expected=25},  # 5 + 10 * 2 = 25
    @{name="Negative Numbers"; file="test_05_negative_numbers.bract"; expected=58},  # -42 + 100 = 58
    @{name="Zero Result"; file="test_06_zero_result.bract"; expected=0},
    @{name="Complex Arithmetic"; file="test_07_complex_arithmetic.bract"; expected=38}  # (10+5)*3-7 = 38
)

$passed = 0
$failed = 0

foreach ($test in $tests) {
    Write-Host "🧪 Testing: $($test.name)" -ForegroundColor Cyan
    Write-Host "   File: $($test.file)"
    
    # Compile the test
    $compileResult = & "./target/release/bract_cranelift.exe" $test.file 2>&1
    
    if ($LASTEXITCODE -ne 0) {
        Write-Host "   ❌ COMPILATION FAILED" -ForegroundColor Red
        Write-Host "   Error: $compileResult"
        $failed++
        continue
    }
    
    Write-Host "   ✅ Compilation successful" -ForegroundColor Green
    
    # Execute the test if it has an expected result
    if ($test.expected -ne $null) {
        $exeName = $test.file -replace "\.bract", ".exe"
        $null = & "./$exeName" 2>&1
        $actualResult = $LASTEXITCODE
        
        if ($actualResult -eq $test.expected) {
            Write-Host "   ✅ Execution successful - Expected: $($test.expected), Got: $actualResult" -ForegroundColor Green
            $passed++
        } else {
            Write-Host "   ❌ EXECUTION FAILED - Expected: $($test.expected), Got: $actualResult" -ForegroundColor Red
            $failed++
        }
    } else {
        Write-Host "   ✅ Unit return test (no expected result)" -ForegroundColor Green
        $passed++
    }
    
    Write-Host ""
}

Write-Host "🏆 TEST SUMMARY" -ForegroundColor Yellow
Write-Host "===============" -ForegroundColor Yellow
Write-Host "✅ Passed: $passed" -ForegroundColor Green
Write-Host "❌ Failed: $failed" -ForegroundColor Red

if ($failed -eq 0) {
    Write-Host ""
    Write-Host "🎉🎉🎉 ALL TESTS PASSED! BRACT NATIVE COMPILATION IS ROCK SOLID! 🎉🎉🎉" -ForegroundColor Green
    exit 0
} else {
    Write-Host ""
    Write-Host "💥 SOME TESTS FAILED! INVESTIGATE AND FIX!" -ForegroundColor Red
    exit 1
} 